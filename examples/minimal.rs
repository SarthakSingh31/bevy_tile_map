use bevy::{diagnostic, input::mouse::MouseWheel, prelude::*};
use bevy_tile_map::prelude::*;

// Controls: Arrow Up, Arrow Down, Arrow Left, Arrow Right to navigate the sprite sheet.

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(diagnostic::DiagnosticsPlugin)
        .add_plugin(diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugin(diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(TileMapPlugin)
        .add_startup_system(setup)
        .add_system(switch_to_next_texture)
        .add_system(control_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    windows: Res<Windows>,
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

    for x in 0..tile_map.size.x {
        for y in 0..tile_map.size.y {
            tile_map[(x, y, 0)] = Tile {
                entity: None,
                kind: Some(TileKind::Sprite {
                    idx: 0,
                    transform: TileTransform::default(),
                    mask_color: Color::WHITE,
                }),
                pickable: true,
            };
        }
    }

    let window = windows.get_primary().unwrap();
    commands.spawn_bundle(TileMapBundle {
        tile_map,
        transform: TransformBundle {
            local: Transform::from_translation(Vec3::new(
                -window.width() / 2.0,
                -window.height() / 2.0,
                0.0,
            )),
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn switch_to_next_texture(input: Res<Input<KeyCode>>, mut tile_maps: Query<&mut TileMap>) {
    let direction: i32 = if input.just_pressed(KeyCode::Up) {
        -16
    } else if input.just_pressed(KeyCode::Down) {
        16
    } else if input.just_pressed(KeyCode::Left) {
        -1
    } else if input.just_pressed(KeyCode::Right) {
        1
    } else {
        0
    };

    if direction != 0 {
        for mut tile_map in tile_maps.iter_mut() {
            for layer in 0..tile_map.size.z {
                for x in 0..tile_map.size.x {
                    for y in 0..tile_map.size.x {
                        if let Some(TileKind::Sprite { idx, .. }) =
                            &mut tile_map[(x, y, layer)].kind
                        {
                            *idx = (*idx as i32 + direction).clamp(0, 256) as u16;
                        }
                    }
                }
            }
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
