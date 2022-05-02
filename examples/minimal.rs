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
        .add_system(switch_to_next_texture)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut tile_sheets: ResMut<Assets<TileSheet>>,
) {
    let tile_sheet = tile_sheets.add(TileSheet::new(
        vec![asset_server.load("0x72_16x16DungeonTileset.v4.png")],
        // UVec2::new(16, 16),
        UVec2::new(16, 16),
    ));

    let mut tile_map = TileMap::new(
        UVec2::new(300, 300),
        UVec2::new(8, 8),
        UVec2::new(16, 16),
        tile_sheet,
    );

    for x in 0..300 {
        for y in 0..300 {
            if (x + y) % 2 == 0 {
                tile_map[(x, y, 0)] = Some(Tile { idx: 0 });
            }
        }
    }

    commands.spawn_bundle(TileMapBundle {
        tile_map,
        transform: TransformBundle {
            local: Transform::from_scale(Vec3::ONE * 3.0),
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn switch_to_next_texture(input: Res<Input<KeyCode>>, mut tile_maps: Query<&mut TileMap>) {
    if input.just_pressed(KeyCode::Space) {
        for mut tile_map in tile_maps.iter_mut() {
            for x in 0..300 {
                for y in 0..300 {
                    if let Some(tile) = &mut tile_map[(x, y, 0)] {
                        tile.idx += 1;
                    }
                }
            }
        }
    }
}
