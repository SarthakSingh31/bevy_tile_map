mod chunk;
mod interaction;
mod render;
mod tile_map;

use bevy::{
    core_pipeline::Transparent2d,
    prelude::*,
    render::{
        render_asset::{PrepareAssetLabel, RenderAssetPlugin},
        render_phase::AddRenderCommand,
        render_resource::SpecializedRenderPipelines,
        RenderApp, RenderStage,
    },
};

use bevy_mod_raycast::RaycastSystem;

pub use interaction::{TileMapInteractionEvent, TileMapRayCastSource};
pub use render::TileSheet;
pub use tile_map::*;

pub struct TileMapPlugin;

impl Plugin for TileMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<render::ChunkShader>()
            .add_plugin(RenderAssetPlugin::<TileSheet>::with_prepare_asset_label(
                PrepareAssetLabel::PreAssetPrepare,
            ))
            .add_asset::<TileSheet>()
            .add_event::<TileMapInteractionEvent>()
            .add_plugin(interaction::TileMapRayCastPlugin::default())
            .add_system_to_stage(
                CoreStage::PreUpdate,
                interaction::update_camera_ray.before(RaycastSystem::BuildRays),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                interaction::queue_interaction_events.after(RaycastSystem::UpdateRaycast),
            )
            .add_system(chunk::generate_or_update_chunks);

        let shader = app
            .world
            .get_resource::<render::ChunkShader>()
            .unwrap()
            .clone();
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(shader)
                .init_resource::<render::TileMapPipeline>()
                .init_resource::<SpecializedRenderPipelines<render::TileMapPipeline>>()
                .init_resource::<render::TileMapMeta>()
                .init_resource::<render::ExtractedChunks>()
                .init_resource::<render::TileUniforms>()
                .add_render_command::<Transparent2d, render::DrawChunk>()
                .add_system_to_stage(RenderStage::Extract, render::extract_chunks)
                .add_system_to_stage(RenderStage::Prepare, render::prepare_tiles)
                .add_system_to_stage(RenderStage::Queue, render::queue_chunks);
        };
    }

    fn name(&self) -> &str {
        "Tilemap Plugin"
    }
}
