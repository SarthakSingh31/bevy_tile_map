use std::ops::{Index, IndexMut};

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::{
    chunk::{ChunkCoord, ChunkEntities},
    TileSheet,
};

#[derive(Debug, Default, Component)]
pub struct TileMap {
    pub(crate) tiles: Vec<Vec<Tile>>,
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
            tiles: vec![vec![Tile::default(); (size.x * size.y) as usize]; 1],
            size: size.extend(1),
            chunk_size,
            tile_size,
            dirty_chunks: HashSet::default(),
            tile_sheet,
        }
    }

    pub fn get(&self, coord: UVec3) -> Option<&Tile> {
        let index = self.coord_to_tile_idx(coord.truncate());
        if let Some(layer) = self.tiles.get(coord.z as usize) {
            layer.get(index)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, coord: UVec3) -> Option<&mut Tile> {
        self.mark_chunk_dirty(coord);

        let index = self.coord_to_tile_idx(coord.truncate());
        if let Some(layer) = self.tiles.get_mut(coord.z as usize) {
            layer.get_mut(index)
        } else {
            None
        }
    }

    /// SAFETY: Does not mark the chunk as dirty. Does not do bound checks. So you need to do both yourself.
    pub unsafe fn get_mut_unchecked(&mut self, coord: UVec3) -> &mut Tile {
        let index = self.coord_to_tile_idx(coord.truncate());
        &mut self.tiles[coord.z as usize][index]
    }

    pub fn add_empty_layer(&mut self) -> u32 {
        self.size.z += 1;
        self.mark_all_chunks_dirty();

        self.tiles
            .push(vec![Tile::default(); (self.size.x * self.size.y) as usize]);
        self.tiles.len() as u32 - 1
    }

    pub fn add_layer(&mut self, tiles: Vec<Tile>) -> u32 {
        self.size.z += 1;
        self.mark_all_chunks_dirty();

        self.tiles.push(tiles);
        self.tiles.len() as u32 - 1
    }

    #[inline]
    pub fn size(&self) -> UVec3 {
        self.size
    }

    #[inline]
    pub fn chunks(&self) -> impl IntoIterator<Item = ChunkCoord> {
        let max = self.coord_to_chunk_coord(self.size).0;

        (0..max.x)
            .flat_map(move |x| (0..max.y).map(move |y| UVec2::new(x, y)))
            .flat_map(move |xy| (0..max.z).map(move |z| xy.extend(z)))
            .map(|coord| ChunkCoord(coord))
    }

    #[inline]
    pub(crate) fn coord_to_tile_idx(&self, index: UVec2) -> usize {
        (index.y * self.size.x + index.x) as usize
    }

    #[inline]
    pub(crate) fn coord_to_chunk_coord(&self, coord: UVec3) -> ChunkCoord {
        ChunkCoord((coord.truncate() / self.chunk_size).extend(coord.z))
    }

    #[inline]
    pub fn mark_chunk_dirty(&mut self, coord: UVec3) {
        self.dirty_chunks.insert(self.coord_to_chunk_coord(coord));
    }

    #[inline]
    pub fn mark_all_chunks_dirty(&mut self) {
        self.dirty_chunks.extend(self.chunks());
    }
}

impl Index<UVec3> for TileMap {
    type Output = Tile;

    #[inline]
    fn index(&self, coord: UVec3) -> &Self::Output {
        let index = self.coord_to_tile_idx(coord.truncate());
        &self.tiles[coord.z as usize][index]
    }
}

impl IndexMut<UVec3> for TileMap {
    #[inline]
    fn index_mut(&mut self, coord: UVec3) -> &mut Self::Output {
        self.mark_chunk_dirty(coord);

        let index = self.coord_to_tile_idx(coord.truncate());
        &mut self.tiles[coord.z as usize][index]
    }
}

impl Index<(u32, u32, u32)> for TileMap {
    type Output = Tile;

    #[inline]
    fn index(&self, coord: (u32, u32, u32)) -> &Self::Output {
        &self[UVec3::from(coord)]
    }
}

impl IndexMut<(u32, u32, u32)> for TileMap {
    #[inline]
    fn index_mut(&mut self, coord: (u32, u32, u32)) -> &mut Self::Output {
        &mut self[UVec3::from(coord)]
    }
}

impl Index<[u32; 3]> for TileMap {
    type Output = Tile;

    #[inline]
    fn index(&self, coord: [u32; 3]) -> &Self::Output {
        &self[UVec3::from(coord)]
    }
}

impl IndexMut<[u32; 3]> for TileMap {
    #[inline]
    fn index_mut(&mut self, coord: [u32; 3]) -> &mut Self::Output {
        &mut self[UVec3::from(coord)]
    }
}

#[derive(Debug, Default, Component, Clone, Copy, PartialEq)]
pub struct Tile {
    pub entity: Option<Entity>,
    pub kind: Option<TileKind>,
    pub pickable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileKind {
    Color(Color),
    Sprite {
        idx: u16,
        transform: TileTransform,
        mask_color: Color,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileTransform {
    pub angle: f32,
    pub translation: Vec2,
    pub scale: Vec2,
}

impl TileTransform {
    pub fn recenter(&self) -> Self {
        let current: Mat3 = self.clone().into();
        let offset = current.transform_point2(Vec2::new(0.5, 0.5));

        Self {
            angle: self.angle,
            translation: self.translation - (Vec2::new(0.5, 0.5) - offset),
            scale: self.scale,
        }
    }
}

impl Default for TileTransform {
    fn default() -> Self {
        TileTransform {
            translation: Vec2::ZERO,
            angle: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl Into<Mat3> for TileTransform {
    fn into(self) -> Mat3 {
        Mat3::from_scale_angle_translation(Vec2::ONE / self.scale, self.angle, -self.translation)
    }
}

impl Into<Mat3> for &TileTransform {
    fn into(self) -> Mat3 {
        Mat3::from_scale_angle_translation(Vec2::ONE / self.scale, self.angle, -self.translation)
    }
}

impl Into<Mat4> for TileTransform {
    fn into(self) -> Mat4 {
        Mat4::from_mat3(self.into())
    }
}

impl Into<Mat4> for &TileTransform {
    fn into(self) -> Mat4 {
        Mat4::from_mat3(self.into())
    }
}

#[derive(Default, Bundle)]
pub struct TileMapBundle {
    pub tile_map: TileMap,
    pub chunks: ChunkEntities,
    #[bundle]
    pub transform: TransformBundle,
}

#[derive(Debug, Component, Clone, PartialEq)]
pub struct AsTiles {
    pub coord: UVec3,
    pub tiles: HashMap<UVec3, TileKind>,
    pub tile_map_entity: Entity,
}

pub(crate) fn sync_as_tiles(
    mut sync_cache: Local<HashMap<Entity, AsTiles>>,
    mut tile_maps: Query<&mut TileMap>,
    as_tiles_query: Query<(Entity, &AsTiles)>,
) {
    for (as_tiles_entity, as_tiles) in as_tiles_query.iter() {
        if let Ok(mut tile_map) = tile_maps.get_mut(as_tiles.tile_map_entity) {
            if let Some(old_tiles) = sync_cache.get_mut(&as_tiles_entity) {
                if old_tiles == as_tiles {
                    continue;
                }

                for (coord, ..) in &old_tiles.tiles {
                    tile_map[old_tiles.coord + *coord] = Tile::default();
                }
                *old_tiles = as_tiles.clone();
            } else {
                sync_cache.insert(as_tiles_entity, as_tiles.clone());
            }

            for (coord, tile_kind) in &as_tiles.tiles {
                tile_map[as_tiles.coord + *coord] = Tile {
                    entity: Some(as_tiles_entity),
                    kind: Some(*tile_kind),
                    pickable: true,
                };
            }
        } else {
            warn!("TileMap entity for a AsTiles does not exist");
        }
    }
}
