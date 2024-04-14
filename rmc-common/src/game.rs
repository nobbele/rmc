use crate::{
    camera::Angle,
    input::InputState,
    physics::{sweep_test, SweepBox, SweepTestResult},
    world::{face_to_normal, raycast, Block, RaycastOutput},
    Blend, Camera,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use ndarray::Array3;
use sdl2::{keyboard::Keycode, mouse::MouseButton};
use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};
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

#[derive(Debug, Clone)]
pub struct Game {
    pub blocks: Rc<Array3<Block>>,

    pub camera: Camera,
    pub velocity: Vec3<f32>,

    pub on_ground: bool,
    pub look_at_raycast: Option<RaycastOutput>,

    pub dirty_blocks: VecDeque<Vec3<i32>>,
    // This is per frame
    pub block_update_count: usize,
}

impl Game {
    pub fn new() -> Self {
        let mut blocks: Array3<Block> = Array3::default((16, 16, 16));
        for y in 8..12 {
            for z in 2..14 {
                for x in 2..14 {
                    blocks[(x, y, z)] = Block::GRASS;
                }
            }
        }

        blocks[(6, 11, 7)] = Block::AIR;
        blocks[(6, 11, 9)] = Block::AIR;
        blocks[(7, 11, 8)] = Block::AIR;
        blocks[(5, 11, 8)] = Block::AIR;
        blocks[(7, 11, 7)] = Block::AIR;
        blocks[(7, 11, 9)] = Block::AIR;
        blocks[(5, 11, 7)] = Block::AIR;
        blocks[(5, 11, 9)] = Block::AIR;

        blocks[(6, 10, 8)] = Block::AIR;
        blocks[(6, 10, 7)] = Block::AIR;
        blocks[(6, 10, 9)] = Block::AIR;
        blocks[(7, 10, 8)] = Block::AIR;
        blocks[(5, 10, 8)] = Block::AIR;
        blocks[(7, 10, 7)] = Block::AIR;
        blocks[(7, 10, 9)] = Block::AIR;
        blocks[(5, 10, 7)] = Block::AIR;
        blocks[(5, 10, 9)] = Block::AIR;

        let mut game = Game {
            blocks: Rc::new(blocks),

            camera: Camera {
                position: Vec3::new(8.5, 18.0, 8.5),
                pitch: Angle(0.0),
                yaw: Angle(0.0),
            },
            velocity: Vec3::zero(),

            on_ground: false,

            look_at_raycast: None,
            dirty_blocks: VecDeque::new(),
            block_update_count: 0,
        };

        game.set_block(Vec3::new(6, 11, 8), Block::LANTERN);

        game
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
        // self.update_lighting();
        self.update_blocks();
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
                .collect_vec()
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

    fn update_blocks(&mut self) {
        const MAX_UPDATES_COUNT: usize = 8192;

        self.block_update_count = 0;

        while self.block_update_count < MAX_UPDATES_COUNT && self.dirty_blocks.len() != 0 {
            let update_count = self.dirty_blocks.len().min(MAX_UPDATES_COUNT);
            self.block_update_count += update_count;

            let dirty_blocks = self.dirty_blocks.drain(..update_count).collect_vec();

            // println!(
            //     "----------- ({:03} / {:03}) -----------",
            //     dirty_blocks.len(),
            //     dirty_blocks.len() + self.dirty_blocks.len()
            // );

            let mut replaces = HashMap::new();
            for position in dirty_blocks {
                let Some(block) = self.get_block(position) else {
                    continue;
                };

                if replaces.contains_key(&position) {
                    continue;
                }

                const INCLUDE_DIAGONAL: bool = true;

                let neighbor_positions =
                    [0, 1, 2, 3, 4, 5].map(|face| position + face_to_normal(face));
                let neighbors = if INCLUDE_DIAGONAL {
                    neighbor_positions
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
                        .map(|position| (position, self.get_block(position)))
                        .collect_vec()
                } else {
                    neighbor_positions
                        .map(|position| (position, self.get_block(position)))
                        .to_vec()
                };

                let light = match () {
                    _ if block.id == Block::LANTERN.id => 255,
                    _ if block.id == Block::AIR.id => neighbors
                        .iter()
                        .map(|&(p, b)| {
                            b.map(|b| {
                                let distance = position.as_::<f32>().distance(p.as_::<f32>());
                                assert!(distance <= 2.0);
                                b.light.checked_sub((16.0 * distance) as u8)
                            })
                            .flatten()
                            .unwrap_or(0)
                        })
                        .max()
                        .unwrap_or(0),
                    _ => 0,
                };

                let new_block = Block { light, ..block };

                if block != new_block {
                    self.dirty_blocks
                        .extend(neighbors.into_iter().map(|(p, _b)| p));
                    replaces.insert(position, new_block);
                }
            }
            self.replace_blocks1(replaces.into_iter(), false);
        }
    }

    pub fn replace_blocks(&mut self, replaces: impl Iterator<Item = (Vec3<i32>, Block)>) {
        self.replace_blocks1(replaces, true);
    }

    pub fn replace_blocks1(
        &mut self,
        replaces: impl Iterator<Item = (Vec3<i32>, Block)>,
        update: bool,
    ) {
        if replaces.size_hint().1 == Some(0) {
            return;
        }

        let mut blocks = Rc::unwrap_or_clone(Rc::clone(&self.blocks));
        for (position, block) in replaces {
            if position.into_iter().all(|e| e >= 0) {
                if let Some(target) = blocks.get_mut(position.map(|e| e as _).into_tuple()) {
                    if *target != block {
                        if update {
                            self.dirty_blocks.push_back(position);
                        }
                        *target = block;
                    }
                }
            }
        }
        self.blocks = Rc::new(blocks);
    }

    pub fn set_block(&mut self, position: Vec3<i32>, block: Block) {
        self.set_block1(position, block, true);
    }

    pub fn set_block1(&mut self, position: Vec3<i32>, block: Block, update: bool) {
        self.replace_blocks1([(position, block)].into_iter(), update);
    }

    pub fn get_block(&self, position: Vec3<i32>) -> Option<Block> {
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
            dirty_blocks: self.dirty_blocks.blend(&other.dirty_blocks, alpha),
            block_update_count: self
                .block_update_count
                .blend(&other.block_update_count, alpha),
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
