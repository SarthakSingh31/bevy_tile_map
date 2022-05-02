use std::num::NonZeroU32;

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};

use super::TileMapPipeline;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "fd3a76be-60a3-4b67-a2da-8c987f65ae16"]
pub struct TileSheet {
    tile_sets: Vec<Handle<Image>>,
    tile_size: UVec2,
    tile_data: Vec<(Vec<u8>, Extent3d, TextureFormat)>,
    gpu_image: Option<GpuImage>,
}

impl TileSheet {
    pub fn new(mut tile_sets: Vec<Handle<Image>>, tile_size: UVec2) -> Self {
        tile_sets.sort();
        tile_sets.dedup();

        TileSheet {
            tile_sets,
            tile_size,
            tile_data: Vec::new(),
            gpu_image: None,
        }
    }

    pub fn load_images(&mut self, images: &Assets<Image>) {
        for (idx, image_handle) in self.tile_sets.iter().enumerate() {
            if let Some(image) = images.get(image_handle) {
                if let Some((data, layers, format)) = self.tile_data.get_mut(idx) {
                    data.clear();
                    data.extend(&image.data);
                    *layers = image.texture_descriptor.size;
                    *format = image.texture_descriptor.format;
                } else {
                    self.tile_data.push((
                        image.data.clone(),
                        image.texture_descriptor.size,
                        image.texture_descriptor.format,
                    ));
                }
            }
        }
    }
}

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
        mut tile_sheet: Self::ExtractedAsset,
        (render_device, render_queue, tile_map_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let fixed_idx = 0; // TODO: very temporary. Only loading the first image

        let (data, size, format) = if let Some(tile_data) = &tile_sheet.tile_data.get(fixed_idx) {
            tile_data
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(tile_sheet));
        };
        let array_count = (size.width * size.height * size.depth_or_array_layers)
            / (tile_sheet.tile_size.x * tile_sheet.tile_size.y);

        let texture = render_device.create_texture_with_data(
            render_queue,
            &TextureDescriptor {
                label: Some("TileSheet::Texture"),
                size: Extent3d {
                    width: tile_sheet.tile_size.x,
                    height: tile_sheet.tile_size.y,
                    depth_or_array_layers: array_count,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: *format,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            },
            data.as_slice(),
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
            format: Some(*format),
            dimension: Some(TextureViewDimension::D2Array),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: NonZeroU32::new(array_count),
        });

        tile_sheet.gpu_image = Some(GpuImage {
            texture_format: *format,
            texture,
            sampler,
            texture_view,
            size: Size::new(tile_sheet.tile_size.x as f32, tile_sheet.tile_size.y as f32),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &tile_sheet.gpu_image.as_ref().unwrap().texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(
                        &tile_sheet.gpu_image.as_ref().unwrap().sampler,
                    ),
                },
            ],
            label: Some("TileMap::TileSheet::BindGroup"),
            layout: &tile_map_pipeline.texture_sampler_layout,
        });

        Ok(GpuTileSheet { bind_group })
    }
}
