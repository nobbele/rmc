use std::{collections::HashSet, rc::Rc};

use crate::{
    camera::Angle,
    world::{raycast, Block, RaycastOutput},
    Blend, Camera,
};
use ndarray::Array3;
use sdl2::{keyboard::Keycode, mouse::MouseState};
use vek::{Vec2, Vec3};

const GRAVITY: f32 = 0.04;
const SPEED: f32 = 0.4;

#[derive(Clone)]
pub struct InputState {
    pub keys: HashSet<Keycode>,
    pub mouse_state: MouseState,
    pub blocked_mouse: bool,
    pub mouse_delta: Vec2<f32>,
}

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

    pub fn update(&mut self, prev_input: &InputState, input: &InputState) {
        let initial = self.clone();

        self.handle_camera_movement(input);
        self.handle_movement(input);

        self.velocity.y -= GRAVITY;

        self.camera.position += self.velocity;

        self.handle_collision(&initial);

        self.look_at_raycast = raycast(
            self.camera.position,
            self.camera.look_at(),
            7.5,
            self.blocks.view(),
        );

        self.handle_place_destroy(prev_input, input);
    }

    fn handle_camera_movement(&mut self, input: &InputState) {
        self.camera.rotate_horizontal(input.mouse_delta.x);
        self.camera.rotate_vertical(input.mouse_delta.y);
    }

    fn handle_movement(&mut self, input: &InputState) {
        let fwd_bck =
            input.keys.contains(&Keycode::W) as i8 - input.keys.contains(&Keycode::S) as i8;
        let rgh_lft =
            input.keys.contains(&Keycode::D) as i8 - input.keys.contains(&Keycode::A) as i8;
        let up_down = input.keys.contains(&Keycode::Space) as i8
            - input.keys.contains(&Keycode::LShift) as i8;
        self.camera.move_forward(fwd_bck as f32 * SPEED);
        self.camera.move_right(rgh_lft as f32 * SPEED);
        self.camera.move_up(up_down as f32 * SPEED);
    }

    fn handle_collision(&mut self, initial: &Game) {
        if self.camera.position.floor().map(|e| e as i32)
            != initial.camera.position.floor().map(|e| e as i32)
        {
            let position_below = (self.camera.position - Vec3::new(0.0, 1.0, 0.0)).floor();

            if let (Some(Some(Some(_))), _) | (_, Some(Some(Some(_)))) = (
                (self.camera.position.map(|e| e.signum()).are_all_positive()).then(|| {
                    self.blocks
                        .get(self.camera.position.floor().map(|e| e as _).into_tuple())
                }),
                (position_below.map(|e| e.signum()).are_all_positive()).then(|| {
                    self.blocks
                        .get(position_below.map(|e| e as usize).into_tuple())
                }),
            ) {
                self.camera.position = initial.camera.position;
                self.velocity = Vec3::zero();
            }
        }
    }

    fn modify_chunk(&mut self, f: impl FnOnce(&mut Array3<Option<Block>>) -> bool) {
        let mut blocks = Rc::<_>::unwrap_or_clone(self.blocks.clone());
        if f(&mut blocks) {
            self.blocks = Rc::new(blocks);
        }
    }

    fn set_block(&mut self, position: Vec3<i32>, block: Option<Block>) {
        self.modify_chunk(|chunk| {
            if let Some(entry) = chunk.get_mut(position.map(|e| e as _).into_tuple()) {
                *entry = block;
                return true;
            }
            return false;
        });
    }

    fn handle_place_destroy(&mut self, prev_input: &InputState, input: &InputState) {
        if let Some(highlighted) = self.look_at_raycast {
            if !input.blocked_mouse && input.mouse_state.left() && !prev_input.mouse_state.left() {
                self.set_block(highlighted.position, None);
            }

            if !input.blocked_mouse && input.mouse_state.right() && !prev_input.mouse_state.right()
            {
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
