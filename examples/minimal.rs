use bevy::{diagnostic, prelude::*};
use bevy_tilemap::{Tile, TileMap, TileMapBundle, TileMapPlugin, TileSheet};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(diagnostic::DiagnosticsPlugin)
        .add_plugin(diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugin(diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(TileMapPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut tile_sheets: ResMut<Assets<TileSheet>>,
) {
    let mut tile_map = TileMap::new(UVec2::new(300, 300), UVec2::new(8, 8), UVec2::new(16, 16));

    let tile_sheet = tile_sheets.add(TileSheet {
        tile_sheet: asset_server.load("0x72_16x16DungeonTileset.v4.png"),
        tile_size: Vec2::new(16.0, 16.0),
    });

    let tile_sheet = tile_map.add_tile_sheet(tile_sheet);

    for x in 0..300 {
        for y in 0..300 {
            tile_map[(x, y, 0)] = Some(Tile {
                tile_sheet,
                tile_idx: 0,
            });
        }
    }

    commands.spawn_bundle(TileMapBundle {
        tile_map,
        ..Default::default()
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}
