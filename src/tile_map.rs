use std::ops::{Index, IndexMut};

use bevy::{prelude::*, utils::HashSet};

use crate::{
    chunk::{ChunkCoord, ChunkEntities},
    TileSheet,
};

#[derive(Debug, Default, Component)]
pub struct TileMap {
    pub(crate) tiles: Vec<Vec<Option<Tile>>>,
    pub size: UVec3,
    pub chunk_size: UVec2,
    pub tile_size: UVec2,
    pub(crate) dirty_chunks: HashSet<ChunkCoord>,
    pub(crate) tile_sheet: Handle<TileSheet>,
}

impl TileMap {
    pub fn new(
        size: UVec2,
        chunk_size: UVec2,
        tile_size: UVec2,
        tile_sheet: Handle<TileSheet>,
    ) -> Self {
        assert!(chunk_size.x >= 1 && chunk_size.y >= 1);

        TileMap {
            tiles: vec![vec![None; (size.x * size.y) as usize]; 1],
            size: size.extend(1),
            chunk_size,
            tile_size,
            dirty_chunks: HashSet::default(),
            tile_sheet,
        }
    }

    pub fn get(&self, coord: UVec3) -> Option<&Option<Tile>> {
        let index = self.coord_to_tile_idx(coord.truncate());
        if let Some(layer) = self.tiles.get(coord.z as usize) {
            layer.get(index)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, coord: UVec3) -> Option<&mut Option<Tile>> {
        self.mark_chunk_dirty(coord);

        let index = self.coord_to_tile_idx(coord.truncate());
        if let Some(layer) = self.tiles.get_mut(coord.z as usize) {
            layer.get_mut(index)
        } else {
            None
        }
    }

    pub fn add_empty_layer(&mut self) -> u32 {
        self.size.z += 1;
        self.mark_all_chunks_dirty();

        self.tiles
            .push(vec![None; (self.size.x * self.size.y) as usize]);
        self.tiles.len() as u32 - 1
    }

    pub fn add_layer(&mut self, tiles: Vec<Option<Tile>>) -> u32 {
        self.size.z += 1;
        self.mark_all_chunks_dirty();

        self.tiles.push(tiles);
        self.tiles.len() as u32 - 1
    }

    pub fn size(&self) -> UVec3 {
        self.size
    }

    pub fn chunks(&self) -> impl IntoIterator<Item = ChunkCoord> {
        let max = self.coord_to_chunk_coord(self.size).0;

        (0..max.x)
            .flat_map(move |x| (0..max.y).map(move |y| UVec2::new(x, y)))
            .flat_map(move |xy| (0..max.z).map(move |z| xy.extend(z)))
            .map(|coord| ChunkCoord(coord))
    }

    pub(crate) fn coord_to_tile_idx(&self, index: UVec2) -> usize {
        (index.y * self.size.x + index.x) as usize
    }

    pub(crate) fn coord_to_chunk_coord(&self, coord: UVec3) -> ChunkCoord {
        ChunkCoord((coord.truncate() / self.chunk_size).extend(coord.z))
    }

    fn mark_chunk_dirty(&mut self, coord: UVec3) {
        self.dirty_chunks.insert(self.coord_to_chunk_coord(coord));
    }

    fn mark_all_chunks_dirty(&mut self) {
        self.dirty_chunks.extend(self.chunks());
    }
}

impl Index<UVec3> for TileMap {
    type Output = Option<Tile>;

    fn index(&self, coord: UVec3) -> &Self::Output {
        assert!(coord.x < self.size.x && coord.y < self.size.y);
        let index = self.coord_to_tile_idx(coord.truncate());
        &self.tiles[coord.z as usize][index]
    }
}

impl IndexMut<UVec3> for TileMap {
    fn index_mut(&mut self, coord: UVec3) -> &mut Self::Output {
        assert!(coord.x < self.size.x && coord.y < self.size.y);
        self.mark_chunk_dirty(coord);

        let index = self.coord_to_tile_idx(coord.truncate());
        &mut self.tiles[coord.z as usize][index]
    }
}

impl Index<(u32, u32, u32)> for TileMap {
    type Output = Option<Tile>;

    fn index(&self, coord: (u32, u32, u32)) -> &Self::Output {
        &self[UVec3::from(coord)]
    }
}

impl IndexMut<(u32, u32, u32)> for TileMap {
    fn index_mut(&mut self, coord: (u32, u32, u32)) -> &mut Self::Output {
        &mut self[UVec3::from(coord)]
    }
}

impl Index<[u32; 3]> for TileMap {
    type Output = Option<Tile>;

    fn index(&self, coord: [u32; 3]) -> &Self::Output {
        &self[UVec3::from(coord)]
    }
}

impl IndexMut<[u32; 3]> for TileMap {
    fn index_mut(&mut self, coord: [u32; 3]) -> &mut Self::Output {
        &mut self[UVec3::from(coord)]
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Tile {
    pub idx: u16,
}

#[derive(Default, Bundle)]
pub struct TileMapBundle {
    pub tile_map: TileMap,
    pub chunks: ChunkEntities,
    #[bundle]
    pub transform: TransformBundle,
}
