use crate::{shader::create_shader, texture::Image};

use super::face_to_tri;
use bytemuck::offset_of;
use glow::HasContext;
use std::mem;
use vek::{Mat3, Vec2};

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct ScreenVertex {
    pub position: Vec2<f32>,
    pub uv: Vec2<f32>,
}

impl ScreenVertex {
    pub fn new(position: Vec2<f32>, uv: Vec2<f32>) -> Self {
        ScreenVertex { position, uv }
    }
}

unsafe impl bytemuck::Pod for ScreenVertex {}
unsafe impl bytemuck::Zeroable for ScreenVertex {}

pub struct ScreenQuadRenderer {
    pub vao: glow::VertexArray,
    #[allow(dead_code)]
    pub vbo: glow::Buffer,
    #[allow(dead_code)]
    pub ebo: glow::Buffer,

    pub program: glow::Program,
}

impl ScreenQuadRenderer {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&[
                ScreenVertex {
                    position: Vec2::new(0.0, 0.0),
                    uv: Vec2::new(0.0, 1.0),
                },
                ScreenVertex {
                    position: Vec2::new(1.0, 0.0),
                    uv: Vec2::new(1.0, 1.0),
                },
                ScreenVertex {
                    position: Vec2::new(0.0, 1.0),
                    uv: Vec2::new(0.0, 0.0),
                },
                ScreenVertex {
                    position: Vec2::new(1.0, 1.0),
                    uv: Vec2::new(1.0, 0.0),
                },
            ]),
            glow::STATIC_DRAW,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(
            0,
            2,
            glow::FLOAT,
            false,
            mem::size_of::<ScreenVertex>() as _,
            offset_of!(ScreenVertex, position) as _,
        );
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(
            1,
            2,
            glow::FLOAT,
            false,
            mem::size_of::<ScreenVertex>() as _,
            offset_of!(ScreenVertex, uv) as _,
        );

        let ebo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice::<[u8; 6], u8>(&[{
                let mut indices = face_to_tri(&[0, 1, 2, 3]);
                indices.reverse();
                indices
            }]),
            glow::STATIC_DRAW,
        );

        let program = create_shader(
            &gl,
            include_str!("../../shaders/screen.vert"),
            include_str!("../../shaders/screen.frag"),
        );

        ScreenQuadRenderer {
            vao,
            vbo,
            ebo,
            program,
        }
    }

    // TODO Instancing
    pub unsafe fn draw(&self, gl: &glow::Context, image: &Image, params: DrawParams) {
        let screen_to_view_scale = Vec2::one() / Vec2::new(1024.0, 768.0);
        // TODO improve
        let screen_mat = Mat3::<f32>::identity()
            * Mat3::translation_2d(params.position * screen_to_view_scale)
            * Mat3::scaling_3d(
                (screen_to_view_scale * image.size.as_::<f32>() * params.scale).with_z(1.0),
            )
            * Mat3::translation_2d(-params.origin);

        gl.use_program(Some(self.program));
        gl.uniform_matrix_3_f32_slice(
            Some(
                &gl.get_uniform_location(self.program, "uniform_Mat")
                    .unwrap(),
            ),
            false,
            screen_mat.as_col_slice(),
        );

        gl.bind_texture(glow::TEXTURE_2D, Some(image.raw));
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_BYTE, 0);
    }
}

// pub enum ScaleOrSize {
//     Scale(Vec2<f32>),
//     Size(Vec2<f32>),
// }

#[derive(Clone, Copy)]
pub struct DrawParams {
    pub position: Vec2<f32>,
    pub origin: Vec2<f32>,
    pub scale: Vec2<f32>,
}

impl DrawParams {
    pub fn scale(mut self, scale: Vec2<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn position(mut self, position: Vec2<f32>) -> Self {
        self.position = position;
        self
    }

    pub fn origin(mut self, origin: Vec2<f32>) -> Self {
        self.origin = origin;
        self
    }
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            position: Vec2::zero(),
            origin: Vec2::zero(),
            scale: Vec2::one(),
        }
    }
}
