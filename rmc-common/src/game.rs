use crate::{
    camera::Angle,
    collision::{sweep_test, SweepBox, SweepTestResult},
    input::InputState,
    light::calculate_block_light,
    raycast::{raycast, RaycastOutput},
    world::{face_neighbors, generate_chunk, Chunk, World, CHUNK_SIZE},
    Blend, Block, BlockType, Camera, DiscreteBlend,
};
use crossbeam_queue::SegQueue;
use enum_assoc::Assoc;
use itertools::Itertools;
use lazy_static::lazy_static;
use noise::NoiseFn;
use sdl2::{keyboard::Keycode, mouse::MouseButton};
use std::{collections::HashMap, ops::Deref, rc::Rc, thread::JoinHandle};
use vek::{Aabb, Extent3, Vec2, Vec3};

pub const TICK_RATE: u32 = 16;
pub const TICK_SPEED: f32 = 1.0;
pub const TICK_DELTA: f32 = 1.0 / TICK_RATE as f32;

const GRAVITY: f32 = 16.0;
const JUMP_HEIGHT: f32 = 1.0;
lazy_static! {
    // sqrt isn't const fn :/
    pub static ref JUMP_STRENGTH: f32 = 1.15 * (2.0 * GRAVITY * JUMP_HEIGHT - 1.0).sqrt();
}
const SPEED: f32 = 6.0;
// const SPEED: f32 = 16.0;

const PLAYER_SIZE: Vec3<f32> = Vec3::new(0.2, 1.8, 0.2);
const PLAYER_ORIGIN: Vec3<f32> = Vec3::new(0.1, 1.5, 0.1);

#[derive(Clone)]
pub struct BlockUpdate {
    pub target: Vec3<i32>,

    /// Which block caused the update,
    /// or None for non-block causes such as user placing/destroying a block.
    pub source: Option<Vec3<i32>>,

    /// If this update was caused by another block changing it's state,
    /// such as user placing/destroying a block,
    /// or a neighbor having it's light updated.
    pub state_changed: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Assoc)]
#[func(pub fn name(&self) -> &'static str { "??" })]
pub enum Item {
    #[default]
    #[assoc(name = "Empty")]
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockOrItem {
    Item(Item),
    Block(BlockType),
}

impl BlockOrItem {
    pub fn name(&self) -> &'static str {
        match self {
            BlockOrItem::Item(item) => item.name(),
            BlockOrItem::Block(block) => block.name(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
pub struct TerrainSampler {
    seed: u32,
}

impl TerrainSampler {
    pub fn new(seed: u32) -> Self {
        TerrainSampler { seed }
    }

    pub fn height(&self, position: Vec2<i32>) -> u32 {
        const SCALE: f64 = 0.027;
        let height = noise::OpenSimplex::new(self.seed)
            .get([position.x as f64 * SCALE, position.y as f64 * SCALE]);
        let height = (1.0 + height) * 0.5;
        let height = height * 20.0;
        32 + height as u32
    }

    pub fn cave(&self, position: Vec3<i32>) -> bool {
        if position.y < 0 || position.y > 32 {
            return false;
        }

        const SCALE: f64 = 0.027;
        let v = noise::OpenSimplex::new(self.seed).get([
            position.x as f64 * SCALE,
            position.y as f64 * SCALE,
            position.z as f64 * SCALE,
        ]);
        v > 0.3
    }
}

impl DiscreteBlend for TerrainSampler {}

#[derive(Clone)]
pub struct ChunkLoader {
    #[allow(dead_code)]
    handle: Rc<Vec<JoinHandle<()>>>,
    tx: crossbeam_channel::Sender<Vec3<i32>>,
    rx: crossbeam_channel::Receiver<(Vec3<i32>, Chunk)>,
}

impl ChunkLoader {
    pub fn new(terrain: TerrainSampler) -> Self {
        let (tx, thread_rx) = crossbeam_channel::unbounded::<Vec3<i32>>();
        let (thread_tx, rx) = crossbeam_channel::unbounded::<(Vec3<i32>, Chunk)>();
        let handle = (0..std::thread::available_parallelism().unwrap().get())
            .map(|_| {
                let thread_rx = thread_rx.clone();
                let thread_tx = thread_tx.clone();
                let terrain = terrain.clone();
                std::thread::spawn(move || {
                    while let Ok(chunk_coord) = thread_rx.recv() {
                        // println!("({}) Handling {}", i, chunk_coord);
                        thread_tx
                            .send((chunk_coord, generate_chunk(&terrain, chunk_coord)))
                            .unwrap();
                    }
                })
            })
            .collect_vec();
        ChunkLoader {
            handle: Rc::new(handle),
            tx,
            rx,
        }
    }

    pub fn request(&self, chunk_coord: Vec3<i32>) {
        self.tx.send(chunk_coord).unwrap();
    }

    pub fn receive(&self) -> Option<(Vec3<i32>, Chunk)> {
        match self.rx.try_recv() {
            Ok((chunk_coord, chunk)) => Some((chunk_coord, chunk)),
            Err(crossbeam_channel::TryRecvError::Empty) => None,
            Err(e) => Err(e).unwrap(),
        }
    }
}

impl DiscreteBlend for ChunkLoader {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discrete<T>(pub T);

impl<T: Deref> Deref for Discrete<T> {
    type Target = <T as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        <T as Deref>::deref(&self.0)
    }
}

impl<T> DiscreteBlend for Discrete<T> {}

#[derive(Clone)]
pub struct Game {
    pub world: World,
    pub chunk_loader: ChunkLoader,

    pub camera: Camera,
    pub velocity: Vec3<f32>,

    pub on_ground: bool,
    pub look_at_raycast: Option<RaycastOutput>,

    pub dirty_blocks: Discrete<Rc<crossbeam_queue::SegQueue<BlockUpdate>>>,
    pub block_update_count: usize,
    pub total_block_update_count: usize,

    pub hotbar: Hotbar,
    pub flying: bool,
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new(Vec3::zero());
        let chunk_loader = ChunkLoader::new(TerrainSampler::new(54327));

        let unloaded_chunks = world.unloaded_chunks().collect_vec();
        let _total = unloaded_chunks.len();
        for chunk_coord in unloaded_chunks {
            chunk_loader.request(chunk_coord);
        }

        let mut _loaded = 0;
        while world.unloaded_chunks().next().is_some() {
            while let Some((chunk_coord, chunk)) = chunk_loader.receive() {
                world.load(chunk_coord, chunk);
                // loaded += 1;
                // println!(
                //     "Loaded chunk {loaded} / {total} ({:.0}%)",
                //     loaded as f32 / total as f32 * 100.0
                // );
            }
        }

        let mut game = Game {
            chunk_loader,
            world,

            camera: Camera {
                position: Vec3::new(8.5, 48.0, 8.5),
                pitch: Angle(0.0),
                yaw: Angle(0.0),
            },
            velocity: Vec3::zero(),

            on_ground: false,

            look_at_raycast: None,
            dirty_blocks: Discrete(Rc::new(SegQueue::new())),
            block_update_count: 0,
            total_block_update_count: 0,

            hotbar: Hotbar::new(),
            flying: false,
        };

        game.set_block(Vec3::new(6, 14, 8), Block::LANTERN);
        game.set_block(Vec3::new(-8, 14, -8), Block::LANTERN);
        game.hotbar.slots[0] = Some(BlockOrItem::Block(BlockType::Wood));
        game.hotbar.slots[1] = Some(BlockOrItem::Block(BlockType::Lantern));
        game.hotbar.slots[2] = Some(BlockOrItem::Block(BlockType::Test));
        game.hotbar.slots[3] = Some(BlockOrItem::Block(BlockType::Stone));
        game.hotbar.slots[4] = Some(BlockOrItem::Block(BlockType::Mesh));

        game
    }

    pub fn update(&mut self, input: &InputState) {
        let initial = self.clone();

        self.handle_camera_movement(input);
        self.handle_movement(input);

        if !self.flying {
            self.velocity.y -= GRAVITY * TICK_DELTA;
        } else {
            self.velocity.y = 0.0;
        }
        self.camera.position += self.velocity * TICK_DELTA;

        self.handle_collision(&initial);

        self.look_at_raycast = raycast(self.camera.position, self.camera.look_at(), 7.5, |pos| {
            self.world.get_block(pos)
        });

        self.hotbar.active = (self.hotbar.active as i32 - input.scroll_delta)
            .rem_euclid(self.hotbar.slots.len() as i32) as usize;

        self.handle_place_destroy(input);
        self.update_blocks();

        if input.get_key(Keycode::P).just_pressed() {
            self.flying = !self.flying;
        }

        if self.chunk_coordinate() != self.world.origin() {
            self.world.set_origin(self.chunk_coordinate());

            let unloaded_chunks = self
                .world
                .unloaded_chunks()
                .filter(|chunk_coord| {
                    chunk_coord
                        .as_::<f32>()
                        .distance(self.world.origin().as_::<f32>())
                        < self.world.extents.as_::<f32>().average()
                })
                .collect_vec();

            for chunk_coord in unloaded_chunks {
                self.chunk_loader.request(chunk_coord);
            }
        }

        while let Some((chunk_coord, chunk)) = self.chunk_loader.receive() {
            self.world.load(chunk_coord, chunk);
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
        self.camera.position += movement_vector.try_normalized().unwrap_or_default()
            * SPEED
            * TICK_DELTA
            * if self.flying { 10.0 } else { 1.0 };

        if self.flying && input.get_key(Keycode::Space).pressed() {
            self.camera.position.y += 10.0 * TICK_DELTA;
        }

        if self.on_ground {
            self.velocity.y = up_down as f32 * *JUMP_STRENGTH;
        }
    }

    fn handle_collision(&mut self, initial: &Game) {
        self.on_ground = false;

        const MAX_ITERATIONS: usize = 4;

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

            for (pos, block) in self
                .world
                .chunks_iter()
                .filter(|(pos, _c)| {
                    broad_box.collides_with_aabb(Aabb {
                        min: pos.as_() * CHUNK_SIZE as f32,
                        max: (pos.as_() + Vec3::one()) * CHUNK_SIZE as f32,
                    })
                })
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
            {
                let block_box = Aabb {
                    min: pos.as_(),
                    max: pos.as_() + Vec3::one(),
                };

                if block.ty != BlockType::Air && broad_box.collides_with_aabb(block_box) {
                    if let Some(result) = sweep_test(player_sweep, block_box) {
                        collisions.push(result);
                    }
                }
            }

            // WTF How does this improve the collision detection???
            collisions.reverse();

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
        const MAX_UPDATES_COUNT: usize = 2048;

        self.block_update_count = 0;

        while self.block_update_count < MAX_UPDATES_COUNT && self.dirty_blocks.len() != 0 {
            let update_count = self.dirty_blocks.len().min(MAX_UPDATES_COUNT);
            self.block_update_count += update_count;
            self.total_block_update_count += update_count;

            let dirty_blocks = (0..update_count)
                .map(|_| self.dirty_blocks.pop().unwrap())
                .collect_vec();

            let mut replaces = HashMap::new();
            for BlockUpdate {
                target: position,
                source,
                state_changed,
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

                new_block.occluded = face_neighbors(position).into_iter().all(|position| {
                    if let Some(block) = self.world.get_block(position) {
                        !block.ty.light_passing()
                    } else {
                        false
                    }
                });

                new_block.light = calculate_block_light(&self.world, position, new_block, source);

                if new_block != block {
                    replaces.insert(position, new_block);
                }

                let should_notify_neighbor =
                    block.light != new_block.light || block.open_to_sky != new_block.open_to_sky;

                // Hack: If the source is None (i.e placed by user).
                // then always update the neighbors.
                // Also, almost all updates are from `concealed`
                if source.is_none() || should_notify_neighbor || state_changed {
                    for neighbor in face_neighbors(position)
                        .into_iter()
                        // .filter(|&p| Some(p) != source)
                        .map(|p| BlockUpdate {
                            target: p,
                            source: Some(p),
                            state_changed: should_notify_neighbor,
                        })
                    {
                        self.dirty_blocks.push(neighbor);
                    }
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
        if self.world.set_block(position, block).is_ok() {
            if update {
                self.dirty_blocks.push(BlockUpdate {
                    target: position,
                    source: None,
                    state_changed: true,
                });
            }
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
            chunk_loader: self.chunk_loader.blend(&other.chunk_loader, alpha),

            camera: self.camera.blend(&other.camera, alpha),
            velocity: self.velocity.blend(&other.velocity, alpha),

            on_ground: self.on_ground.blend(&other.on_ground, alpha),

            look_at_raycast: self.look_at_raycast.blend(&other.look_at_raycast, alpha),
            dirty_blocks: self.dirty_blocks.blend(&other.dirty_blocks, alpha),
            block_update_count: self
                .block_update_count
                .blend(&other.block_update_count, alpha),
            total_block_update_count: self
                .total_block_update_count
                .blend(&other.total_block_update_count, alpha),

            hotbar: self.hotbar.blend(&other.hotbar, alpha),
            flying: self.flying.blend(&other.flying, alpha),
        }
    }
}

#[test]
pub fn test_game_state_size() {
    // The size of the game state should not grow too large due to frequent use of cloning during updates and blending.
    const MAX_SIZE: usize = 512;

    assert!(
        std::mem::size_of::<Game>() < MAX_SIZE,
        "Size of `Game` ({} bytes) needs to be smaller than {} bytes",
        std::mem::size_of::<Game>(),
        MAX_SIZE
    );
}
