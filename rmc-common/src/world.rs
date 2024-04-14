use std::{cmp::Ordering, rc::Rc};

use ndarray::Array3;
use vek::Vec3;

use crate::DiscreteBlend;

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct Block {
    pub id: u8,
    pub light: u8,
}

impl Block {
    pub const AIR: Block = Block { id: 0, light: 0 };
    pub const TEST: Block = Block { id: 1, light: 0 };
    pub const GRASS: Block = Block { id: 2, light: 0 };
    pub const LANTERN: Block = Block { id: 3, light: 0 };

    // Transparent rendering is hard :(
    pub const MESH: Block = Block { id: 4, light: 0 };

    pub const WOOD: Block = Block { id: 5, light: 0 };
}

impl DiscreteBlend for Block {}

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
            blocks: Rc::new(Array3::default((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE))),
        }
    }
}

// trait ChebyshevDistance {
//     type Output;
//     fn chebyshev_distance(self, other: Self) -> Self::Output;
// }

// impl ChebyshevDistance for Vec3<i32> {
//     type Output = u32;
//     fn chebyshev_distance(self, other: Vec3<i32>) -> u32 {
//         self.x
//             .abs_diff(other.x)
//             .max(self.y.abs_diff(other.y))
//             .max(self.z.abs_diff(other.z))
//     }
// }

#[derive(Clone)]
pub struct World {
    pub origin: Vec3<i32>,
    pub chunks: Array3<Chunk>,

    // Half width to the sides, excluding middle. i.e (chunks.dim() - 1) / 2
    pub extents: Vec3<i32>,
}

impl World {
    pub fn new(origin: Vec3<i32>) -> Self {
        let extents = Vec3::one();
        World {
            chunks: Array3::default((extents * 2 + Vec3::one()).as_().into_tuple()),
            extents,
            origin,
        }
    }

    pub fn chunk_index(&self, position: Vec3<i32>) -> Option<Vec3<usize>> {
        let chunk_coord = position.map(|e| (e as f32 / CHUNK_SIZE as f32).floor() as i32);
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
    }

    /// World coords to chunk.
    pub fn chunk_at_mut(&mut self, position: Vec3<i32>) -> Option<&mut Chunk> {
        self.chunks
            .get_mut(self.chunk_index(position)?.into_tuple())
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
        self.chunks.indexed_iter().map(|(index, chunk)| {
            (
                Vec3::<usize>::from(index).as_::<i32>() - self.extents + self.origin,
                chunk.clone(),
            )
        })
        // std::iter::once(((Vec3::zero()), self.chunks[(1, 1, 1)].clone())).chain(std::iter::once((
        //     (Vec3::new(1, 0, 0)),
        //     self.chunks[(2, 1, 1)].clone(),
        // )))
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

    let chunk = &mut world.chunks[(0, 1, 0)];
    let mut new_blocks = Rc::unwrap_or_clone(Rc::clone(&chunk.blocks));
    new_blocks[(12, 4, 14)] = Block::GRASS;
    chunk.blocks = Rc::new(new_blocks);

    assert_eq!(world.get_block(Vec3::new(-4, 4, -2)), Some(Block::GRASS));

    assert_eq!(world.get_block(Vec3::new(-4, 4, -1)), Some(Block::AIR));
    world.set_block(Vec3::new(-4, 4, -1), Block::GRASS);
    assert_eq!(world.get_block(Vec3::new(-4, 4, -1)), Some(Block::GRASS));
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct RaycastOutput {
    pub position: Vec3<i32>,
    pub normal: Vec3<i8>,
}

impl DiscreteBlend for RaycastOutput {}

pub fn raycast_generalized<F: FnMut(Vec3<i32>) -> bool>(
    pos: Vec3<f32>,
    dir: Vec3<f32>,
    radius: f32,
    voxel_size: f32,
    mut has_voxel: F,
) -> Option<RaycastOutput> {
    if dir.normalized().magnitude() == 0.0 {
        return None;
    }

    let step = dir.map(|e| e.signum() as i32);
    let t_delta = (Vec3::broadcast(voxel_size) / dir).map(|e| e.abs());

    let ipos = pos.floor().map(|e| e as i32);
    if has_voxel(ipos) {
        return Some(RaycastOutput {
            position: ipos,
            normal: Vec3::zero(),
        });
    }

    let dist = step.zip(ipos).zip(pos).map(|((e_step, e_ipos), e_pos)| {
        if e_step > 0 {
            e_ipos as f32 + voxel_size - e_pos
        } else {
            e_pos - e_ipos as f32
        }
    });

    let mut grid_pos = ipos;
    let mut t_max = t_delta.zip(dist).map(|(e_delta, e_dist)| {
        if e_delta < f32::INFINITY {
            e_delta * e_dist
        } else {
            f32::INFINITY
        }
    });

    while pos.distance(grid_pos.map(|e| e as f32)) <= radius {
        let min_axis = t_max
            .into_iter()
            .enumerate()
            .min_by(|&a, &b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            .unwrap()
            .0;

        grid_pos[min_axis] += step[min_axis];
        t_max[min_axis] += t_delta[min_axis];

        if pos.distance(grid_pos.map(|e| e as f32)) > radius {
            break;
        }

        if has_voxel(grid_pos) {
            return Some(RaycastOutput {
                position: grid_pos,
                normal: {
                    let mut v = Vec3::zero();
                    v[min_axis] = -dir[min_axis].signum() as i8;
                    v
                },
            });
        }
    }

    None
}

pub fn raycast_candidates(pos: Vec3<f32>, dir: Vec3<f32>, radius: f32) -> Vec<Vec3<i32>> {
    let mut blocks = Vec::new();
    raycast_generalized(pos, dir, radius, 1.0, |grid_pos| {
        blocks.push(grid_pos);
        false
    });
    blocks
}

pub fn raycast(
    pos: Vec3<f32>,
    dir: Vec3<f32>,
    radius: f32,
    get_block: impl Fn(Vec3<i32>) -> Option<Block>,
) -> Option<RaycastOutput> {
    raycast_generalized(pos, dir, radius, 1.0, |grid_pos| {
        matches!(get_block(grid_pos), Some(Block { id: 1.., .. }))
    })
}

pub fn face_to_normal(face: u8) -> Vec3<i32> {
    match face {
        0 => Vec3::new(1, 0, 0),
        1 => Vec3::new(0, 1, 0),
        2 => Vec3::new(0, 0, 1),
        3 => Vec3::new(-1, 0, 0),
        4 => Vec3::new(0, -1, 0),
        5 => Vec3::new(0, 0, -1),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raycast_candidates() {
        assert_eq!(
            raycast_candidates(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(1.0, 0.4, 0.0),
                4.0
            ),
            vec![
                Vec3 { x: 8, y: 8, z: 0 },
                Vec3 { x: 9, y: 8, z: 0 },
                Vec3 { x: 10, y: 8, z: 0 },
                Vec3 { x: 10, y: 9, z: 0 },
                Vec3 { x: 11, y: 9, z: 0 }
            ]
        );
        assert_eq!(
            raycast_candidates(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(1.0, 0.0, 0.0),
                4.0
            ),
            vec![
                Vec3 { x: 8, y: 8, z: 0 },
                Vec3 { x: 9, y: 8, z: 0 },
                Vec3 { x: 10, y: 8, z: 0 },
                Vec3 { x: 11, y: 8, z: 0 },
                Vec3 { x: 12, y: 8, z: 0 }
            ]
        );

        assert_eq!(
            raycast_candidates(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(1.0, 0.0, 0.0),
                4.0
            ),
            vec![
                Vec3 { x: 8, y: 8, z: 0 },
                Vec3 { x: 9, y: 8, z: 0 },
                Vec3 { x: 10, y: 8, z: 0 },
                Vec3 { x: 11, y: 8, z: 0 },
                Vec3 { x: 12, y: 8, z: 0 }
            ]
        );

        assert_eq!(
            raycast_candidates(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(1.0, 0.0, 0.0),
                0.1
            ),
            vec![Vec3 { x: 8, y: 8, z: 0 }]
        );
    }

    #[test]
    fn test_raycast2() {
        let mut blocks: ndarray::Array3<Block> = ndarray::Array3::default((16, 16, 16));
        blocks[(9, 8, 0)] = Block::TEST;
        blocks[(9, 9, 0)] = Block::TEST;
        blocks[(9, 10, 0)] = Block::TEST;

        assert_eq!(
            raycast(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(1.0, 0.4, 0.0),
                16.0,
                |pos| if pos.into_iter().all(|e| e >= 0) {
                    blocks.get(pos.as_::<usize>().into_tuple()).cloned()
                } else {
                    None
                },
            ),
            Some(RaycastOutput {
                position: Vec3::new(9, 8, 0),
                normal: Vec3::new(-1, 0, 0),
            })
        );

        assert_eq!(
            raycast(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(1.0, 1.1, 0.0),
                16.0,
                |pos| if pos.into_iter().all(|e| e >= 0) {
                    blocks.get(pos.as_::<usize>().into_tuple()).cloned()
                } else {
                    None
                },
            ),
            Some(RaycastOutput {
                position: Vec3::new(9, 9, 0),
                normal: Vec3::new(-1, 0, 0),
            })
        );

        assert_eq!(
            raycast(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(0.0, 1.0, 0.0),
                16.0,
                |pos| if pos.into_iter().all(|e| e >= 0) {
                    blocks.get(pos.as_::<usize>().into_tuple()).cloned()
                } else {
                    None
                },
            ),
            None
        );

        assert_eq!(
            raycast(
                vek::Vec3::new(7.0, 7.0, 0.2),
                vek::Vec3::new(1.0, 1.5, 0.2),
                16.0,
                |pos| if pos.into_iter().all(|e| e >= 0) {
                    blocks.get(pos.as_::<usize>().into_tuple()).cloned()
                } else {
                    None
                },
            ),
            Some(RaycastOutput {
                position: Vec3::new(9, 9, 0),
                normal: Vec3::new(-1, 0, 0),
            })
        );

        assert_eq!(
            raycast(
                vek::Vec3::new(8.0, 8.0, -0.2),
                dbg!(vek::Vec3::new(1.0, 2.0, 0.2).normalized()),
                16.0,
                |pos| if pos.into_iter().all(|e| e >= 0) {
                    blocks.get(pos.as_::<usize>().into_tuple()).cloned()
                } else {
                    None
                },
            ),
            Some(RaycastOutput {
                position: Vec3::new(9, 10, 0),
                normal: Vec3::new(0, 0, -1),
            })
        );
    }

    #[test]
    fn test_raycast() {
        let mut blocks: Array3<Block> = Array3::default((16, 16, 16));
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    blocks[(x, y, z)] = Block::TEST;
                }
            }
        }

        fn look_at(yaw: f32, pitch: f32) -> Vec3<f32> {
            Vec3::new(
                yaw.sin() * pitch.cos(),
                -pitch.sin(),
                -yaw.cos() * pitch.cos(),
            )
        }

        // let Some(output) = raycast(
        //     Vec3::new(8.0, 18.0, 8.0),
        //     look_at(2.3, 1.2),
        //     16.0,
        //     blocks.view(),
        // ) else {
        //     panic!("Raycast was none");
        // };

        // assert_eq!(output.position, Vec3::new(8, 15, 8));
        // assert_eq!(output.face, Vec3::new(0, 1, 0));

        let Some(output) = raycast(
            Vec3::new(8.0, 18.0, 8.0),
            look_at(2.28, 1.14),
            16.0,
            |pos| {
                if pos.into_iter().all(|e| e >= 0) {
                    blocks.get(pos.as_::<usize>().into_tuple()).cloned()
                } else {
                    None
                }
            },
        ) else {
            panic!("Raycast was none");
        };

        assert_eq!(output.position, Vec3::new(8, 15, 8));
        assert_eq!(output.normal, Vec3::new(0, 1, 0));

        let Some(output) = raycast(Vec3::new(8.0, 18.0, 8.0), look_at(1.9, 0.9), 16.0, |pos| {
            if pos.into_iter().all(|e| e >= 0) {
                blocks.get(pos.as_::<usize>().into_tuple()).cloned()
            } else {
                None
            }
        }) else {
            panic!("Raycast was none");
        };

        assert_eq!(output.position, Vec3::new(9, 15, 8));
        assert_eq!(output.normal, Vec3::new(0, 1, 0));
    }
}
