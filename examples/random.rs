use bevy::{diagnostic, input::mouse::MouseWheel, prelude::*};
use bevy_tile_map::prelude::*;
use rand::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(diagnostic::DiagnosticsPlugin)
        .add_plugin(diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugin(diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(TileMapPlugin)
        .add_startup_system(setup)
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
        UVec2::new(1024, 1024),
        UVec2::new(128, 128),
        UVec2::new(16, 16),
        tile_sheet,
    );

    let mut rng = thread_rng();

    for x in 0..tile_map.size.x {
        for y in 0..tile_map.size.y {
            tile_map[(x, y, 0)] = Tile {
                entity: None,
                kind: Some(TileKind::Sprite {
                    idx: rng.gen_range(0..256),
                    transform: TileTransform::default(),
                    mask_color: Color::WHITE,
                }),
                pickable: true,
            };
        }
    }

    commands.spawn_bundle(TileMapBundle {
        tile_map,
        transform: TransformBundle {
            local: Transform::from_translation(Vec3::new(-512.0, -512.0, 0.0) * 16.0),
            ..Default::default()
        },
        ..Default::default()
    });
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scale = 25.0;
    commands.spawn_bundle(camera);
}

pub struct TileSwitchTimer(Timer);

impl Default for TileSwitchTimer {
    fn default() -> Self {
        TileSwitchTimer(Timer::new(std::time::Duration::from_millis(100), true))
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
                    let tile = unsafe { tile_map.get_mut_unchecked(UVec3::new(x, y, 0)) };
                    if let Some(TileKind::Sprite { idx, .. }) = &mut tile.kind {
                        *idx = rng.gen_range(0..256);
                    }
                }
            }
            tile_map.mark_all_chunks_dirty();
        }
    }
}

fn control_camera(
    input: Res<Input<KeyCode>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<Camera>>,
) {
    for (mut transform, mut projection) in camera.iter_mut() {
        const SPEED: f32 = 20.0;

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

        const MOUSE_SPEED: f32 = 0.1;

        for event in mouse_wheel_events.iter() {
            projection.scale = (projection.scale - event.y * MOUSE_SPEED).max(0.0001);
        }
    }
}
