use ndarray::ArrayView3;
use vek::Vec3;

#[derive(Debug, Copy, Clone)]
pub struct Block {
    pub position: Vec3<i32>,
    pub id: u8,
}

#[derive(Debug, Copy, Clone)]
pub struct RaycastOutput {
    pub position: Vec3<i32>,
    pub face: Vec3<i8>,
}

pub fn raycast(
    pos: Vec3<f32>,
    dir: Vec3<f32>,
    radius: f32,
    blocks: ArrayView3<Option<Block>>,
) -> Option<RaycastOutput> {
    if let Some(&Some(block)) = blocks.get((pos.map(|e| e.floor() as _)).into_tuple()) {
        return Some(RaycastOutput {
            position: block.position,
            face: Vec3::zero(),
        });
    }

    const PRECISION: f32 = 0.01;

    let ndir = dir.normalized();
    for t in 0.. {
        let offset = Vec3::broadcast(t as f32 * PRECISION) * ndir;
        if offset.magnitude() > radius {
            break;
        }
        let target = pos + offset;
        if let Some(&Some(block)) = blocks.get((target.map(|e| e.floor() as _)).into_tuple()) {
            let point_before_inside = pos + Vec3::broadcast((t - 1) as f32 * PRECISION) * ndir;
            let diff = point_before_inside.map(|e| e.floor() as i32) - block.position;
            if diff.sum() <= 1 {
                return Some(RaycastOutput {
                    position: block.position,
                    face: diff.map(|e| e as i8),
                });
            }

            let new_diff = if diff.z > 0 {
                Vec3::new(diff.x.max(1), diff.y.max(1), 0)
            } else if diff.y > 0 {
                Vec3::new(diff.x.max(1), 0, diff.z.max(1))
            } else {
                unreachable!()
            };

            return Some(RaycastOutput {
                position: block.position,
                face: new_diff.map(|e| e as i8),
            });
        }
    }

    return None;
}
