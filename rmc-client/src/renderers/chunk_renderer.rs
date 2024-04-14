use std::mem;

use bytemuck::offset_of;
use glow::HasContext;
use ndarray::ArrayView3;
use rmc_common::world::{face_to_normal, Block};
use vek::{Vec2, Vec3};

/*
push(generate_face(
            Vec3::new(1.0, 0.0, 0.0),
            Vec2::new(2.0 / 3.0, 0.0),
            0,
        ));
        push(generate_face(
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(1.0 / 3.0, 0.0),
            1,
        ));
        push(generate_face(
            Vec3::new(0.0, 0.0, 1.0),
            Vec2::new(0.0, 0.0),
            2,
        ));
        push(generate_face(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec2::new(2.0 / 3.0, 0.5),
            3,
        ));
        push(generate_face(
            Vec3::new(0.0, -1.0, 0.0),
            Vec2::new(1.0 / 3.0, 0.5),
            4,
        ));
        push(generate_face(
            Vec3::new(0.0, 0.0, -1.0),
            Vec2::new(0.0, 0.5),
            5,
        )); */

#[derive(Debug, Default, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3<f32>,
    pub uv: Vec2<f32>,
    pub face: u8,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct Instance {
    pub position: Vec3<f32>,
    pub texture: u8,
    pub light: [u8; 6],
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
    ib: glow::Buffer,
    ib_size: usize,
}

fn generate_face(normal: Vec3<f32>, texture_origin: Vec2<f32>, face: u8) -> [Vertex; 4] {
    let (card, card_cross) = if normal.x == 0.0 {
        (
            Vec3::unit_x() * normal.sum(),
            normal.cross(Vec3::unit_x() * normal.sum()),
        )
    } else {
        (
            -Vec3::unit_z() * normal.sum(),
            (Vec3::unit_z() * normal.sum()).cross(normal),
        )
    };
    [
        Vertex {
            position: -card - card_cross,
            uv: Vec2::zero(),
            face,
        },
        Vertex {
            position: card - card_cross,
            uv: Vec2::zero(),
            face,
        },
        Vertex {
            position: -card + card_cross,
            uv: Vec2::zero(),
            face,
        },
        Vertex {
            position: card + card_cross,
            uv: Vec2::zero(),
            face,
        },
    ]
    .map(|e| {
        let position = (normal + e.position).map(|pe| (pe + 1.0) / 2.0);
        let uv_offset = Vec2::new(
            if card.sum() == 1.0 {
                (position * card).magnitude()
            } else {
                1.0 - (position * card).magnitude()
            },
            if card_cross.sum() == 1.0 {
                1.0 - (position * card_cross).magnitude()
            } else if card_cross.sum() == -1.0 {
                (position * card_cross).magnitude()
            } else {
                unreachable!()
            },
        );
        Vertex {
            position,
            uv: texture_origin + uv_offset / Vec2::new(3.0, 2.0),
            ..e
        }
    })
}

fn get_block_light(blocks: ArrayView3<Block>, pos: Vec3<i32>) -> u8 {
    if pos.into_iter().all(|e| e >= 0) {
        if let Some(block) = blocks.get(pos.map(|e| e as usize).into_tuple()) {
            return block.light;
        }
    }

    15
}

impl ChunkRenderer {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u8> = Vec::new();

        let mut push = |vs: [Vertex; 4]| {
            indices.extend([0, 1, 2, 3, 2, 1].map(|i| i + vertices.len() as u8));
            vertices.extend_from_slice(&vs);
        };

        push(generate_face(
            Vec3::new(1.0, 0.0, 0.0),
            Vec2::new(2.0 / 3.0, 0.0),
            0,
        ));
        push(generate_face(
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(1.0 / 3.0, 0.0),
            1,
        ));
        push(generate_face(
            Vec3::new(0.0, 0.0, 1.0),
            Vec2::new(0.0, 0.0),
            2,
        ));
        push(generate_face(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec2::new(2.0 / 3.0, 0.5),
            3,
        ));
        push(generate_face(
            Vec3::new(0.0, -1.0, 0.0),
            Vec2::new(1.0 / 3.0, 0.5),
            4,
        ));
        push(generate_face(
            Vec3::new(0.0, 0.0, -1.0),
            Vec2::new(0.0, 0.5),
            5,
        ));

        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(vertices.as_slice()),
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
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_i32(
            2,
            1,
            glow::UNSIGNED_BYTE,
            mem::size_of::<Vertex>() as _,
            offset_of!(Vertex, face) as _,
        );

        let ebo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            indices.as_slice(),
            glow::STATIC_DRAW,
        );

        let ib = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(ib));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &[], glow::STATIC_DRAW);

        gl.enable_vertex_attrib_array(3);
        gl.vertex_attrib_pointer_f32(
            3,
            3,
            glow::FLOAT,
            false,
            mem::size_of::<Instance>() as _,
            offset_of!(Instance, position) as _,
        );
        gl.vertex_attrib_divisor(3, 1);
        gl.enable_vertex_attrib_array(4);
        gl.vertex_attrib_pointer_i32(
            4,
            1,
            glow::UNSIGNED_BYTE,
            mem::size_of::<Instance>() as _,
            offset_of!(Instance, texture) as _,
        );
        gl.vertex_attrib_divisor(4, 1);
        gl.enable_vertex_attrib_array(5);
        gl.vertex_attrib_pointer_i32(
            5,
            4,
            glow::UNSIGNED_BYTE,
            mem::size_of::<Instance>() as _,
            offset_of!(Instance, light) as _,
        );
        gl.vertex_attrib_divisor(5, 1);
        gl.enable_vertex_attrib_array(6);
        gl.vertex_attrib_pointer_i32(
            6,
            2,
            glow::UNSIGNED_BYTE,
            mem::size_of::<Instance>() as _,
            offset_of!(Instance, light) as i32 + 4,
        );
        gl.vertex_attrib_divisor(6, 1);

        ChunkRenderer {
            vao,
            vbo,
            ebo,
            ib,
            ib_size: 0,
        }
    }

    pub unsafe fn update_blocks(&mut self, gl: &glow::Context, blocks: ArrayView3<Block>) {
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.ib));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice::<_, u8>(
                blocks
                    .indexed_iter()
                    .map(|(idx, block)| (idx, if block.id == 0 { None } else { Some(block) }))
                    .filter_map(|(pos, block)| {
                        block.map(|b| (Vec3::new(pos.0 as i32, pos.1 as i32, pos.2 as i32), b))
                    })
                    .map(|(pos, block)| Instance {
                        position: pos.as_(),
                        texture: block.id - 1,
                        light: [0, 1, 2, 3, 4, 5]
                            .map(|face| get_block_light(blocks, pos + face_to_normal(face))),
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            glow::STATIC_DRAW,
        );
        self.ib_size = blocks.len();
    }

    pub unsafe fn draw(&self, gl: &glow::Context) {
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_elements_instanced(
            glow::TRIANGLES,
            36,
            glow::UNSIGNED_BYTE,
            0,
            self.ib_size as _,
        );
    }
}
