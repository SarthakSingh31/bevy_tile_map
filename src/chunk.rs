use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        primitives::Aabb,
    },
    utils::HashMap,
};

use crate::TileMap;

#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct ChunkEntities(HashMap<UVec3, Entity>);

#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct ChunkMesh(pub(crate) Option<Handle<Mesh>>);

#[derive(Debug, Default, Bundle)]
pub struct ChunkBundle {
    mesh: ChunkMesh,
    aabb: Aabb,
    #[bundle]
    transform: TransformBundle,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
}

#[derive(Debug, Deref, DerefMut)]
pub struct ChunkSize(UVec2);

impl Default for ChunkSize {
    fn default() -> Self {
        ChunkSize(UVec2::new(8, 8))
    }
}

pub fn generate_or_update_chunks(
    mut commands: Commands,
    chunk_size: Res<ChunkSize>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tile_maps: Query<(Entity, &mut ChunkEntities, &TileMap), Changed<TileMap>>,
    mut chunk_meshs: Query<(&mut Aabb, &mut ChunkMesh), Without<TileMap>>,
) {
    for (entity, mut chunk_entities, tile_map) in tile_maps.iter_mut() {
        for layer in 0..tile_map.layer_count() {
            let mut coord_counts = tile_map.size / chunk_size.0;
            if tile_map.size % chunk_size.0 != UVec2::ZERO {
                coord_counts += UVec2::ONE;
            }

            for x in 0..coord_counts.x {
                for y in 0..coord_counts.y {
                    let coord = UVec2::new(x, y);

                    if let Some(chunk) = chunk_entities.get(&coord.extend(layer as u32)) {
                        let (mut aabb, mut chunk_mesh) = chunk_meshs
                            .get_mut(*chunk)
                            .expect("A chunk for a tile map is missing");

                        chunk_mesh.0 =
                            build_mesh(coord, chunk_size.0, tile_map, layer as u32).map(|mesh| {
                                *aabb = mesh.compute_aabb().unwrap();
                                meshes.add(mesh)
                            });
                    } else {
                        commands.entity(entity).with_children(|child_builder| {
                            let mut aabb = Aabb::default();
                            let mesh_handle =
                                build_mesh(coord, chunk_size.0, tile_map, 0).map(|mesh| {
                                    aabb = mesh.compute_aabb().unwrap();
                                    meshes.add(mesh)
                                });
                            let entity = child_builder
                                .spawn_bundle(ChunkBundle {
                                    mesh: ChunkMesh(mesh_handle),
                                    aabb,
                                    transform: TransformBundle {
                                        local: Transform::from_xyz(
                                            x as f32 * chunk_size.x as f32 * tile_map.tile_size.x,
                                            y as f32 * chunk_size.y as f32 * tile_map.tile_size.y,
                                            0.0,
                                        ),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .id();
                            chunk_entities.insert(coord.extend(layer as u32), entity);
                        });
                    }
                }
            }
        }
    }
}

fn build_mesh(
    chunk_coord: UVec2,
    chunk_size: UVec2,
    tile_map: &TileMap,
    layer: u32,
) -> Option<Mesh> {
    const SQUARE_UVS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];

    let step_x = tile_map.tile_size.x;
    let step_y = tile_map.tile_size.y;
    let scaled_square_positions = SQUARE_UVS.map(|pos| [pos[0] * step_x, pos[1] * step_y]);
    let start_index = chunk_coord * chunk_size;

    let mut positions = Vec::with_capacity((chunk_size.x * chunk_size.y * 4) as usize);
    let mut uvs = Vec::with_capacity((chunk_size.x * chunk_size.y * 4) as usize);
    let mut indices = Vec::with_capacity((chunk_size.x * chunk_size.y * 6) as usize);

    let mut next_indices: [u16; 6] = [0, 2, 1, 0, 3, 2];

    for x in 0..chunk_size.x {
        for y in 0..chunk_size.y {
            if let Some(Some(_)) = tile_map.get((start_index + UVec2::new(x, y)).extend(layer)) {
                let x = x as f32;
                let y = y as f32;

                for position in scaled_square_positions {
                    positions.push([
                        position[0] + step_x * x,
                        position[1] + step_y * y,
                        layer as f32,
                    ]);
                }
                uvs.extend(SQUARE_UVS);

                indices.extend(next_indices);
                next_indices = next_indices.map(|index| index + 4);
            }
        }
    }

    if positions.len() != 0 {
        positions.shrink_to_fit();
        uvs.shrink_to_fit();
        indices.shrink_to_fit();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U16(indices)));

        Some(mesh)
    } else {
        None
    }
}
