use crate::world::{face_neighbors, Block, World};
use itertools::Itertools;
use vek::Vec3;

pub fn calculate_block_light(
    world: &World,
    position: Vec3<i32>,
    block: Block,
    source: Option<Vec3<i32>>,
) -> u8 {
    if block.ty.light_passing() && block.open_to_sky {
        return 255;
    }

    if let Some(emission) = block.ty.light_emission() {
        emission
    } else if block.ty.light_passing() {
        let all_neighbors = face_neighbors(position)
            .into_iter()
            .map(|position| (position, world.get_block(position)))
            .filter_map(|(p, b)| b.map(|b| (p, b)))
            .collect_vec();

        calculate_light((position, block), all_neighbors, source)
    } else {
        0
    }
}

fn calculate_light(
    (position, block): (Vec3<i32>, Block),
    checks: impl IntoIterator<Item = (Vec3<i32>, Block)>,
    source: Option<Vec3<i32>>,
) -> u8 {
    checks
        .into_iter()
        .map(|(p, b)| calculate_light_from((position, block), (p, b), source))
        .max()
        .unwrap_or(0)
}

fn calculate_light_from(
    (position, block): (Vec3<i32>, Block),
    (p, b): (Vec3<i32>, Block),
    source: Option<Vec3<i32>>,
) -> u8 {
    let distance = position.as_::<f32>().distance(p.as_::<f32>());
    assert!(distance <= 2.0);
    let new_light = b.light.checked_sub((16.0 * distance) as u8).unwrap_or(0);
    if new_light < block.light && Some(p) == source {
        return 0;
    }

    new_light
}
