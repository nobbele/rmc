use crate::{
    camera::Angle,
    collision::{sweep_test, SweepBox, SweepTestResult},
    input::InputState,
    light::calculate_block_light,
    raycast::{raycast, RaycastOutput},
    world::{face_neighbors, Chunk, World, CHUNK_SIZE},
    Blend, Block, BlockType, Camera, DiscreteBlend,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use sdl2::{keyboard::Keycode, mouse::MouseButton};
use std::collections::{HashMap, VecDeque};
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

const PLAYER_SIZE: Vec3<f32> = Vec3::new(0.2, 1.8, 0.2);
const PLAYER_ORIGIN: Vec3<f32> = Vec3::new(0.1, 1.5, 0.1);

#[derive(Clone)]
pub struct BlockUpdate {
    pub target: Vec3<i32>,
    pub source: Option<Vec3<i32>>,
}

#[derive(Clone, Copy)]
pub enum Item {}

#[derive(Clone, Copy)]
pub enum BlockOrItem {
    Item(Item),
    Block(BlockType),
}

#[derive(Clone, Copy)]
pub struct Hotbar {
    pub slots: [Option<BlockOrItem>; 9],
    pub active: usize,
}

impl Hotbar {
    pub fn new() -> Self {
        Hotbar {
            slots: [None; 9],
            active: 0,
        }
    }
}

impl DiscreteBlend for Hotbar {}

#[derive(Clone)]
pub struct Game {
    pub world: World,

    pub camera: Camera,
    pub velocity: Vec3<f32>,

    pub on_ground: bool,
    pub look_at_raycast: Option<RaycastOutput>,

    pub dirty_blocks: VecDeque<BlockUpdate>,
    // This is per frame
    pub block_update_count: usize,

    pub hotbar: Hotbar,
}

fn initialize_chunk(world: &mut World, chunk: Vec3<i32>) {
    for y in 0..14 {
        for z in 0..16 {
            for x in 0..16 {
                world.set_block(
                    chunk * CHUNK_SIZE as i32 + Vec3::new(x, y, z).as_(),
                    Block::GRASS,
                );
            }
        }
    }
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new(Vec3::zero());
        for (chunk_x, chunk_z) in (-1_i32..=1).cartesian_product(-1_i32..=1) {
            initialize_chunk(&mut world, Vec3::new(chunk_x, 0, chunk_z));
        }

        for z in 0..15 {
            for x in 0..15 {
                world.set_block(
                    Vec3::new(-1 * CHUNK_SIZE as i32 + x, 19, -1 * CHUNK_SIZE as i32 + z).as_(),
                    Block::WOOD,
                );
            }
        }

        for x in 0..15 {
            for y in 0..6 {
                world.set_block(
                    Vec3::new(
                        -1 * CHUNK_SIZE as i32 + x,
                        y + 14,
                        -1 * CHUNK_SIZE as i32 + 0,
                    )
                    .as_(),
                    Block::WOOD,
                );
            }
        }

        for x in 0..15 {
            for y in 0..6 {
                world.set_block(
                    Vec3::new(-1 * CHUNK_SIZE as i32 + x, y + 14, -2).as_(),
                    Block::WOOD,
                );
            }
        }

        for z in 0..15 {
            for y in 0..6 {
                world.set_block(
                    Vec3::new(
                        -1 * CHUNK_SIZE as i32 + 0,
                        y + 14,
                        -1 * CHUNK_SIZE as i32 + z,
                    )
                    .as_(),
                    Block::WOOD,
                );
            }
        }

        let mut game = Game {
            world,

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

            hotbar: Hotbar::new(),
        };

        game.set_block(Vec3::new(6, 14, 8), Block::LANTERN);
        game.set_block(Vec3::new(-8, 14, -8), Block::LANTERN);
        game.hotbar.slots[0] = Some(BlockOrItem::Block(BlockType::Wood));
        game.hotbar.slots[1] = Some(BlockOrItem::Block(BlockType::Lantern));

        game
    }

    pub fn update(&mut self, input: &InputState) {
        let initial = self.clone();

        self.handle_camera_movement(input);
        self.handle_movement(input);

        self.velocity.y -= GRAVITY * TICK_DELTA;
        self.camera.position += self.velocity * TICK_DELTA;

        self.handle_collision(&initial);

        self.look_at_raycast = raycast(self.camera.position, self.camera.look_at(), 7.5, |pos| {
            self.world.get_block(pos)
        });

        self.hotbar.active = (self.hotbar.active as i32 - input.scroll_delta)
            .rem_euclid(self.hotbar.slots.len() as i32) as usize;

        self.handle_place_destroy(input);
        self.update_blocks();

        if self.chunk_coordinate() != initial.chunk_coordinate() {
            self.world.set_origin(self.chunk_coordinate());

            let unloaded_chunks = self
                .world
                .chunks
                .indexed_iter()
                .filter_map(|(idx, chunk)| {
                    if chunk.is_none() {
                        Some(self.world.index_to_chunk(Vec3::<usize>::from(idx)))
                    } else {
                        None
                    }
                })
                .collect_vec();

            for chunk_coordinate in unloaded_chunks {
                self.world.load(chunk_coordinate, Chunk::default());

                if chunk_coordinate.y == 0 {
                    initialize_chunk(&mut self.world, chunk_coordinate);
                }

                // TODO do this in a parallel thread to not be super slow?
                // for (idx, _block) in self
                //     .world
                //     .chunk_at(chunk_coordinate)
                //     .unwrap()
                //     .blocks
                //     .indexed_iter()
                // {
                //     let local_coord = Vec3::<usize>::from(idx).as_();
                //     // Only update the borders
                //     if local_coord.into_iter().any(|e| e == 0 || e == 15) {
                //         let world_coord = chunk_coordinate * CHUNK_SIZE as i32 + local_coord;
                //         self.dirty_blocks.push_back(BlockUpdate {
                //             target: world_coord,
                //             source: None,
                //         });
                //     }
                // }
            }
        }
    }

    fn handle_camera_movement(&mut self, input: &InputState) {
        self.camera.rotate_horizontal(input.mouse_delta.x);
        self.camera.rotate_vertical(input.mouse_delta.y);
    }

    fn handle_movement(&mut self, input: &InputState) {
        let up_down = input.get_key(Keycode::Space).pressed() as i8
            - input.get_key(Keycode::LShift).pressed() as i8;
        let input_vector = input.get_movement_vector();
        let movement_vector =
            input_vector.x * self.camera.right() + input_vector.y * self.camera.forward();
        self.camera.position +=
            movement_vector.try_normalized().unwrap_or_default() * SPEED * TICK_DELTA;

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

            // TODO broad phase over chunks.
            for (pos, _block) in self
                .world
                .chunks_iter()
                .flat_map(|(chunk_coord, chunk)| {
                    chunk
                        .blocks
                        .indexed_iter()
                        .map(|(offset, block)| {
                            (
                                chunk_coord * CHUNK_SIZE as i32 + Vec3::<usize>::from(offset).as_(),
                                *block,
                            )
                        })
                        .collect_vec()
                        .into_iter()
                })
                .map(|(pos, block)| {
                    (
                        pos,
                        if block.ty == BlockType::Air {
                            None
                        } else {
                            Some(block)
                        },
                    )
                })
                .filter_map(|(pos, block)| block.map(|b| (pos, b)))
                // WTF How does this improve the collision detection???
                .collect_vec()
                .into_iter()
                .rev()
            {
                let block_box = Aabb {
                    min: pos.as_(),
                    max: pos.as_() + Vec3::one(),
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

            if normal == -Vec3::unit_y() {
                self.velocity.y = 0.0;
            }

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

            let mut replaces = HashMap::new();
            for BlockUpdate {
                target: position,
                source,
            } in dirty_blocks
            {
                // TODO If this fails, it should be put in some wait queue that gets flushed once a chunk loads.
                let Some(block) = self.world.get_block(position) else {
                    continue;
                };

                if replaces.contains_key(&position) {
                    continue;
                }

                let mut new_block = block;

                new_block.open_to_sky =
                    if let Some(block_above) = self.world.get_block(position + Vec3::unit_y()) {
                        block_above.ty.light_passing() && block_above.open_to_sky
                    } else {
                        // World border, true.
                        true
                    };

                new_block.light = calculate_block_light(&self.world, position, new_block, source);

                // Hack: If the source is None (i.e placed by user).
                // Then always update the neighbors,
                if block != new_block || source.is_none() {
                    self.dirty_blocks.extend(
                        face_neighbors(position)
                            .into_iter()
                            // .filter(|&p| Some(p) != source)
                            .map(|p| BlockUpdate {
                                target: p,
                                source: Some(p),
                            }),
                    );
                    replaces.insert(position, new_block);
                }
            }

            for (position, block) in replaces {
                self.set_block1(position, block, false);
            }
        }
    }

    pub fn set_block(&mut self, position: Vec3<i32>, block: Block) {
        self.set_block1(position, block, true);
    }

    pub fn set_block1(&mut self, position: Vec3<i32>, block: Block, update: bool) {
        self.world.set_block(position, block);
        if update {
            self.dirty_blocks.push_back(BlockUpdate {
                target: position,
                source: None,
            });
        }
    }

    fn handle_place_destroy(&mut self, input: &InputState) {
        if let Some(highlighted) = self.look_at_raycast {
            if input.get_mouse_button(MouseButton::Left).just_pressed() {
                self.set_block(highlighted.position, Block::AIR);
            }

            if input.get_mouse_button(MouseButton::Right).just_pressed() {
                let position = highlighted.position + highlighted.normal.numcast().unwrap();

                if let Some(BlockOrItem::Block(block_ty)) = self.hotbar.slots[self.hotbar.active] {
                    self.set_block(position, Block::new(block_ty));
                }
            }

            if input.get_mouse_button(MouseButton::Middle).just_pressed() {
                let position = highlighted.position + highlighted.normal.numcast().unwrap();

                self.set_block(position, Block::LANTERN);
            }
        }
    }

    pub fn block_coordinate(&self) -> Vec3<i32> {
        self.camera.position.map(|e| e.floor() as i32)
    }

    pub fn chunk_coordinate(&self) -> Vec3<i32> {
        self.world.world_to_chunk(self.block_coordinate())
    }
}

impl Blend for Game {
    fn blend(&self, other: &Game, alpha: f32) -> Self {
        Self {
            world: self.world.blend(&other.world, alpha),

            camera: self.camera.blend(&other.camera, alpha),
            velocity: self.velocity.blend(&other.velocity, alpha),

            on_ground: self.on_ground.blend(&other.on_ground, alpha),

            look_at_raycast: self.look_at_raycast.blend(&other.look_at_raycast, alpha),
            dirty_blocks: self.dirty_blocks.blend(&other.dirty_blocks, alpha),
            block_update_count: self
                .block_update_count
                .blend(&other.block_update_count, alpha),

            hotbar: self.hotbar.blend(&other.hotbar, alpha),
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
