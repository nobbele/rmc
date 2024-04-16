use crate::shader::create_shader;

use super::screen_quad_renderer::{DrawParams, ScreenVertex};
use bytemuck::offset_of;
use glow::HasContext;
use rmc_common::BlockType;
use std::mem;
use vek::{Mat3, Vec2};

pub struct IsometricBlockRenderer {
    pub vao: glow::VertexArray,
    #[allow(dead_code)]
    pub vbo: glow::Buffer,
    #[allow(dead_code)]
    pub ebo: glow::Buffer,

    pub program: glow::Program,
}

impl IsometricBlockRenderer {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let mut indices = Vec::new();
        let mut vertices = Vec::new();

        let mut push = |vs: [ScreenVertex; 3]| {
            indices.extend([0, 1, 2].map(|i| i + vertices.len() as u8));
            vertices.extend_from_slice(&vs);
        };

        // TODO this is still scuffed...

        let angle = 30_f32.to_radians().sin().atan();
        let h = angle.sin() * std::f32::consts::SQRT_2 / 2.0;
        let w = angle.cos() * std::f32::consts::SQRT_2 / 2.0;

        let full_height = 0.5 * std::f32::consts::SQRT_2 + h * 2.0;
        let full_width = 2.0 * w;

        let scale = if full_width > full_height {
            1.0 / full_width
        } else {
            1.0 / full_height
        };

        let w = w * scale;
        let h = h * scale;
        let full_height = full_height * scale;

        let points = [
            Vec2::new(0.5, 0.0),
            Vec2::new(0.5 + w, h),
            Vec2::new(0.5 + w, full_height - h),
            Vec2::new(0.5, full_height),
            Vec2::new(0.5 - w, full_height - h),
            Vec2::new(0.5 - w, h),
        ];

        let center = Vec2::new(0.5, h * 2.0);

        // Front
        push([
            ScreenVertex::new(points[5], Vec2::new(0.0 / 3.0, 0.0 / 2.0)),
            ScreenVertex::new(points[4], Vec2::new(0.0 / 3.0, 1.0 / 2.0)),
            ScreenVertex::new(center, Vec2::new(1.0 / 3.0, 0.0 / 2.0)),
        ]);
        push([
            ScreenVertex::new(points[4], Vec2::new(0.0 / 3.0, 1.0 / 2.0)),
            ScreenVertex::new(points[3], Vec2::new(1.0 / 3.0, 1.0 / 2.0)),
            ScreenVertex::new(center, Vec2::new(1.0 / 3.0, 0.0 / 2.0)),
        ]);

        // Right
        push([
            ScreenVertex::new(points[3], Vec2::new(2.0 / 3.0, 1.0 / 2.0)),
            ScreenVertex::new(points[2], Vec2::new(3.0 / 3.0, 1.0 / 2.0)),
            ScreenVertex::new(center, Vec2::new(2.0 / 3.0, 0.0 / 2.0)),
        ]);
        push([
            ScreenVertex::new(points[2], Vec2::new(3.0 / 3.0, 1.0 / 2.0)),
            ScreenVertex::new(points[1], Vec2::new(3.0 / 3.0, 0.0 / 2.0)),
            ScreenVertex::new(center, Vec2::new(2.0 / 3.0, 0.0 / 2.0)),
        ]);

        // Top
        push([
            ScreenVertex::new(points[1], Vec2::new(2.0 / 3.0, 0.0 / 2.0)),
            ScreenVertex::new(points[0], Vec2::new(1.0 / 3.0, 0.0 / 2.0)),
            ScreenVertex::new(center, Vec2::new(2.0 / 3.0, 1.0 / 2.0)),
        ]);
        push([
            ScreenVertex::new(points[0], Vec2::new(1.0 / 3.0, 0.0 / 2.0)),
            ScreenVertex::new(points[5], Vec2::new(1.0 / 3.0, 1.0 / 2.0)),
            ScreenVertex::new(center, Vec2::new(2.0 / 3.0, 1.0 / 2.0)),
        ]);

        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
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
            indices.as_slice(),
            glow::STATIC_DRAW,
        );

        let program = create_shader(
            &gl,
            include_str!("../../shaders/isometric_block.vert"),
            include_str!("../../shaders/isometric_block.frag"),
        );

        IsometricBlockRenderer {
            vao,
            vbo,
            ebo,
            program,
        }
    }

    // TODO Instancing
    pub unsafe fn draw(&self, gl: &glow::Context, block_ty: BlockType, params: DrawParams) {
        if block_ty == BlockType::Air {
            return;
        };

        let screen_to_view_scale = Vec2::one() / Vec2::new(1024.0, 768.0);
        // TODO improve
        let screen_mat = Mat3::<f32>::identity()
            * Mat3::translation_2d(params.position * screen_to_view_scale)
            * Mat3::scaling_3d(
                (screen_to_view_scale * Vec2::new(32.0, 32.0) * params.scale).with_z(1.0),
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
        gl.uniform_1_u32(
            Some(
                &gl.get_uniform_location(self.program, "uniform_TextureLayer")
                    .unwrap(),
            ),
            block_ty as u32 - 1,
        );

        gl.bind_vertex_array(Some(self.vao));
        gl.draw_elements(glow::TRIANGLES, 18, glow::UNSIGNED_BYTE, 0);
    }
}
