mod tile_sheet;

use std::cmp::Ordering;

use bevy::{
    core::FloatOrd,
    core_pipeline::Transparent2d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::*,
        render_resource::{std140::AsStd140, *},
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderWorld,
    },
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

use crate::{chunk::ChunkData, Tile};

pub use tile_sheet::TileSheet;

#[derive(Clone)]
pub struct ChunkShader(Handle<Shader>);

impl FromWorld for ChunkShader {
    fn from_world(world: &mut World) -> Self {
        let mut shaders = world
            .get_resource_mut::<Assets<Shader>>()
            .expect("Couldn't get the shader assets to load the chunk shader into it");
        ChunkShader(shaders.add(Shader::from_wgsl(include_str!("chunk.wgsl"))))
    }
}

pub struct TileMapPipeline {
    view_layout: BindGroupLayout,
    tiles_layout: BindGroupLayout,
    texture_sampler_layout: BindGroupLayout,
    chunk_shader: Handle<Shader>,
}

impl FromWorld for TileMapPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
                },
                count: None,
            }],
            label: Some("TileMap::View::Layout"),
        });

        let tiles_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(i32::std140_size_static() as u64),
                },
                count: None,
            }],
            label: Some("TileMap::Tiles::Layout"),
        });

        let texture_sampler_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: Some("TileMap::Texture::Sampler::Layout"),
            });

        let chunk_shader = world.resource::<ChunkShader>().0.clone();

        TileMapPipeline {
            view_layout,
            tiles_layout,
            texture_sampler_layout,
            chunk_shader,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct TileMapPipelineKey: u32 {
        const NONE                        = 0;
        const MSAA_RESERVED_BITS          = TileMapPipelineKey::MSAA_MASK_BITS << TileMapPipelineKey::MSAA_SHIFT_BITS;
    }
}

impl TileMapPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        TileMapPipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
}

impl SpecializedRenderPipeline for TileMapPipeline {
    type Key = TileMapPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let instance_layout = ChunkInstance::vertex_buffer_layout();

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.chunk_shader.as_weak(),
                entry_point: "vertex".into(),
                shader_defs: Vec::default(),
                buffers: vec![instance_layout],
            },
            fragment: Some(FragmentState {
                shader: self.chunk_shader.as_weak(),
                shader_defs: Vec::default(),
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![
                self.view_layout.clone(),
                self.tiles_layout.clone(),
                self.texture_sampler_layout.clone(),
            ]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("TileMap::Pipeline".into()),
        }
    }
}

pub struct ExtractedChunk {
    index: usize,
    data: Vec<Option<Tile>>,
    chunk_size: UVec2,
    tile_size: UVec2,
    tile_sheet_handle: Handle<TileSheet>,
    transform: GlobalTransform,
}

#[derive(Default)]
pub struct ExtractedChunks {
    chunks: Vec<ExtractedChunk>,
}

pub fn extract_chunks(
    images: Res<Assets<Image>>,
    mut render_world: ResMut<RenderWorld>,
    mut tile_sheets: ResMut<Assets<TileSheet>>,
    chunks: Query<(&ComputedVisibility, &ChunkData, &GlobalTransform)>,
) {
    let mut extracted_chunks = render_world.resource_mut::<ExtractedChunks>();
    extracted_chunks.chunks.clear();

    for (index, (visibility, chunk_data, transform)) in chunks.iter().enumerate() {
        if !visibility.is_visible {
            continue;
        }

        if let Some(tile_sheet) = tile_sheets.get_mut(chunk_data.tile_sheet()) {
            tile_sheet.load_images(&images);
        }

        extracted_chunks.chunks.push(ExtractedChunk {
            index,
            data: chunk_data.tiles().clone(),
            chunk_size: chunk_data.chunk_size(),
            tile_size: chunk_data.tile_size(),
            tile_sheet_handle: chunk_data.tile_sheet().as_weak(),
            transform: transform.clone(),
        });
    }
}

#[derive(Default)]
pub struct TileUniform(HashMap<usize, StorageBuffer<i32>>);

#[derive(Component)]
pub struct TilesBindGroup(BindGroup);

pub fn prepare_tiles(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut extracted_chunks: ResMut<ExtractedChunks>,
    mut tile_uniforms: ResMut<TileUniform>,
) {
    extracted_chunks.chunks.sort_by(|a, b| {
        match a
            .transform
            .translation
            .z
            .partial_cmp(&b.transform.translation.z)
        {
            Some(Ordering::Equal) | None => a.index.cmp(&b.index),
            Some(other) => other,
        }
    });

    for (_, buffer) in &mut tile_uniforms.0 {
        buffer.clear()
    }

    for chunk in &mut extracted_chunks.chunks {
        let buffer = if let Some(buffer) = tile_uniforms.0.get_mut(&chunk.index) {
            buffer
        } else {
            tile_uniforms
                .0
                .insert(chunk.index, StorageBuffer::default());
            tile_uniforms.0.get_mut(&chunk.index).unwrap()
        };
        for tile in &chunk.data {
            if let Some(tile) = tile {
                // TODO: This requries a lot of work to support multiple tilesheets
                // chunk.tile_sheet_handle = Some(Handle::weak(tile.tile_sheet.id()));

                buffer.push(tile.idx as i32);
            } else {
                buffer.push(-1);
            }
        }
        buffer.write_buffer(&render_device, &render_queue);
    }
}

pub struct TileMapMeta {
    view_bind_group: Option<BindGroup>,
    // Chunk size to index vec
    index_buffers: HashMap<UVec2, BufferVec<u16>>,
}

impl Default for TileMapMeta {
    fn default() -> Self {
        Self {
            view_bind_group: None,
            index_buffers: HashMap::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ChunkInstance {
    transform: Mat4,
    chunk_size: UVec2,
    tile_size: UVec2,
}

impl ChunkInstance {
    fn vertex_buffer_layout() -> VertexBufferLayout {
        VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Instance,
            [
                // transform
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                // chunk_size
                VertexFormat::Uint32x2,
                // tile_size
                VertexFormat::Uint32x2,
            ],
        )
    }
}

#[derive(Component)]
pub struct ChunkInstanceBuffer(BufferVec<ChunkInstance>);

#[derive(Component)]
pub struct ChunkInstanceData {
    chunk_size: UVec2,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_chunks(
    mut commands: Commands,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut chunk_meta: ResMut<TileMapMeta>,
    view_uniforms: Res<ViewUniforms>,
    tile_map_pipeline: Res<TileMapPipeline>,
    msaa: Res<Msaa>,
    extracted_chunks: Res<ExtractedChunks>,
    tile_uniforms: Res<TileUniform>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TileMapPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let chunk_meta = &mut chunk_meta;

        chunk_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("TileMap::ViewBindGroup"),
            layout: &tile_map_pipeline.view_layout,
        }));

        let draw_chunk_function = draw_functions.read().get_id::<DrawChunk>().unwrap();
        let key = TileMapPipelineKey::from_msaa_samples(msaa.samples);
        let pipeline = pipelines.specialize(&mut pipeline_cache, &tile_map_pipeline, key);

        for mut transparent_phase in views.iter_mut() {
            let extracted_chunks = &extracted_chunks.chunks;
            transparent_phase.items.reserve(extracted_chunks.len());

            for chunk in extracted_chunks.iter() {
                // Init index buffer if its not already ready
                if chunk_meta.index_buffers.get(&chunk.chunk_size).is_none() {
                    let mut buffer = BufferVec::new(BufferUsages::INDEX);

                    const INDICES: [u16; 6] = [0, 3, 1, 0, 2, 3];
                    for tile_idx in 0..chunk.chunk_size.x * chunk.chunk_size.y {
                        for index in INDICES {
                            buffer.push(index + (4 * tile_idx) as u16);
                        }
                    }

                    buffer.write_buffer(&render_device, &render_queue);
                    chunk_meta.index_buffers.insert(chunk.chunk_size, buffer);
                }

                let tiles_bind_group = if let Some(Some(tiles_binding)) = tile_uniforms
                    .0
                    .get(&chunk.index)
                    .map(|buffer| buffer.binding())
                {
                    render_device.create_bind_group(&BindGroupDescriptor {
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: tiles_binding,
                        }],
                        label: Some("TileMap::TilesBindGroup"),
                        layout: &tile_map_pipeline.tiles_layout,
                    })
                } else {
                    continue;
                };

                let mut instance_buffer = BufferVec::new(BufferUsages::VERTEX);
                instance_buffer.push(ChunkInstance {
                    transform: chunk.transform.compute_matrix(),
                    chunk_size: chunk.chunk_size,
                    tile_size: chunk.tile_size,
                });
                instance_buffer.write_buffer(&render_device, &render_queue);

                let entity = commands
                    .spawn_bundle((
                        ChunkInstanceData {
                            chunk_size: chunk.chunk_size,
                        },
                        ChunkInstanceBuffer(instance_buffer),
                        TilesBindGroup(tiles_bind_group),
                        chunk.tile_sheet_handle.as_weak::<TileSheet>(),
                    ))
                    .id();
                let sort_key = FloatOrd(chunk.transform.translation.z);

                transparent_phase.add(Transparent2d {
                    draw_function: draw_chunk_function,
                    pipeline,
                    entity,
                    sort_key,
                    batch_range: None,
                });
            }
        }
    }
}

pub type DrawChunk = (
    SetItemPipeline,
    SetChunkViewBindGroup<0>,
    SetChunkTilesBindGroup<1>,
    SetChunkTextureBindGroup<2>,
    DrawChunkCommand,
);

pub struct SetChunkViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetChunkViewBindGroup<I> {
    type Param = (SRes<TileMapMeta>, SQuery<Read<ViewUniformOffset>>);

    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        (chunk_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            chunk_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetChunkTilesBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetChunkTilesBindGroup<I> {
    type Param = SQuery<Read<TilesBindGroup>>;

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        tiles_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let tiles_bind_group = tiles_query.get_inner(item).unwrap();
        pass.set_bind_group(I, &tiles_bind_group.0, &[]);
        RenderCommandResult::Success
    }
}

pub struct SetChunkTextureBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetChunkTextureBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TileSheet>>,
        SQuery<Read<Handle<TileSheet>>>,
    );

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (assets, handle_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let tile_sheet_handle = handle_query.get(item).unwrap();
        if let Some(tile_sheet) = assets.into_inner().get(tile_sheet_handle) {
            pass.set_bind_group(I, &tile_sheet.bind_group, &[]);

            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

pub struct DrawChunkCommand;

impl EntityRenderCommand for DrawChunkCommand {
    type Param = (
        SRes<TileMapMeta>,
        SQuery<(Read<ChunkInstanceBuffer>, Read<ChunkInstanceData>)>,
    );

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meta, chunk_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (buffer, data) = chunk_query.get_inner(item).unwrap();
        pass.set_vertex_buffer(0, buffer.0.buffer().unwrap().slice(..));

        pass.set_index_buffer(
            meta.into_inner().index_buffers[&data.chunk_size]
                .buffer()
                .unwrap()
                .slice(..),
            0,
            IndexFormat::Uint16,
        );
        pass.draw_indexed(0..(data.chunk_size.x * data.chunk_size.y * 6), 0, 0..1);

        RenderCommandResult::Success
    }
}
