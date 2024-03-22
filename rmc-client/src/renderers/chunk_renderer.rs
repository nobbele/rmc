use std::mem;

use bytemuck::offset_of;
use glow::HasContext;
use rmc_common::world::Block;
use vek::{Vec2, Vec3};

use super::face_to_tri;

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3<f32>,
    pub uv: Vec2<f32>,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct Instance {
    pub position: Vec3<f32>,
    pub texture: u8,
}

unsafe impl bytemuck::Pod for Instance {}
unsafe impl bytemuck::Zeroable for Instance {}

pub struct ChunkRenderer {
    #[allow(dead_code)]
    vao: glow::VertexArray,
    #[allow(dead_code)]
    vbo: glow::Buffer,
    #[allow(dead_code)]
    ebo: glow::Buffer,
    ibo: glow::Buffer,
    ibo_size: usize,
}

impl ChunkRenderer {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&[
                // Back vertices
                Vertex {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    uv: Vec2::new(1.0 / 3.0, 2.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 0.0, 0.0),
                    uv: Vec2::new(0.0 / 3.0, 2.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 1.0, 0.0),
                    uv: Vec2::new(1.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 1.0, 0.0),
                    uv: Vec2::new(0.0 / 3.0, 1.0 / 2.0),
                },
                // Front vertices
                Vertex {
                    position: Vec3::new(0.0, 0.0, 1.0),
                    uv: Vec2::new(0.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 0.0, 1.0),
                    uv: Vec2::new(1.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 1.0, 1.0),
                    uv: Vec2::new(0.0, 0.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 1.0, 1.0),
                    uv: Vec2::new(1.0 / 3.0, 0.0 / 2.0),
                },
                // Right vertices
                Vertex {
                    position: Vec3::new(1.0, 0.0, 0.0),
                    uv: Vec2::new(3.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 0.0, 1.0),
                    uv: Vec2::new(2.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 1.0, 0.0),
                    uv: Vec2::new(3.0 / 3.0, 0.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 1.0, 1.0),
                    uv: Vec2::new(2.0 / 3.0, 0.0 / 2.0),
                },
                // Left vertices
                Vertex {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    uv: Vec2::new(2.0 / 3.0, 2.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.0, 1.0),
                    uv: Vec2::new(3.0 / 3.0, 2.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 1.0, 0.0),
                    uv: Vec2::new(2.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 1.0, 1.0),
                    uv: Vec2::new(3.0 / 3.0, 1.0 / 2.0),
                },
                // Top vertices
                Vertex {
                    position: Vec3::new(0.0, 1.0, 1.0),
                    uv: Vec2::new(1.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 1.0, 1.0),
                    uv: Vec2::new(2.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 1.0, 0.0),
                    uv: Vec2::new(1.0 / 3.0, 0.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 1.0, 0.0),
                    uv: Vec2::new(2.0 / 3.0, 0.0 / 2.0),
                },
                // Bottom vertices
                Vertex {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    uv: Vec2::new(2.0 / 3.0, 2.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 0.0, 0.0),
                    uv: Vec2::new(1.0 / 3.0, 2.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.0, 1.0),
                    uv: Vec2::new(2.0 / 3.0, 1.0 / 2.0),
                },
                Vertex {
                    position: Vec3::new(1.0, 0.0, 1.0),
                    uv: Vec2::new(1.0 / 3.0, 1.0 / 2.0),
                },
            ]),
            glow::STATIC_DRAW,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(
            0,
            3,
            glow::FLOAT,
            false,
            mem::size_of::<Vertex>() as _,
            offset_of!(Vertex, position) as _,
        );
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(
            1,
            2,
            glow::FLOAT,
            false,
            mem::size_of::<Vertex>() as _,
            offset_of!(Vertex, uv) as _,
        );

        let ebo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice::<[u8; 6], u8>(&[
                // Back face
                face_to_tri(&[1, 0, 3, 2]),
                // Front face
                face_to_tri(&[4, 5, 6, 7]),
                // Right face
                face_to_tri(&[9, 8, 11, 10]),
                // Left face
                face_to_tri(&[12, 13, 14, 15]),
                // Top face
                face_to_tri(&[16, 17, 18, 19]),
                // Bottom face
                face_to_tri(&[20, 21, 22, 23]),
            ]),
            glow::STATIC_DRAW,
        );

        let ibo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(ibo));

        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_f32(
            2,
            3,
            glow::FLOAT,
            false,
            mem::size_of::<Instance>() as _,
            offset_of!(Instance, position) as _,
        );
        gl.vertex_attrib_divisor(2, 1);
        gl.enable_vertex_attrib_array(3);
        gl.vertex_attrib_pointer_i32(
            3,
            1,
            glow::BYTE,
            mem::size_of::<Instance>() as _,
            offset_of!(Instance, texture) as _,
        );
        gl.vertex_attrib_divisor(3, 1);

        ChunkRenderer {
            vao,
            vbo,
            ebo,
            ibo,
            ibo_size: 0,
        }
    }

    pub unsafe fn update_blocks(&mut self, gl: &glow::Context, blocks: &[Block]) {
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.ibo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice::<_, u8>(
                blocks
                    .iter()
                    .map(|block| Instance {
                        position: block.position.map(|e| e as f32),
                        texture: block.id,
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            glow::STATIC_DRAW,
        );
        self.ibo_size = blocks.len();
    }

    pub unsafe fn draw(&self, gl: &glow::Context) {
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_elements_instanced(
            glow::TRIANGLES,
            36,
            glow::UNSIGNED_BYTE,
            0,
            self.ibo_size as _,
        );
    }
}
