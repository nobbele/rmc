use vek::Vec3;

#[derive(Debug, Copy, Clone)]
pub struct Block {
    pub position: Vec3<i32>,
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
    blocks: &[Block],
) -> Option<RaycastOutput> {
    if let Some(block) = blocks
        .iter()
        .find(|b| b.position == pos.map(|e| e.floor() as i32))
    {
        return Some(RaycastOutput {
            position: block.position,
            face: Vec3::zero(),
        });
    }

    let ndir = dir.normalized();
    for t in 0.. {
        let offset = Vec3::broadcast(t as f32 / 10.) * ndir;
        if offset.magnitude() > radius {
            break;
        }
        let target = pos + offset;
        if let Some(block) = blocks
            .iter()
            .find(|b| b.position == target.map(|e| e.floor() as i32))
        {
            let point_before_inside = pos + Vec3::broadcast((t - 1) as f32 / 10.) * ndir;
            let block_position_before = point_before_inside.map(|e| e.floor() as i32);
            let diff = block_position_before - block.position;
            if diff.sum() <= 1 {
                return Some(RaycastOutput {
                    position: block.position,
                    face: diff.map(|e| e as i8),
                });
            }

            let new_diff = if diff.z == 1 {
                Vec3::new(diff.x, diff.y, 0)
            } else if diff.y == 1 {
                Vec3::new(diff.x, 0, diff.z)
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
