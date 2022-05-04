use bevy::{diagnostic, input::mouse::MouseWheel, prelude::*, utils::HashMap};
use bevy_tile_map::{
    AsTiles, Tile, TileMap, TileMapBundle, TileMapPlugin, TileSheet, TileTransform,
};

// Controls: Arrow Up, Arrow Down, Arrow Left, Arrow Right to move the multi tile sprite.

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(diagnostic::DiagnosticsPlugin)
        .add_plugin(diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugin(diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(TileMapPlugin)
        .add_startup_system(setup)
        .add_system(move_character)
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
        vec![
            asset_server.load("0x72_16x16DungeonTileset.v4.png"),
            asset_server.load("0x72_16x16DungeonTileset_walls.v2.png"),
        ],
        UVec2::new(16, 16),
    ));

    let mut tile_map = TileMap::new(
        UVec2::new(256, 256),
        UVec2::new(32, 32),
        UVec2::new(16, 16),
        tile_sheet,
    );

    for x in 0..tile_map.size.x {
        for y in 0..tile_map.size.y {
            tile_map[(x, y, 0)] = Tile::Sprite {
                idx: 364,
                transform: TileTransform::default(),
                mask_color: Color::WHITE,
            };
        }
    }

    tile_map.add_empty_layer();

    let window = windows.get_primary().unwrap();
    let tile_map_entity = commands
        .spawn_bundle(TileMapBundle {
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
        })
        .id();
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let scale = 0.8;
    commands.spawn().insert(AsTiles {
        coord: UVec3::new(7, 7, 1),
        tiles: HashMap::from_iter([
            (
                UVec3::new(0, 1, 0),
                Tile::Sprite {
                    idx: 186,
                    transform: TileTransform {
                        translation: Vec2::new(1.0 - scale, 0.0) * (1.0 / scale),
                        scale: Vec2::new(scale, scale),
                        ..Default::default()
                    },
                    mask_color: Color::WHITE,
                },
            ),
            (
                UVec3::new(1, 1, 0),
                Tile::Sprite {
                    idx: 187,
                    transform: TileTransform {
                        scale: Vec2::new(scale, scale),
                        ..Default::default()
                    },
                    mask_color: Color::WHITE,
                },
            ),
            (
                UVec3::new(0, 0, 0),
                Tile::Sprite {
                    idx: 202,
                    transform: TileTransform {
                        translation: Vec2::new(1.0 - scale, 1.0 - scale) * (1.0 / scale),
                        scale: Vec2::new(scale, scale),
                        ..Default::default()
                    },
                    mask_color: Color::WHITE,
                },
            ),
            (
                UVec3::new(1, 0, 0),
                Tile::Sprite {
                    idx: 203,
                    transform: TileTransform {
                        translation: Vec2::new(0.0, 1.0 - scale) * (1.0 / scale),
                        scale: Vec2::new(scale, scale),
                        ..Default::default()
                    },
                    mask_color: Color::WHITE,
                },
            ),
        ]),
        tile_map_entity,
    });
}

fn move_character(input: Res<Input<KeyCode>>, mut on_map_entity: Query<&mut AsTiles>) {
    let direction = if input.just_pressed(KeyCode::Up) {
        IVec2::new(0, 1)
    } else if input.just_pressed(KeyCode::Down) {
        IVec2::new(0, -1)
    } else if input.just_pressed(KeyCode::Left) {
        IVec2::new(-1, 0)
    } else if input.just_pressed(KeyCode::Right) {
        IVec2::new(1, 0)
    } else {
        IVec2::new(0, 0)
    };

    if let Ok(mut as_tiles) = on_map_entity.get_single_mut() {
        as_tiles.coord = (as_tiles.coord.as_ivec3() + direction.extend(0)).as_uvec3();
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
