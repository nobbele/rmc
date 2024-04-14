use glow::HasContext;
use rmc_common::Game;
use vek::{Mat3, Mat4, Vec2, Vec3};

use crate::{
    shader::create_shader,
    texture::{load_array_texture, DataSource},
};

use super::ChunkRenderer;

pub struct GameRenderer {
    pub projection: Mat4<f32>,

    pub chunk_renderer: ChunkRenderer,

    block_array_texture: glow::Texture,
    screen_program: glow::Program,
    program: glow::Program,
}

impl GameRenderer {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let block_array_texture = load_array_texture(
            &gl,
            &[
                DataSource::Inline(include_bytes!("../../textures/test.png")),
                DataSource::Inline(include_bytes!("../../textures/grass.png")),
                DataSource::Inline(include_bytes!("../../textures/test.png")),
            ],
        );

        let program = create_shader(
            &gl,
            include_str!("../../shaders/cube.vert"),
            include_str!("../../shaders/cube.frag"),
        );

        let screen_program = create_shader(
            &gl,
            include_str!("../../shaders/screen.vert"),
            include_str!("../../shaders/screen.frag"),
        );

        GameRenderer {
            projection: Mat4::<f32>::infinite_perspective_rh(120_f32.to_radians(), 4. / 3., 0.0001),

            chunk_renderer: ChunkRenderer::new(gl),

            block_array_texture,
            screen_program,
            program,
        }
    }

    pub unsafe fn draw(&self, gl: &glow::Context, game: &Game) {
        let mvp = self.projection * game.camera.to_matrix();

        gl.use_program(Some(self.program));
        gl.uniform_matrix_4_f32_slice(
            Some(
                &gl.get_uniform_location(self.program, "uniform_Mvp")
                    .unwrap(),
            ),
            false,
            mvp.as_col_slice(),
        );
        let uniform_highlighted = game
            .look_at_raycast
            .map(|v| v.position.map(|e| e as f32))
            .unwrap_or(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
        gl.uniform_3_f32(
            Some(
                &gl.get_uniform_location(self.program, "uniform_Highlighted")
                    .unwrap(),
            ),
            uniform_highlighted.x,
            uniform_highlighted.y,
            uniform_highlighted.z,
        );

        gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.block_array_texture));
        self.chunk_renderer.draw(&gl);

        let size = Vec2::new(48.0, 48.0);
        let screen_mat = Mat3::<f32>::scaling_3d((size / Vec2::new(1024.0, 768.0)).with_z(1.0))
            * Mat3::<f32>::translation_2d(Vec2::new(-1.0, -1.0))
            * Mat3::<f32>::scaling_3d(Vec2::broadcast(2.0).with_z(1.0));

        gl.use_program(Some(self.screen_program));
        gl.uniform_matrix_3_f32_slice(
            Some(
                &gl.get_uniform_location(self.screen_program, "uniform_Mat")
                    .unwrap(),
            ),
            false,
            screen_mat.as_col_slice(),
        );
    }
}
