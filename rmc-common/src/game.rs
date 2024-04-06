use std::{cmp::Ordering, rc::Rc};

use crate::{
    camera::Angle,
    input::InputState,
    world::{raycast, Block, RaycastOutput},
    Blend, Camera,
};
use ndarray::Array3;
use sdl2::{keyboard::Keycode, mouse::MouseButton};
use vek::Vec3;

pub const TICK_RATE: u32 = 32;
pub const TICK_DELTA: f32 = 1.0 / TICK_RATE as f32;

const GRAVITY: f32 = 8.0;
const SPEED: f32 = 4.0;

#[derive(Clone)]
pub struct Game {
    pub blocks: Rc<Array3<Option<Block>>>,

    pub camera: Camera,
    pub velocity: Vec3<f32>,

    pub look_at_raycast: Option<RaycastOutput>,
}

impl Game {
    pub fn new() -> Self {
        let mut blocks: Array3<Option<Block>> = Array3::default((16, 16, 16));
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    blocks[(x, y, z)] = Some(Block { id: 1 });
                }
            }
        }

        // blocks[(8, 8, 8)] = Some(Block { id: 1 });

        Game {
            blocks: Rc::new(blocks),

            camera: Camera {
                position: Vec3::new(8.0, 18.0, 8.0),
                pitch: Angle(0.0),
                yaw: Angle(0.0),
            },
            velocity: Vec3::zero(),

            look_at_raycast: None,
        }
    }

    pub fn update(&mut self, input: &InputState) {
        // let initial = self.clone();

        self.handle_camera_movement(input);
        self.handle_movement(input);

        self.velocity.y -= GRAVITY * TICK_DELTA;
        self.camera.position += self.velocity * TICK_DELTA;

        self.handle_collision();

        self.look_at_raycast = raycast(
            self.camera.position,
            self.camera.look_at(),
            7.5,
            self.blocks.view(),
        );

        self.handle_place_destroy(input);
    }

    fn handle_camera_movement(&mut self, input: &InputState) {
        self.camera.rotate_horizontal(input.mouse_delta.x);
        self.camera.rotate_vertical(input.mouse_delta.y);
    }

    fn handle_movement(&mut self, input: &InputState) {
        let fwd_bck =
            input.get_key(Keycode::W).pressed() as i8 - input.get_key(Keycode::S).pressed() as i8;
        let rgh_lft =
            input.get_key(Keycode::D).pressed() as i8 - input.get_key(Keycode::A).pressed() as i8;
        let up_down = input.get_key(Keycode::Space).pressed() as i8
            - input.get_key(Keycode::LShift).pressed() as i8;
        self.camera
            .move_forward(fwd_bck as f32 * SPEED * TICK_DELTA);
        self.camera.move_right(rgh_lft as f32 * SPEED * TICK_DELTA);
        self.camera.move_up(up_down as f32 * SPEED * TICK_DELTA);
    }

    // TODO Still not perfect!!!
    fn handle_collision(&mut self) {
        const MAX_COLLISIONS_TESTS_PER_FRAME: usize = 4;
        'retry_loop: for _ in 0..MAX_COLLISIONS_TESTS_PER_FRAME {
            let mut collided = false;

            'block_loop: for (idx, _block) in self
                .blocks
                .indexed_iter()
                .filter_map(|(idx, block)| block.map(|b| (idx, b)))
            {
                if self.blocks.get(idx).is_none() {
                    continue;
                }

                let camera_box = BoundingBox {
                    position: self.camera.position + Vec3::new(-0.1, -1.5, -0.1),
                    size: Vec3::new(0.2, 2.0, 0.2),
                };

                if let Some(mtv) = sat_test(
                    camera_box,
                    BoundingBox {
                        position: Vec3::new(idx.0 as f32, idx.1 as f32, idx.2 as f32),
                        size: Vec3::one(),
                    },
                ) {
                    println!("mtv: {}", mtv);
                    self.camera.position += mtv;
                    self.velocity = Vec3::zero();

                    collided = true;
                    break 'block_loop;
                }
            }

            // If we never collided with anything this iteration, there's no need to check for collisions anymore this frame.
            if !collided {
                break 'retry_loop;
            }
        }
    }

    fn modify_chunk(&mut self, f: impl FnOnce(&mut Array3<Option<Block>>) -> bool) {
        let mut blocks = Rc::<_>::unwrap_or_clone(self.blocks.clone());
        if f(&mut blocks) {
            self.blocks = Rc::new(blocks);
        }
    }

    pub fn set_block(&mut self, position: Vec3<i32>, block: Option<Block>) {
        self.modify_chunk(|chunk| {
            if let Some(entry) = chunk.get_mut(position.map(|e| e as _).into_tuple()) {
                *entry = block;
                return true;
            }
            return false;
        });
    }

    pub fn get_block(&mut self, position: Vec3<i32>) -> Option<Block> {
        // TODO chunks
        if !position.iter().all(|e| *e >= 0) {
            return None;
        }

        self.blocks
            .get(position.map(|e| e as usize).into_tuple())
            .cloned()
            .flatten()
    }

    fn handle_place_destroy(&mut self, input: &InputState) {
        if let Some(highlighted) = self.look_at_raycast {
            if input.get_mouse_button(MouseButton::Left).just_pressed() {
                self.set_block(highlighted.position, None);
            }

            if input.get_mouse_button(MouseButton::Right).just_pressed() {
                let position = highlighted.position + highlighted.normal.numcast().unwrap();

                self.set_block(position, Some(Block { id: 0 }));
            }
        }
    }
}

impl Blend for Game {
    fn blend(&self, other: &Game, alpha: f32) -> Self {
        Self {
            blocks: self.blocks.blend(&other.blocks, alpha),

            camera: self.camera.blend(&other.camera, alpha),
            velocity: self.velocity.blend(&other.velocity, alpha),

            look_at_raycast: self.look_at_raycast.blend(&other.look_at_raycast, alpha),
        }
    }
}

#[test]
pub fn test_game_state_size() {
    // The size of the game state should not grow too large due to frequent use of cloning during updates and blending.
    const MAX_SIZE: usize = 256;

    assert!(
        std::mem::size_of::<Game>() < MAX_SIZE,
        "Size of `Game` ({} bytes) needs to be smaller than {} bytes",
        std::mem::size_of::<Game>(),
        MAX_SIZE
    );
}

#[derive(Debug, Clone, Copy)]
struct BoundingBox {
    position: Vec3<f32>,
    size: Vec3<f32>,
}

/// Returns the minimum translation vector between two bounding boxes.
fn sat_test(a: BoundingBox, b: BoundingBox) -> Option<Vec3<f32>> {
    let Some(sat_x) = sat_axis_test_index(0, a, b) else {
        return None;
    };
    let Some(sat_y) = sat_axis_test_index(1, a, b) else {
        return None;
    };
    let Some(sat_z) = sat_axis_test_index(2, a, b) else {
        return None;
    };

    // println!("{:#?} {:#?}", a, b);
    // println!("{:?}", [sat_x, sat_y, sat_z]);

    let min_sat = [sat_x, sat_y, sat_z]
        .into_iter()
        .min_by(|a, b| {
            a.magnitude_squared()
                .partial_cmp(&b.magnitude_squared())
                .unwrap_or(Ordering::Equal)
        })
        .unwrap();

    Some(min_sat)
}

fn sat_axis_test_index(axis_idx: usize, a: BoundingBox, b: BoundingBox) -> Option<Vec3<f32>> {
    sat_axis_test(
        if axis_idx == 0 {
            Vec3::unit_x()
        } else if axis_idx == 1 {
            Vec3::unit_y()
        } else if axis_idx == 2 {
            Vec3::unit_z()
        } else {
            panic!()
        },
        a.position[axis_idx],
        a.position[axis_idx] + a.size[axis_idx],
        b.position[axis_idx],
        b.position[axis_idx] + b.size[axis_idx],
    )
}

/// Implements SAT (Seperating Axis Theorem)
fn sat_axis_test(
    axis: Vec3<f32>,
    min_a: f32,
    max_a: f32,
    min_b: f32,
    max_b: f32,
) -> Option<Vec3<f32>> {
    let axis_mag = axis.magnitude();
    if axis_mag < 0.01 {
        return None;
    }

    let d0 = max_b - min_a;
    let d1 = max_a - min_b;

    if d0 <= 0.0 || d1 <= 0.0 {
        return None;
    }

    let overlap = if d0 < d1 { d0 } else { -d1 };

    Some(axis * (overlap / axis_mag))
}

#[test]
pub fn test_sat_test() {
    assert_eq!(
        sat_test(
            BoundingBox {
                position: Vec3::zero(),
                size: Vec3::one()
            },
            BoundingBox {
                position: Vec3::one() * 2.0,
                size: Vec3::one()
            }
        ),
        None
    );

    assert_eq!(
        sat_test(
            BoundingBox {
                position: Vec3::zero(),
                size: Vec3::one()
            },
            BoundingBox {
                position: Vec3::new(0.0, 0.8, 0.0),
                size: Vec3::one()
            }
        ),
        Some(Vec3 {
            x: -0.0,
            y: -0.19999999,
            z: -0.0
        })
    );

    assert_eq!(
        sat_test(
            BoundingBox {
                position: Vec3::zero(),
                size: Vec3::one()
            },
            BoundingBox {
                position: Vec3::new(0.8, 0.8, 0.0),
                size: Vec3::one()
            }
        ),
        Some(Vec3 {
            x: -0.19999999,
            y: -0.0,
            z: -0.0
        })
    );
}
