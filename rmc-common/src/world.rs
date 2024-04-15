use std::rc::Rc;

use itertools::Itertools;
use ndarray::Array3;
use vek::Vec3;

use crate::{Block, DiscreteBlend};

pub const CHUNK_SIZE: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub blocks: Rc<Array3<Block>>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk::default()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Chunk {
            blocks: Rc::new(Array3::from_elem(
                (CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE),
                Block {
                    open_to_sky: true,
                    ..Block::AIR
                },
            )),
        }
    }
}

#[derive(Clone)]
pub struct World {
    pub origin: Vec3<i32>,

    // None means unloaded
    pub chunks: Array3<Option<Chunk>>,

    // Half width to the sides, excluding middle. i.e (chunks.dim() - 1) / 2
    pub extents: Vec3<i32>,
}

impl World {
    pub fn new(origin: Vec3<i32>) -> Self {
        let extents = Vec3::one();
        World {
            chunks: Array3::from_shape_simple_fn(
                (extents * 2 + Vec3::one()).as_().into_tuple(),
                || Some(Chunk::default()),
            ),
            extents,
            origin,
        }
    }

    pub fn block_to_chunk(&self, position: Vec3<i32>) -> Vec3<i32> {
        position.map(|e| (e as f32 / CHUNK_SIZE as f32).floor() as i32)
    }

    pub fn chunk_index(&self, position: Vec3<i32>) -> Option<Vec3<usize>> {
        let chunk_coord = self.block_to_chunk(position);
        let offset = chunk_coord - self.origin;

        if offset.zip(self.extents).into_iter().any(|(o, e)| o > e) {
            return None;
        }

        Some((offset + self.extents).as_())
    }

    /// World coords to chunk.
    pub fn chunk_at(&self, position: Vec3<i32>) -> Option<Chunk> {
        self.chunks
            .get(self.chunk_index(position)?.into_tuple())
            .cloned()
            .flatten()
    }

    /// World coords to chunk.
    pub fn chunk_at_mut(&mut self, position: Vec3<i32>) -> Option<&mut Chunk> {
        self.chunks
            .get_mut(self.chunk_index(position)?.into_tuple())
            .map(|r| r.as_mut())
            .flatten()
    }

    pub fn get_block(&self, position: Vec3<i32>) -> Option<Block> {
        let chunk = self.chunk_at(position)?;
        let chunk_offset = position.map(|e| (e as i32).rem_euclid(CHUNK_SIZE as i32));

        chunk.blocks.get(chunk_offset.as_().into_tuple()).cloned()
    }

    pub fn set_block(&mut self, position: Vec3<i32>, block: Block) {
        let chunk = self.chunk_at_mut(position).unwrap();
        let chunk_offset = position.map(|e| (e as i32).rem_euclid(CHUNK_SIZE as i32));

        let mut new_blocks = Rc::unwrap_or_clone(Rc::clone(&chunk.blocks));
        new_blocks[chunk_offset.as_().into_tuple()] = block;
        chunk.blocks = Rc::new(new_blocks);
    }

    pub fn chunks_iter(&self) -> impl Iterator<Item = (Vec3<i32>, Chunk)> + '_ {
        self.chunks.indexed_iter().filter_map(|(index, chunk)| {
            chunk.clone().map(|chunk| {
                (
                    Vec3::<usize>::from(index).as_::<i32>() - self.extents + self.origin,
                    chunk,
                )
            })
        })
    }
}

impl Default for World {
    fn default() -> Self {
        World::new(Vec3::zero())
    }
}

impl DiscreteBlend for World {}

#[test]
fn test_world() {
    let mut world = World::default();
    assert!(world.chunk_at(Vec3::new(4, 4, 4)).is_some());
    assert!(world.chunk_at(Vec3::new(-4, 4, 4)).is_some());
    assert!(world.chunk_at(Vec3::new(-4, 4, -8)).is_some());
    assert_eq!(world.chunk_at(Vec3::new(-20, 4, 4)), None);

    assert_eq!(world.get_block(Vec3::new(-4, 4, -2)), Some(Block::AIR));

    let chunk = (&mut world.chunks[(0, 1, 0)]).as_mut().unwrap();
    let mut new_blocks = Rc::unwrap_or_clone(Rc::clone(&chunk.blocks));
    new_blocks[(12, 4, 14)] = Block::GRASS;
    chunk.blocks = Rc::new(new_blocks);

    assert_eq!(world.get_block(Vec3::new(-4, 4, -2)), Some(Block::GRASS));

    assert_eq!(world.get_block(Vec3::new(-4, 4, -1)), Some(Block::AIR));
    world.set_block(Vec3::new(-4, 4, -1), Block::GRASS);
    assert_eq!(world.get_block(Vec3::new(-4, 4, -1)), Some(Block::GRASS));
}

pub fn face_to_normal(face: u8) -> Vec3<i32> {
    match face {
        0 => Vec3::unit_x(),
        1 => Vec3::unit_y(),
        2 => Vec3::unit_z(),
        3 => -Vec3::unit_x(),
        4 => -Vec3::unit_y(),
        5 => -Vec3::unit_z(),
        _ => unreachable!(),
    }
}

pub fn face_neighbors(position: Vec3<i32>) -> [Vec3<i32>; 6] {
    [0, 1, 2, 3, 4, 5].map(|face| position + face_to_normal(face))
}

pub fn surrounding_neighbors(position: Vec3<i32>) -> [Vec3<i32>; 6 + 8] {
    face_neighbors(position)
        .into_iter()
        .chain(
            [
                Vec3::new(1, 1, 1),
                Vec3::new(1, 1, -1),
                Vec3::new(1, -1, 1),
                Vec3::new(1, -1, -1),
                Vec3::new(-1, 1, 1),
                Vec3::new(-1, 1, -1),
                Vec3::new(-1, -1, 1),
                Vec3::new(-1, -1, -1),
            ]
            .into_iter()
            .map(|o| position + o),
        )
        .collect_vec()
        .try_into()
        .unwrap()
}
