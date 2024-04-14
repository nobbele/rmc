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
use vek::{num_integer::Roots, Aabb, Extent3, Vec3};

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
        for y in 8..12 {
            for z in 2..14 {
                for x in 2..14 {
                    blocks[(x, y, z)] = Block::GRASS;
                }
            }
        }

        blocks[(6, 11, 8)] = Block::LANTERN;
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
        self.update_lighting();
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

    fn update_lighting(&mut self) {
        fn cast_(utup: (usize, usize, usize)) -> Vec3<i32> {
            Vec3::new(utup.0 as i32, utup.1 as i32, utup.2 as i32)
        }

        // Light sources
        {
            let light_sources = self
                .blocks
                .indexed_iter()
                .map(|(idx, block)| (cast_(idx), *block))
                .filter(|(_idx, block)| block.id == Block::LANTERN.id)
                .collect::<Vec<_>>();

            let blocks = self
                .blocks
                .indexed_iter()
                .map(|(idx, block)| (cast_(idx), *block))
                .collect::<Vec<_>>();

            let mut new_chunk = None;
            for (light_source_pos, _light_source_block) in light_sources {
                let strength = 10;

                for (pos, _b) in blocks.iter().cloned() {
                    let distance = light_source_pos.distance_squared(pos);
                    if distance > (strength as i32).pow(2) {
                        continue;
                    }

                    // match raycast(
                    //     pos.as_(),
                    //     (light_source_pos.as_::<f32>() - pos.as_::<f32>())
                    //         .try_normalized()
                    //         .unwrap_or_default(),
                    //     strength as f32,
                    //     self.blocks.view(),
                    // ) {
                    //     None => continue,
                    //     Some(raycast_output) => {
                    //         if pos != raycast_output.position {
                    //             continue;
                    //         }
                    //     }
                    // }

                    if let Some(entry) = self.get_block(pos) {
                        let light = strength - (distance.sqrt() as u8);
                        if entry.light != light {
                            let chunk = new_chunk.get_or_insert_with(|| {
                                Rc::unwrap_or_clone(Rc::clone(&self.blocks))
                            });
                            chunk[pos.map(|e| e as _).into_tuple()].light = light;
                        }
                    }
                }
            }

            if let Some(new_blocks) = new_chunk {
                self.blocks = Rc::new(new_blocks);
            }
        }

        // Light from sky
        // {
        //     let blocks = self
        //         .blocks
        //         .indexed_iter()
        //         .map(|(idx, block)| (cast_(idx), *block))
        //         .collect::<Vec<_>>();

        //     let mut new_chunk = None;
        //     for (block_pos, block) in blocks.iter().filter(|(_, b)| b.id != Block::AIR.id) {
        //         if block.light != 15 {
        //             let free_sky = blocks.iter().all(|(pos, block)| {
        //                 if pos.x != block_pos.x || pos.z != block_pos.z || pos.y <= block_pos.y {
        //                     return true;
        //                 }

        //                 block.id == Block::AIR.id
        //             });
        //             if free_sky {
        //                 let chunk = new_chunk
        //                     .get_or_insert_with(|| Rc::unwrap_or_clone(Rc::clone(&self.blocks)));
        //                 chunk[block_pos.map(|e| e as _).into_tuple()].light = 15;
        //             }
        //         }
        //     }

        //     if let Some(new_blocks) = new_chunk {
        //         self.blocks = Rc::new(new_blocks);
        //     }
        // }
    }

    fn modify_chunk(&mut self, f: impl FnOnce(&mut Array3<Block>) -> bool) {
        let mut blocks = Rc::unwrap_or_clone(Rc::clone(&self.blocks));
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

            false
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
