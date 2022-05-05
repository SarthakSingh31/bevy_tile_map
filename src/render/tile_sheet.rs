use std::num::NonZeroU32;

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::const_uvec2,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, TextureFormatPixelInfo},
    },
    utils::HashSet,
};

use super::TileMapPipeline;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "fd3a76be-60a3-4b67-a2da-8c987f65ae16"]
pub struct TileSheet {
    tile_sets: Vec<Handle<Image>>,
    tile_size: UVec2,
    tile_data: Vec<u8>,
    array_count: u32,
    format: Option<TextureFormat>,
}

impl TileSheet {
    pub fn new(mut tile_sets: Vec<Handle<Image>>, tile_size: UVec2) -> Self {
        tile_sets.sort();
        tile_sets.dedup();

        TileSheet {
            tile_sets,
            tile_size,
            tile_data: Vec::new(),
            array_count: 0,
            format: None,
        }
    }

    pub fn empty() -> TileSheet {
        TileSheet {
            tile_sets: Vec::new(),
            tile_size: const_uvec2!([1, 1]),
            tile_data: vec![0, 0, 0, 0],
            array_count: 1,
            format: Some(TextureFormat::bevy_default()),
        }
    }

    pub fn update_images(
        &mut self,
        images: &Assets<Image>,
        updated_images: &HashSet<Handle<Image>>,
    ) {
        if self
            .tile_sets
            .iter()
            .any(|handle| updated_images.contains(handle))
        {
            let mut used_space = 0;
            let mut format = None;

            for image_handle in self.tile_sets.iter() {
                if let Some(img) = images.get(image_handle) {
                    let needed_space = img
                        .data
                        .len()
                        .checked_sub(self.tile_data.len() - used_space);
                    if let Some(needed_space) = needed_space {
                        self.tile_data.extend(vec![0; needed_space]);
                    }

                    Self::make_into_tiles(
                        &mut self.tile_data[used_space..(used_space + img.data.len())],
                        &img.data,
                        self.tile_size,
                        img.texture_descriptor.format,
                    );

                    used_space += img.data.len();
                    if let Some(format) = format {
                        assert_eq!(format, img.texture_descriptor.format);
                    } else {
                        format = Some(img.texture_descriptor.format);
                    }
                }
            }

            self.format = format;
            if let Some(format) = self.format {
                self.array_count = (used_space
                    / (self.tile_size.x as usize * self.tile_size.y as usize * format.pixel_size()))
                    as u32;
            }
        }
    }

    fn make_into_tiles(dest: &mut [u8], src: &[u8], tile_size: UVec2, format: TextureFormat) {
        let pixel_size = format.pixel_size();

        let tile_stride = tile_size.x as usize * pixel_size;
        let row_stride = tile_size.y as usize * tile_stride;

        for (idx, dest_chunk) in dest.chunks_exact_mut(tile_stride).enumerate() {
            let x = (idx / tile_size.y as usize) % tile_size.x as usize;
            let sub_tile_y = (tile_size.y - 1) as usize - (idx % tile_size.y as usize);
            let y = idx / (tile_size.y * tile_size.x) as usize;

            let src_start = (y * tile_size.y as usize * row_stride)
                + (row_stride * sub_tile_y)
                + (x * tile_stride);
            let src_end = src_start + tile_stride;

            dest_chunk.copy_from_slice(&src[src_start..src_end]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuTileSheet {
    pub bind_group: BindGroup,
}

impl RenderAsset for TileSheet {
    type ExtractedAsset = TileSheet;
    type PreparedAsset = GpuTileSheet;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>, SRes<TileMapPipeline>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        tile_sheet: Self::ExtractedAsset,
        (render_device, render_queue, tile_map_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let format = if let Some(format) = tile_sheet.format {
            format
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(tile_sheet));
        };

        let texture = render_device.create_texture_with_data(
            render_queue,
            &TextureDescriptor {
                label: Some("TileSheet::Texture"),
                size: Extent3d {
                    width: tile_sheet.tile_size.x,
                    height: tile_sheet.tile_size.y,
                    depth_or_array_layers: tile_sheet.array_count,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: format,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            },
            &tile_sheet.tile_data,
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("TileSheet::Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: std::f32::MAX,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
        });

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: Some("TileSheet::TextureView"),
            format: Some(format),
            dimension: Some(TextureViewDimension::D2Array),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: NonZeroU32::new(tile_sheet.array_count),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("TileMap::TileSheet::BindGroup"),
            layout: &tile_map_pipeline.texture_sampler_layout,
        });

        Ok(GpuTileSheet { bind_group })
    }
}
