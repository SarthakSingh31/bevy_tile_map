use std::ops::{Index, IndexMut};

use bevy::{prelude::*, reflect::TypeUuid};

use crate::chunk::ChunkEntities;

#[derive(Debug, Default, Component)]
pub struct TileMap {
    pub(crate) tiles: Vec<Vec<Option<Tile>>>,
    pub(crate) size: UVec2,
    pub tile_size: Vec2,
}

impl TileMap {
    pub fn new(size: UVec2, tile_size: Vec2) -> Self {
        TileMap {
            tiles: vec![vec![None; (size.x * size.y) as usize]; 1],
            size,
            tile_size,
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
        let index = self.coord_to_tile_idx(coord.truncate());
        if let Some(layer) = self.tiles.get_mut(coord.z as usize) {
            layer.get_mut(index)
        } else {
            None
        }
    }

    pub fn add_empty_layer(&mut self) -> u32 {
        self.tiles
            .push(vec![None; (self.size.x * self.size.y) as usize]);
        self.tiles.len() as u32 - 1
    }

    pub fn add_layer(&mut self, tiles: Vec<Option<Tile>>) -> u32 {
        self.tiles.push(tiles);
        self.tiles.len() as u32 - 1
    }

    pub fn layer_count(&self) -> usize {
        self.tiles.len()
    }

    pub(crate) fn coord_to_tile_idx(&self, index: UVec2) -> usize {
        (index.y * self.size.x + index.x) as usize
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

// impl Index<UVec2> for TileMap {
//     type Output = HashMap<u16, Tile>;

//     fn index(&self, index: UVec2) -> &Self::Output {
//         let index = self.coord_to_tile_idx(index);
//         &self.tiles[index]
//     }
// }

// impl IndexMut<UVec2> for TileMap {
//     fn index_mut(&mut self, index: UVec2) -> &mut Self::Output {
//         let index = self.coord_to_tile_idx(index);
//         &mut self.tiles[index]
//     }
// }

// impl Index<(u32, u32)> for TileMap {
//     type Output = HashMap<u16, Tile>;

//     fn index(&self, index: (u32, u32)) -> &Self::Output {
//         let index: UVec2 = index.into();
//         &self[index]
//     }
// }

// impl IndexMut<(u32, u32)> for TileMap {
//     fn index_mut(&mut self, index: (u32, u32)) -> &mut Self::Output {
//         let index: UVec2 = index.into();
//         &mut self[index]
//     }
// }

#[derive(Debug, Clone)]
pub struct Tile {
    pub tile_sheet: Handle<TileSheet>,
    pub tile_idx: u8,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "fd3a76be-60a3-4b67-a2da-8c987f65ae16"]
pub struct TileSheet {
    pub tile_sheet: Handle<Image>,
    pub tile_size: Vec2,
}

#[derive(Default, Bundle)]
pub struct TileMapBundle {
    pub tile_map: TileMap,
    pub chunks: ChunkEntities,
    #[bundle]
    pub transform: TransformBundle,
}
