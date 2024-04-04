use super::face_to_tri;
use bytemuck::offset_of;
use glow::HasContext;
use std::mem;
use vek::Vec2;

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct ScreenVertex {
    pub position: Vec2<f32>,
    pub uv: Vec2<f32>,
}

unsafe impl bytemuck::Pod for ScreenVertex {}
unsafe impl bytemuck::Zeroable for ScreenVertex {}

pub struct ScreenQuadRenderer {
    #[allow(dead_code)]
    vao: glow::VertexArray,
    #[allow(dead_code)]
    vbo: glow::Buffer,
    #[allow(dead_code)]
    ebo: glow::Buffer,
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
            bytemuck::cast_slice::<[u8; 6], u8>(&[face_to_tri(&[0, 1, 2, 3])]),
            glow::STATIC_DRAW,
        );

        ScreenQuadRenderer { vao, vbo, ebo }
    }

    pub unsafe fn draw(&self, gl: &glow::Context, texture: glow::Texture) {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_BYTE, 0);
    }
}
