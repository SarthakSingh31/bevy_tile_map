use bevy::{diagnostic, prelude::*};
use bevy_tilemap::{Tile, TileMap, TileMapBundle, TileMapPlugin, TileSheet};
use rand::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(diagnostic::DiagnosticsPlugin)
        .add_plugin(diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugin(diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(TileMapPlugin)
        .add_startup_system(setup)
        // .add_system(switch_to_next_texture)
        .add_system(switch_tiles_to_random)
        .add_system(control_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut tile_sheets: ResMut<Assets<TileSheet>>,
) {
    let tile_sheet = tile_sheets.add(TileSheet::new(
        vec![asset_server.load("0x72_16x16DungeonTileset.v4.png")],
        UVec2::new(16, 16),
    ));

    let mut tile_map = TileMap::new(
        UVec2::new(1000, 1000),
        UVec2::new(8, 8),
        UVec2::new(16, 16),
        tile_sheet,
    );

    let mut rng = thread_rng();

    for x in 0..tile_map.size.x {
        for y in 0..tile_map.size.y {
            tile_map[(x, y, 0)] = Some(Tile {
                idx: rng.gen_range(0..256),
            });
        }
    }

    commands.spawn_bundle(TileMapBundle {
        tile_map,
        transform: TransformBundle {
            local: Transform::from_scale(Vec3::ONE * 0.01),
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

pub struct TileSwitchTimer(Timer);

impl Default for TileSwitchTimer {
    fn default() -> Self {
        TileSwitchTimer(Timer::new(std::time::Duration::from_secs(1), true))
    }
}

fn switch_tiles_to_random(
    mut timer: Local<TileSwitchTimer>,
    time: Res<Time>,
    mut tile_maps: Query<&mut TileMap>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut rng = thread_rng();

        for mut tile_map in tile_maps.iter_mut() {
            for x in 0..tile_map.size.x {
                for y in 0..tile_map.size.x {
                    if let Some(tile) = &mut tile_map[(x, y, 0)] {
                        tile.idx = rng.gen_range(0..256);
                    }
                }
            }
        }
    }
}

fn control_camera(input: Res<Input<KeyCode>>, mut camera: Query<&mut Transform, With<Camera>>) {
    for mut transform in camera.iter_mut() {
        const SPEED: f32 = 10.0;

        if input.pressed(KeyCode::W) {
            transform.translation.y += SPEED;
        }
        if input.pressed(KeyCode::S) {
            transform.translation.y -= SPEED;
        }
        if input.pressed(KeyCode::A) {
            transform.translation.x -= SPEED;
        }
        if input.pressed(KeyCode::D) {
            transform.translation.x += SPEED;
        }
    }
}
