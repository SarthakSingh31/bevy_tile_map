use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        primitives::Aabb,
    },
    utils::HashMap,
};

use crate::{Tile, TileMap};

#[derive(Debug, Default, Component, Clone, Copy, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct ChunkCoord(pub UVec3);

#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct ChunkEntities(HashMap<ChunkCoord, Entity>);

#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct ChunkMesh(pub(crate) Handle<Mesh>);

#[derive(Debug, Default, Bundle)]
pub struct ChunkBundle {
    mesh: ChunkMesh,
    data: ChunkData,
    aabb: Aabb,
    #[bundle]
    transform: TransformBundle,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
}

#[derive(Debug, Default)]
pub struct ChunkMeshIdCache(HashMap<UVec2, Handle<Mesh>>);

impl ChunkMeshIdCache {
    pub fn get_or_insert_mesh(
        &mut self,
        size: UVec2,
        meshes: &mut ResMut<Assets<Mesh>>,
    ) -> Handle<Mesh> {
        if let Some(mesh_handle) = self.0.get(&size) {
            mesh_handle.as_weak()
        } else {
            const SQUARE: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
            let positions =
                SQUARE.map(|coord| [coord[0] * size.x as f32, coord[1] * size.y as f32, 0.0]);

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, SQUARE.to_vec());

            const INDICES: [u16; 6] = [0, 2, 1, 0, 3, 2];
            mesh.set_indices(Some(Indices::U16(INDICES.to_vec())));

            let mesh_handle = meshes.add(mesh);
            let ret = mesh_handle.as_weak();
            self.0.insert(size, mesh_handle);

            ret
        }
    }
}

pub fn generate_or_update_chunks(
    mut commands: Commands,
    mut chunk_mesh_id_cache: ResMut<ChunkMeshIdCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tile_maps: Query<(Entity, &mut ChunkEntities, &mut TileMap)>,
    mut chunk_meshs: Query<(&mut Aabb, &mut ChunkMesh, &mut ChunkData), Without<TileMap>>,
) {
    for (entity, mut chunk_entities, mut tile_map) in tile_maps.iter_mut() {
        if tile_map.dirty_chunks.len() == 0 {
            continue;
        }

        let screen_chunk_size = tile_map.chunk_size * tile_map.tile_size;

        let mesh_handle = chunk_mesh_id_cache.get_or_insert_mesh(screen_chunk_size, &mut meshes);
        let computed_aabb = meshes
            .get(mesh_handle.clone_weak())
            .unwrap()
            .compute_aabb()
            .unwrap();

        for chunk_coord in tile_map.dirty_chunks.drain().collect::<Vec<_>>() {
            if let Some(chunk) = chunk_entities.get(&chunk_coord) {
                let (mut aabb, mut chunk_mesh, mut chunk_data) = chunk_meshs
                    .get_mut(*chunk)
                    .expect("A chunk for a tile map is missing");

                *aabb = computed_aabb.clone();
                chunk_mesh.0 = mesh_handle.clone_weak();
                chunk_data.sync(&tile_map);
            } else {
                commands.entity(entity).with_children(|child_builder| {
                    let entity = child_builder
                        .spawn_bundle(ChunkBundle {
                            mesh: ChunkMesh(mesh_handle.clone_weak()),
                            aabb: computed_aabb.clone(),
                            data: ChunkData::new(chunk_coord, &tile_map),
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
}

impl ChunkData {
    pub fn new(chunk_coord: ChunkCoord, tile_map: &TileMap) -> Self {
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
}
