use bevy::prelude::*;
use bevy_mod_raycast::*;

use crate::chunk::ChunkData;

pub fn update_camera_ray(
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    mut ray_sources: Query<(&mut TileMapRayCastSource, &Camera, &GlobalTransform)>,
) {
    let window = windows.get_primary().unwrap();

    for (mut source, camera, camera_transform) in ray_sources.iter_mut() {
        if let Some(cursor_pos) = window.cursor_position() {
            *source = source.with_ray_screenspace(
                cursor_pos,
                &windows,
                &images,
                camera,
                camera_transform,
            );
        } else {
            source.cast_method = RayCastMethod::Screenspace(Vec2::ZERO);
        }
    }
}

pub enum TileMapInteractionEvent {
    JustEntered(Entity, UVec3),
    Hovering(Entity, UVec3),
    JustExited(Entity, UVec3),
    Clicked(Entity, UVec3),
}

pub fn queue_interaction_events(
    mut last_selected: Local<Option<(Entity, UVec3)>>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut interaction_writer: EventWriter<TileMapInteractionEvent>,
    ray_source: Query<&TileMapRayCastSource>,
    chunks: Query<(&ChunkData, &Parent)>,
) {
    let source = ray_source.single();

    let mut new_selected = None;
    if let Some(intersections) = source.intersect_list() {
        for (entity, intersection) in intersections {
            if let Ok((chunk_data, tile_map_entity)) = chunks.get(*entity) {
                let position = intersection.position();
                let chunk_tile_coord = (position.truncate() / chunk_data.tile_size.as_vec2())
                    .as_uvec2()
                    % chunk_data.chunk_size;

                if let Some(_) = chunk_data.tiles
                    [(chunk_tile_coord.y * chunk_data.chunk_size.x + chunk_tile_coord.x) as usize]
                {
                    let coord = (chunk_tile_coord
                        + chunk_data.chunk_size * chunk_data.chunk_coord.0.truncate())
                    .extend(chunk_data.chunk_coord.0.z);

                    new_selected = Some((tile_map_entity.0, coord));
                    break;
                }
            }
        }
    }

    if let Some((new_tile_map, new_coord)) = new_selected {
        if let Some((last_tile_map, last_coord)) = last_selected.as_mut() {
            if new_tile_map == *last_tile_map && new_coord == *last_coord {
                interaction_writer.send(TileMapInteractionEvent::Hovering(new_tile_map, new_coord));
            } else {
                interaction_writer.send(TileMapInteractionEvent::JustExited(
                    *last_tile_map,
                    *last_coord,
                ));
                interaction_writer.send(TileMapInteractionEvent::JustEntered(
                    new_tile_map,
                    new_coord,
                ));
            }
        } else {
            interaction_writer.send(TileMapInteractionEvent::JustEntered(
                new_tile_map,
                new_coord,
            ));
        }
        *last_selected = Some((new_tile_map, new_coord));
    } else {
        if let Some((last_tile_map, last_coord)) = last_selected.as_ref() {
            interaction_writer.send(TileMapInteractionEvent::JustExited(
                *last_tile_map,
                *last_coord,
            ));
        }
        *last_selected = None;
    }

    if let Some((tile_map, coord)) = last_selected.as_ref() {
        if mouse_button_input.just_pressed(MouseButton::Left) {
            interaction_writer.send(TileMapInteractionEvent::Clicked(*tile_map, *coord));
        }
    }
}

pub struct TileMapRayCast;

pub type TileMapRayCastMesh = RayCastMesh<TileMapRayCast>;
pub type TileMapRayCastSource = RayCastSource<TileMapRayCast>;
pub type TileMapRayCastPlugin = DefaultRaycastingPlugin<TileMapRayCast>;
