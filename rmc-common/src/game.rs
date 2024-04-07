use crate::{
    camera::Angle,
    input::InputState,
    world::{raycast, Block, RaycastOutput},
    Blend, Camera,
};
use lazy_static::lazy_static;
use ndarray::Array3;
use sdl2::{keyboard::Keycode, mouse::MouseButton};
use std::rc::Rc;
use vek::Vec3;

pub const TICK_RATE: u32 = 32;
pub const TICK_SPEED: f32 = 1.0;
pub const TICK_DELTA: f32 = 1.0 / TICK_RATE as f32;

const GRAVITY: f32 = 16.0;
const JUMP_HEIGHT: f32 = 1.0;
lazy_static! {
    // sqrt isn't const fn :/
    pub static ref JUMP_STRENGTH: f32 = 1.2 * (2.0 * GRAVITY * JUMP_HEIGHT - 1.0).sqrt();
}
const SPEED: f32 = 4.0;

const PLAYER_SIZE: Vec3<f32> = Vec3::new(0.2, 2.0, 0.2);
const PLAYER_ORIGIN: Vec3<f32> = Vec3::new(0.1, 1.5, 0.1);

#[derive(Clone)]
pub struct Game {
    pub blocks: Rc<Array3<Option<Block>>>,

    pub camera: Camera,
    pub velocity: Vec3<f32>,

    pub on_ground: bool,

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

        // blocks[(8, 15, 8)] = Some(Block { id: 1 });

        Game {
            blocks: Rc::new(blocks),

            camera: Camera {
                position: Vec3::new(8.5, 18.0, 8.5),
                pitch: Angle(0.0),
                yaw: Angle(0.0),
            },
            velocity: Vec3::zero(),

            on_ground: false,

            look_at_raycast: None,
        }
    }

    pub fn update(&mut self, input: &InputState) {
        let initial = self.clone();

        self.handle_camera_movement(input);
        self.handle_movement(input);

        self.velocity.y -= GRAVITY * TICK_DELTA;
        self.camera.position += self.velocity * TICK_DELTA;

        self.handle_collision(&initial);

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

        if self.on_ground {
            self.velocity.y = up_down as f32 * *JUMP_STRENGTH;
        }
    }

    // TODO Still not great!!
    // Works much better in positive direction than negative for some reason?
    fn handle_collision(&mut self, initial: &Game) {
        self.on_ground = false;

        let player_box = AABB {
            position: initial.camera.position - PLAYER_ORIGIN,
            size: PLAYER_SIZE,
        };

        for (idx, _block) in self
            .blocks
            .indexed_iter()
            .filter_map(|(idx, block)| block.map(|b| (idx, b)))
        {
            if self.blocks.get(idx).is_none() {
                continue;
            }

            let block_box = AABB {
                position: Vec3::new(idx.0 as f32, idx.1 as f32, idx.2 as f32),
                size: Vec3::one(),
            };

            if player_box.intersects(block_box.scaled(1.0 - f32::EPSILON)) {
                panic!("Camera cannot be inside a block");
            }

            let camera_velocity = self.camera.position - initial.camera.position;

            if let Some(SweepTestResult { normal, time }) = sweep_test(
                SweepBox {
                    position: player_box.position,
                    size: player_box.size,
                    velocity: camera_velocity,
                },
                block_box,
            ) {
                self.camera.position = initial.camera.position + camera_velocity * time;

                // Sliding
                let remaining_time = 1.0 - time;
                let remaining_velocity = camera_velocity * remaining_time;
                let projected_velocity =
                    remaining_velocity - remaining_velocity.dot(normal) * normal;
                self.camera.position += projected_velocity;

                if normal.y > 0.0 {
                    self.on_ground = true;
                }
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

            on_ground: self.on_ground.blend(&other.on_ground, alpha),

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub position: Vec3<f32>,
    pub size: Vec3<f32>,
}

impl AABB {
    pub fn scaled(self, s: f32) -> Self {
        AABB {
            position: self.position + self.size * (1.0 - s),
            size: self.size * s,
        }
    }

    pub fn min_x(self) -> f32 {
        self.position.x
    }

    pub fn min_y(self) -> f32 {
        self.position.y
    }

    pub fn min_z(self) -> f32 {
        self.position.z
    }

    pub fn max_x(self) -> f32 {
        self.min_x() + self.size.x
    }

    pub fn max_y(self) -> f32 {
        self.min_y() + self.size.y
    }

    pub fn max_z(self) -> f32 {
        self.min_z() + self.size.z
    }

    pub fn intersects(self, other: AABB) -> bool {
        (self.max_x() > other.min_x()
            && self.max_y() > other.min_y()
            && self.max_z() > other.min_z())
            && (self.min_x() < other.max_x()
                && self.min_y() < other.max_y()
                && self.min_z() < other.max_z())
    }
}

// https://www.gamedev.net/tutorials/programming/general-and-gameplay-programming/swept-aabb-collision-detection-and-response-r3084/
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepBox {
    pub position: Vec3<f32>,
    pub size: Vec3<f32>,
    pub velocity: Vec3<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
// A normal of zero and travel of zero means the movement is already perfectly aligned.
pub struct SweepTestResult {
    pub normal: Vec3<f32>,
    pub time: f32,
}

pub fn sweep_test(a: SweepBox, b: AABB) -> Option<SweepTestResult> {
    fn calc_axis_abs(a_min: f32, a_max: f32, b_min: f32, b_max: f32, direction: f32) -> (f32, f32) {
        if direction > 0.0 {
            (b_min - a_max, b_max - a_min)
        } else {
            (b_max - a_min, b_min - a_max)
        }
    }
    fn calc_axis_abs_aabb(a: SweepBox, b: AABB, velocity: Vec3<f32>, axis: usize) -> (f32, f32) {
        calc_axis_abs(
            a.position[axis],
            a.position[axis] + a.size[axis],
            b.position[axis],
            b.position[axis] + b.size[axis],
            velocity[axis],
        )
    }
    fn calc_axis_rel(enter: f32, exit: f32, velocity: f32) -> (f32, f32) {
        if velocity == 0.0 {
            (-f32::INFINITY, f32::INFINITY)
        } else {
            (enter / velocity, exit / velocity)
        }
    }
    let (x_abs_enter, x_abs_exit) = calc_axis_abs_aabb(a, b, a.velocity, 0);
    let (y_abs_enter, y_abs_exit) = calc_axis_abs_aabb(a, b, a.velocity, 1);
    let (z_abs_enter, z_abs_exit) = calc_axis_abs_aabb(a, b, a.velocity, 2);

    let (x_enter, x_exit) = calc_axis_rel(x_abs_enter, x_abs_exit, a.velocity.x);
    let (y_enter, y_exit) = calc_axis_rel(y_abs_enter, y_abs_exit, a.velocity.y);
    let (z_enter, z_exit) = calc_axis_rel(z_abs_enter, z_abs_exit, a.velocity.z);

    let entry_time = x_enter.max(y_enter).max(z_enter);
    let exit_time = x_exit.min(y_exit).min(z_exit);

    if entry_time > exit_time
        || (x_enter < 0.0 && y_enter < 0.0 && z_enter < 0.0)
        || x_enter > 1.0
        || y_enter > 1.0
        || z_enter > 1.0
    {
        return None;
    }

    fn norm(v: f32, vel: f32) -> f32 {
        if v != 0.0 {
            -v.signum()
        } else {
            -vel.signum()
        }
    }

    let x_norm = norm(x_abs_enter, a.velocity.x);
    let y_norm = norm(y_abs_enter, a.velocity.y);
    let z_norm = norm(z_abs_enter, a.velocity.z);

    let normal = if x_enter > y_enter && x_enter > z_enter {
        Vec3::new(x_norm, 0.0, 0.0)
    } else if y_enter > x_enter && y_enter > z_enter {
        Vec3::new(0.0, y_norm, 0.0)
    } else if z_enter > x_enter && z_enter > y_enter {
        Vec3::new(0.0, 0.0, z_norm)
    }
    // Edges and corners
    else {
        // x,y edge
        if x_enter > z_enter && y_enter > z_enter {
            Vec3::new(x_norm, y_norm, 0.0).normalized()
        }
        // x,z edge
        else if x_enter > y_enter && z_enter > y_enter {
            Vec3::new(x_norm, 0.0, z_norm).normalized()
        }
        // y,z edge
        else if y_enter > x_enter && z_enter > x_enter {
            Vec3::new(0.0, y_norm, z_norm).normalized()
        }
        // x,y,z corner
        else {
            Vec3::new(x_norm, y_norm, z_norm).normalized()
        }
    };

    Some(SweepTestResult {
        normal,
        time: entry_time,
    })
}

#[test]
pub fn test_sweep_test() {
    assert_eq!(
        sweep_test(
            SweepBox {
                position: Vec3::zero(),
                size: Vec3::one(),
                velocity: Vec3::new(0.5, 0.0, 0.0),
            },
            AABB {
                position: Vec3::new(2.0, 2.0, 0.0),
                size: Vec3::one()
            }
        ),
        None
    );
    assert_eq!(
        sweep_test(
            SweepBox {
                position: Vec3::zero(),
                size: Vec3::one(),
                velocity: Vec3::new(1.2, 0.0, 0.0),
            },
            AABB {
                position: Vec3::new(2.0, 2.0, 0.0),
                size: Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3::new(-1.0, 0.0, 0.0),
            time: 0.8333333,
        })
    );

    assert_eq!(
        sweep_test(
            SweepBox {
                position: Vec3::zero(),
                size: Vec3::one(),
                velocity: Vec3::new(1.2, 1.3, 0.0),
            },
            AABB {
                position: Vec3::new(2.0, 2.0, 0.0),
                size: Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: -1.0,
                y: 0.0,
                z: 0.0
            },
            time: 0.8333333
        })
    );

    assert_eq!(
        sweep_test(
            SweepBox {
                position: Vec3::zero(),
                size: Vec3::one(),
                velocity: Vec3::new(1.2, 1.2, 0.0),
            },
            AABB {
                position: Vec3::new(2.0, 2.0, 0.0),
                size: Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: -0.70710677,
                y: -0.70710677,
                z: 0.0
            },
            time: 0.8333333
        })
    );

    assert_eq!(
        sweep_test(
            SweepBox {
                position: Vec3::new(0.5, 1.5, 0.5),
                size: Vec3::one(),
                velocity: Vec3::new(0.0, -0.1, 0.0),
            },
            AABB {
                position: Vec3::one(),
                size: Vec3::one()
            }
        ),
        None
    );
    assert_eq!(
        sweep_test(
            SweepBox {
                position: Vec3::new(0.5, 1.1, 0.5),
                size: Vec3::one(),
                velocity: Vec3::new(0.0, -0.2, 0.0),
            },
            AABB {
                position: Vec3::zero(),
                size: Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0
            },
            time: 0.5000001
        })
    );
    assert_eq!(
        sweep_test(
            SweepBox {
                position: Vec3::new(0.5, 2.1, 0.5),
                size: Vec3::one(),
                velocity: Vec3::new(0.0, -0.2, 0.0),
            },
            AABB {
                position: Vec3::one(),
                size: Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0
            },
            time: 0.49999952
        })
    );
    assert_eq!(
        dbg!(sweep_test(
            SweepBox {
                position: Vec3::new(0.5, 1.1, 0.5),
                size: Vec3::one(),
                velocity: Vec3::new(0.0, -0.4, 0.0),
            },
            AABB {
                position: Vec3::zero(),
                size: Vec3::one()
            }
        )),
        Some(SweepTestResult {
            normal: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0
            },
            time: 0.25000006
        })
    );
}
