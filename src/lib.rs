mod chunk;
mod render;
mod tile_map;

use bevy::{
    core_pipeline::Transparent2d,
    prelude::*,
    render::{
        render_phase::AddRenderCommand, render_resource::SpecializedRenderPipelines, RenderApp,
        RenderStage,
    },
};

pub use chunk::ChunkSize;
pub use tile_map::*;

pub struct TileMapPlugin;

impl Plugin for TileMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TileSheet>()
            .init_resource::<ChunkSize>()
            .init_resource::<render::ChunkShader>()
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
                .init_resource::<render::ChunkMeta>()
                .init_resource::<render::ExtractedChunks>()
                .add_render_command::<Transparent2d, render::DrawChunk>()
                .add_system_to_stage(RenderStage::Extract, render::extract_chunks)
                .add_system_to_stage(RenderStage::Queue, render::queue_chunks);
        };
    }

    fn name(&self) -> &str {
        "Tilemap Plugin"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
