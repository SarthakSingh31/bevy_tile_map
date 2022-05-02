use bevy::{prelude::*, render::primitives::Aabb, utils::HashMap};

use crate::{Tile, TileMap, TileSheet};

#[derive(Debug, Default, Component, Clone, Copy, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct ChunkCoord(pub UVec3);

#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct ChunkEntities(HashMap<ChunkCoord, Entity>);

#[derive(Debug, Default, Bundle)]
pub struct ChunkBundle {
    data: ChunkData,
    aabb: Aabb,
    #[bundle]
    transform: TransformBundle,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
}

pub fn generate_or_update_chunks(
    mut commands: Commands,
    mut tile_maps: Query<(Entity, &mut ChunkEntities, &mut TileMap)>,
    mut chunk_meshs: Query<(&mut Aabb, &mut ChunkData), Without<TileMap>>,
) {
    for (entity, mut chunk_entities, mut tile_map) in tile_maps.iter_mut() {
        if tile_map.dirty_chunks.len() == 0 {
            continue;
        }

        let screen_chunk_size = tile_map.chunk_size * tile_map.tile_size;
        let computed_aabb = Aabb::from_min_max(Vec3::ZERO, screen_chunk_size.extend(0).as_vec3());

        for chunk_coord in tile_map.dirty_chunks.drain().collect::<Vec<_>>() {
            if let Some(chunk) = chunk_entities.get(&chunk_coord) {
                let (mut aabb, mut chunk_data) = chunk_meshs
                    .get_mut(*chunk)
                    .expect("A chunk for a tile map is missing");

                *aabb = computed_aabb.clone();
                chunk_data.sync(&tile_map);
            } else {
                commands.entity(entity).with_children(|child_builder| {
                    let entity = child_builder
                        .spawn_bundle(ChunkBundle {
                            aabb: computed_aabb.clone(),
                            data: ChunkData::new(
                                chunk_coord,
                                &tile_map,
                                tile_map.tile_sheet.as_weak(),
                            ),
                            transform: TransformBundle {
                                local: Transform::from_translation(
                                    (chunk_coord.0 * screen_chunk_size.extend(1)).as_vec3(),
                                ),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .id();
                    chunk_entities.insert(chunk_coord, entity);
                });
            }
        }
    }
}

#[derive(Debug, Default, Component, Clone)]
pub struct ChunkData {
    tiles: Vec<Option<Tile>>,
    chunk_coord: ChunkCoord,
    chunk_size: UVec2,
    tile_size: UVec2,
    tile_sheet: Handle<TileSheet>,
}

impl ChunkData {
    pub fn new(chunk_coord: ChunkCoord, tile_map: &TileMap, tile_sheet: Handle<TileSheet>) -> Self {
        let mut tiles = vec![None; (tile_map.chunk_size.x * tile_map.chunk_size.y) as usize];

        Self::copy_tiles(
            &mut tiles,
            &tile_map.tiles[chunk_coord.z as usize],
            chunk_coord.0.truncate(),
            tile_map.chunk_size,
            tile_map.size.truncate(),
        );

        ChunkData {
            tiles,
            chunk_coord,
            chunk_size: tile_map.chunk_size,
            tile_size: tile_map.tile_size,
            tile_sheet,
        }
    }

    pub fn sync(&mut self, tile_map: &TileMap) {
        self.tile_size = tile_map.tile_size;

        Self::copy_tiles(
            &mut self.tiles,
            &tile_map.tiles[self.chunk_coord.z as usize],
            self.chunk_coord.0.truncate(),
            tile_map.chunk_size,
            tile_map.size.truncate(),
        );
    }

    fn copy_tiles(
        dest: &mut [Option<Tile>],
        src: &[Option<Tile>],
        chunk_coord: UVec2,
        chunk_size: UVec2,
        tile_map_size: UVec2,
    ) {
        let start_tile_coord = chunk_coord * chunk_size;
        let copy_width = (tile_map_size.x - start_tile_coord.x).min(chunk_size.x) as usize;

        for y in 0..chunk_size.y {
            let row_start_tile_coord = start_tile_coord + UVec2::new(0, y);
            if row_start_tile_coord.y < tile_map_size.y {
                let dest_start = (y * chunk_size.x) as usize;
                let dest_end = dest_start + copy_width;

                let src_start =
                    (row_start_tile_coord.y * tile_map_size.x + row_start_tile_coord.x) as usize;
                let src_end = src_start + copy_width;

                dest[dest_start..dest_end].copy_from_slice(&src[src_start..src_end]);
            }
        }
    }

    pub fn tiles(&self) -> &Vec<Option<Tile>> {
        &self.tiles
    }

    pub fn tile_size(&self) -> UVec2 {
        self.tile_size
    }

    pub fn chunk_size(&self) -> UVec2 {
        self.chunk_size
    }

    pub fn tile_sheet(&self) -> &Handle<TileSheet> {
        &self.tile_sheet
    }
}
