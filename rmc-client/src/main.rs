use std::mem;

use glow::HasContext;
use vek::Vec3;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Vertex {
    position: Vec3<f32>,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

fn face_to_tri(v: &[u8; 4]) -> [u8; 6] {
    [v[0], v[1], v[3], v[3], v[2], v[0]]
}

fn main() {
    unsafe {
        let sdl = sdl2::init().unwrap();
        let video = sdl.video().unwrap();
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(4, 6);
        let window = video
            .window("RMC", 1024, 768)
            .opengl()
            .resizable()
            .build()
            .unwrap();
        let _sdl2_gl_context = window.gl_create_context().unwrap();
        let gl = glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _);
        let mut event_loop = sdl.event_pump().unwrap();

        // gl.enable(glow::DEBUG_OUTPUT);
        // gl.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS);
        // gl.debug_message_callback(|_ty, _id, _severity, _length, message| println!("{}", message));

        gl.clear_color(0.1, 0.2, 0.3, 1.0);

        let program = create_shader(&gl);
        let vao = create_cube(&gl);

        let mut running = true;
        while running {
            {
                for event in event_loop.poll_iter() {
                    match event {
                        sdl2::event::Event::Quit { .. } => running = false,
                        _ => {}
                    }
                }
            }

            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.use_program(Some(program));
            gl.bind_vertex_array(Some(vao));
            gl.draw_elements(glow::TRIANGLES, 6 * 6, glow::UNSIGNED_BYTE, 0);

            window.gl_swap_window();
        }
    }
}

unsafe fn create_shader(gl: &glow::Context) -> glow::Program {
    let program = gl.create_program().expect("Cannot create program");

    let shader_sources = [
        (
            glow::VERTEX_SHADER,
            r#"
#version 460 core

layout(location = 0) in vec3 in_position;

out vec3 vert_Position;

void main() {
    vert_Position = in_position;
    gl_Position = vec4(in_position, 1.0);
}
            "#,
        ),
        (
            glow::FRAGMENT_SHADER,
            r#"
#version 460 core

in vec3 vert_Position;

out vec4 frag_Color;

void main() {
    frag_Color = vec4(vert_Position, 1.0);
}
            "#,
        ),
    ];

    let mut shaders = Vec::with_capacity(shader_sources.len());
    for (shader_type, shader_source) in shader_sources.iter() {
        let shader = gl
            .create_shader(*shader_type)
            .expect("Cannot create shader");
        gl.shader_source(shader, shader_source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            panic!("{}", gl.get_shader_info_log(shader));
        }
        gl.attach_shader(program, shader);
        shaders.push(shader);
    }

    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("{}", gl.get_program_info_log(program));
    }

    for shader in shaders {
        gl.detach_shader(program, shader);
        gl.delete_shader(shader);
    }

    program
}

unsafe fn create_cube(gl: &glow::Context) -> glow::VertexArray {
    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(vao));

    let vbo = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    gl.buffer_data_u8_slice(
        glow::ARRAY_BUFFER,
        bytemuck::cast_slice(&[
            // Front vertices
            Vertex {
                position: Vec3::new(0.0, 0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(1.0, 0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(0.0, 1.0, 1.0),
            },
            Vertex {
                position: Vec3::new(1.0, 1.0, 1.0),
            },
            // Back vertices
            Vertex {
                position: Vec3::new(0.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.0, 1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(1.0, 1.0, 0.0),
            },
        ]),
        glow::STATIC_DRAW,
    );

    // gl.bind_vertex_buffer(
    //     0,
    //     Some(vbo),
    //     0,
    //     mem::size_of::<Vertex>().try_into().unwrap(),
    // );

    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);

    //gl.vertex_attrib_format_f32(0, 3, glow::FLOAT, false, 0);
    //gl.vertex_attrib_binding(0, 0);

    let ebo = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
    gl.buffer_data_u8_slice(
        glow::ELEMENT_ARRAY_BUFFER,
        bytemuck::cast_slice::<[u8; 6], u8>(&[
            // Front face
            face_to_tri(&[0, 1, 2, 3]),
            // Right face
            face_to_tri(&[1, 5, 3, 7]),
            // Top face
            face_to_tri(&[2, 3, 6, 7]),
            // Left face
            face_to_tri(&[4, 0, 6, 2]),
            // Bottom face
            face_to_tri(&[1, 0, 5, 4]),
            // Back face
            face_to_tri(&[5, 4, 7, 6]),
        ]),
        glow::STATIC_DRAW,
    );

    vao
}
