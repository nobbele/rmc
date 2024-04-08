use std::cmp::Ordering;

use ndarray::ArrayView3;
use vek::Vec3;

use crate::DiscreteBlend;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Block {
    pub id: u8,
}

impl DiscreteBlend for Block {}

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
    blocks: ArrayView3<Option<Block>>,
) -> Option<RaycastOutput> {
    raycast_generalized(pos, dir, radius, 1.0, |grid_pos| {
        matches!(
            blocks.get(grid_pos.map(|e| e as _).into_tuple()),
            Some(Some(_))
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

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
        let mut blocks: ndarray::Array3<Option<Block>> = ndarray::Array3::default((16, 16, 16));
        blocks[(9, 8, 0)] = Some(Block { id: 0 });
        blocks[(9, 9, 0)] = Some(Block { id: 0 });
        blocks[(9, 10, 0)] = Some(Block { id: 0 });

        assert_eq!(
            raycast(
                vek::Vec3::new(8.0, 8.0, 0.0),
                vek::Vec3::new(1.0, 0.4, 0.0),
                16.0,
                blocks.view(),
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
                blocks.view(),
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
                blocks.view(),
            ),
            None
        );

        assert_eq!(
            raycast(
                vek::Vec3::new(7.0, 7.0, 0.2),
                vek::Vec3::new(1.0, 1.5, 0.2),
                16.0,
                blocks.view(),
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
                blocks.view(),
            ),
            Some(RaycastOutput {
                position: Vec3::new(9, 10, 0),
                normal: Vec3::new(0, 0, -1),
            })
        );
    }

    #[test]
    fn test_raycast() {
        let mut blocks: Array3<Option<Block>> = Array3::default((16, 16, 16));
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    blocks[(x, y, z)] = Some(Block { id: 1 });
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
            blocks.view(),
        ) else {
            panic!("Raycast was none");
        };

        assert_eq!(output.position, Vec3::new(8, 15, 8));
        assert_eq!(output.normal, Vec3::new(0, 1, 0));

        let Some(output) = raycast(
            Vec3::new(8.0, 18.0, 8.0),
            look_at(1.9, 0.9),
            16.0,
            blocks.view(),
        ) else {
            panic!("Raycast was none");
        };

        assert_eq!(output.position, Vec3::new(9, 15, 8));
        assert_eq!(output.normal, Vec3::new(0, 1, 0));
    }
}
