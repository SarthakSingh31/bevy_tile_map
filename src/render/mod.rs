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
        mesh::GpuBufferInfo,
        render_asset::RenderAssets,
        render_phase::{
            DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::{std140::AsStd140, *},
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderWorld,
    },
};
use bytemuck::{Pod, Zeroable};
use copyless::VecHelper;

use crate::chunk::ChunkMesh;

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

        let chunk_shader = world.resource::<ChunkShader>().0.clone();

        TileMapPipeline {
            view_layout,
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
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            [
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
            ],
        );
        let mut instance_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Instance,
            [
                // transform
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
            ],
        );
        // FIXME: This is to work around a bevy bug: https://github.com/bevyengine/bevy/issues/4634
        instance_layout
            .attributes
            .iter_mut()
            .for_each(|attr| attr.shader_location += 2);

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.chunk_shader.as_weak(),
                entry_point: "vertex".into(),
                shader_defs: Vec::default(),
                buffers: vec![vertex_layout, instance_layout],
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
            layout: Some(vec![self.view_layout.clone()]),
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
    mesh_handle: Handle<Mesh>,
    transform: GlobalTransform,
}

#[derive(Default)]
pub struct ExtractedChunks {
    chunks: Vec<ExtractedChunk>,
}

pub fn extract_chunks(
    mut render_world: ResMut<RenderWorld>,
    chunks: Query<(&ComputedVisibility, &ChunkMesh, &GlobalTransform)>,
) {
    let mut extracted_sprites = render_world.resource_mut::<ExtractedChunks>();
    extracted_sprites.chunks.clear();

    for (visibility, chunk_mesh, transform) in chunks.iter() {
        if !visibility.is_visible {
            continue;
        }

        if let Some(mesh_handle) = &chunk_mesh.0 {
            extracted_sprites.chunks.alloc().init(ExtractedChunk {
                mesh_handle: mesh_handle.as_weak(),
                transform: transform.clone(),
            });
        }
    }
}

pub struct ChunkMeta {
    view_bind_group: Option<BindGroup>,
}

impl Default for ChunkMeta {
    fn default() -> Self {
        Self {
            view_bind_group: None,
        }
    }
}

#[derive(Component)]
pub struct Chunk {
    transform_buffer: BufferVec<Mat4>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_chunks(
    mut commands: Commands,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut chunk_meta: ResMut<ChunkMeta>,
    view_uniforms: Res<ViewUniforms>,
    tile_map_pipeline: Res<TileMapPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TileMapPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    mut extracted_chunks: ResMut<ExtractedChunks>,
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
            let extracted_chunks = &mut extracted_chunks.chunks;
            transparent_phase.items.reserve(extracted_chunks.len());

            extracted_chunks.sort_unstable_by(|a, b| {
                match a
                    .transform
                    .translation
                    .z
                    .partial_cmp(&b.transform.translation.z)
                {
                    Some(Ordering::Equal) | None => a.mesh_handle.cmp(&b.mesh_handle),
                    Some(other) => other,
                }
            });

            for extracted_chunk in extracted_chunks.iter() {
                let mut transform_buffer = BufferVec::new(BufferUsages::VERTEX);
                transform_buffer.push(extracted_chunk.transform.compute_matrix());
                transform_buffer.write_buffer(&render_device, &render_queue);

                let entity = commands
                    .spawn_bundle((
                        extracted_chunk.mesh_handle.as_weak::<Mesh>(),
                        Chunk { transform_buffer },
                    ))
                    .id();
                let sort_key = FloatOrd(extracted_chunk.transform.translation.z);

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
    // SetChunkTextureBindGroup<1>,
    DrawChunkCommand,
);

pub struct SetChunkViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetChunkViewBindGroup<I> {
    type Param = (SRes<ChunkMeta>, SQuery<Read<ViewUniformOffset>>);

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

pub struct SetChunkTextureBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetChunkTextureBindGroup<I> {
    type Param = (SRes<ChunkMeta>, SQuery<Read<ViewUniformOffset>>);

    #[inline]
    fn render<'w>(
        _view: Entity,
        _item: Entity,
        (_chunk_meta, _view_query): SystemParamItem<'w, '_, Self::Param>,
        _pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // let view_uniform = view_query.get(view).unwrap();
        // pass.set_bind_group(
        //     I,
        //     chunk_meta.into_inner().view_bind_group.as_ref().unwrap(),
        //     &[view_uniform.offset],
        // );
        RenderCommandResult::Success
    }
}

pub struct DrawChunkCommand;

impl EntityRenderCommand for DrawChunkCommand {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<Handle<Mesh>>>,
        SQuery<Read<Chunk>>,
    );

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query, chunk_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item).unwrap();
        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

            let chunk = chunk_query.get_inner(item).unwrap();
            pass.set_vertex_buffer(1, chunk.transform_buffer.buffer().unwrap().slice(..));

            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(0..*count, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed { vertex_count } => {
                    pass.draw(0..*vertex_count, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}
