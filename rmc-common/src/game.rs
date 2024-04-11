use crate::{
    camera::Angle,
    input::InputState,
    physics::{sweep_test, SweepBox, SweepTestResult},
    world::{raycast, Block, RaycastOutput},
    Blend, Camera,
};
use lazy_static::lazy_static;
use ndarray::Array3;
use sdl2::{keyboard::Keycode, mouse::MouseButton};
use std::rc::Rc;
use vek::{Aabb, Extent3, Vec3};

pub const TICK_RATE: u32 = 16;
pub const TICK_SPEED: f32 = 1.0;
pub const TICK_DELTA: f32 = 1.0 / TICK_RATE as f32;

const GRAVITY: f32 = 16.0;
const JUMP_HEIGHT: f32 = 1.0;
lazy_static! {
    // sqrt isn't const fn :/
    pub static ref JUMP_STRENGTH: f32 = 1.15 * (2.0 * GRAVITY * JUMP_HEIGHT - 1.0).sqrt();
}
const SPEED: f32 = 4.0;

const PLAYER_SIZE: Vec3<f32> = Vec3::new(0.2, 2.0, 0.2);
const PLAYER_ORIGIN: Vec3<f32> = Vec3::new(0.1, 1.5, 0.1);

#[derive(Clone)]
pub struct Game {
    pub blocks: Rc<Array3<Block>>,

    pub camera: Camera,
    pub velocity: Vec3<f32>,

    pub on_ground: bool,

    pub look_at_raycast: Option<RaycastOutput>,
}

impl Game {
    pub fn new() -> Self {
        let mut blocks: Array3<Block> = Array3::default((16, 16, 16));
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    blocks[(x, y, z)] = Block::GRASS;
                }
            }
        }

        // blocks[(7, 15, 8)] = Block::GRASS;
        // blocks[(8, 15, 8)] = Block::GRASS;

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
        let up_down = input.get_key(Keycode::Space).pressed() as i8
            - input.get_key(Keycode::LShift).pressed() as i8;
        let movement_vector = input.get_movement_vector();
        self.camera.position += (movement_vector.x * self.camera.right()
            + movement_vector.y * self.camera.forward())
        .try_normalized()
        .unwrap_or_default()
            * SPEED
            * TICK_DELTA;

        if self.on_ground {
            self.velocity.y = up_down as f32 * *JUMP_STRENGTH;
        }
    }

    // TODO use vek::Aabb
    fn handle_collision(&mut self, initial: &Game) {
        self.on_ground = false;

        const MAX_ITERATIONS: usize = 8;

        'iteration_loop: for _ in 0..MAX_ITERATIONS {
            let player_box_position = initial.camera.position - PLAYER_ORIGIN;
            let player_box = Aabb {
                min: player_box_position,
                max: player_box_position + PLAYER_SIZE,
            };

            let player_velocity = self.camera.position - initial.camera.position;

            let player_sweep = SweepBox {
                collider: Aabb {
                    min: player_box.min,
                    max: player_box.min + player_box.size(),
                },
                velocity: player_velocity,
            };

            let broad_box_position =
                player_box
                    .min
                    .zip(player_velocity)
                    .map(|(p, v)| if v > 0.0 { p } else { p + v });
            let broad_box = Aabb {
                min: broad_box_position,
                max: broad_box_position
                    + player_box
                        .size()
                        .zip(Extent3::<f32>::from(player_velocity))
                        .map(|(s, v)| s + v.abs()),
            };

            let mut collisions = Vec::new();

            for (idx, _block) in self
                .blocks
                .indexed_iter()
                .map(|(idx, block)| (idx, if block.id == 0 { None } else { Some(block) }))
                .filter_map(|(idx, block)| block.map(|b| (idx, b)))
                // WTF How does this improve the collision detection???
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                let block_box = Aabb {
                    min: Vec3::new(idx.0 as f32, idx.1 as f32, idx.2 as f32),
                    max: Vec3::new(idx.0 as f32, idx.1 as f32, idx.2 as f32) + Vec3::one(),
                };

                if broad_box.collides_with_aabb(block_box) {
                    if let Some(result) = sweep_test(player_sweep, block_box) {
                        collisions.push(result);
                    }
                }
            }

            let Some(SweepTestResult { normal, time }) = collisions
                .into_iter()
                .min_by(|a, b| a.time.partial_cmp(&b.time).unwrap())
            else {
                break 'iteration_loop;
            };

            self.camera.position = initial.camera.position + player_velocity * time;

            // Sliding
            let remaining_time = 1.0 - time;
            let remaining_velocity = player_velocity * remaining_time;
            let projected_velocity = remaining_velocity - remaining_velocity.dot(normal) * normal;
            self.camera.position += projected_velocity;

            if normal.y > 0.0 {
                self.on_ground = true;
            }
        }
    }

    fn modify_chunk(&mut self, f: impl FnOnce(&mut Array3<Block>) -> bool) {
        let mut blocks = Rc::<_>::unwrap_or_clone(self.blocks.clone());
        if f(&mut blocks) {
            self.blocks = Rc::new(blocks);
        }
    }

    pub fn set_block(&mut self, position: Vec3<i32>, block: Block) {
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
    }

    fn handle_place_destroy(&mut self, input: &InputState) {
        if let Some(highlighted) = self.look_at_raycast {
            if input.get_mouse_button(MouseButton::Left).just_pressed() {
                self.set_block(highlighted.position, Block::AIR);
            }

            if input.get_mouse_button(MouseButton::Right).just_pressed() {
                let position = highlighted.position + highlighted.normal.numcast().unwrap();

                self.set_block(position, Block::TEST);
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
