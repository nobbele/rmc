use bytemuck::offset_of;
use glow::HasContext;
use rmc_common::{
    world::{raycast, Block},
    Camera,
};
use sdl2::{event::Event, keyboard::Keycode};
use shader::create_shader;
use std::{collections::HashSet, mem};
use vek::{Mat4, Vec2, Vec3};

mod shader;

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
struct Vertex {
    position: Vec3<f32>,
    uv: Vec2<f32>,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Instance {
    position: Vec3<f32>,
}

unsafe impl bytemuck::Pod for Instance {}
unsafe impl bytemuck::Zeroable for Instance {}

fn face_to_tri(v: &[u8; 4]) -> [u8; 6] {
    [v[0], v[1], v[3], v[3], v[2], v[0]]
}

fn main() {
    unsafe {
        let sdl = sdl2::init().unwrap();
        let video = sdl.video().unwrap();
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);
        let window = video
            .window("RMC", 1024, 768)
            .opengl()
            .resizable()
            .build()
            .unwrap();
        let window_gl_context = window.gl_create_context().unwrap();
        window.gl_make_current(&window_gl_context).unwrap();
        window.subsystem().gl_set_swap_interval(1).unwrap();

        let gl = glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _);
        let mut event_pump = sdl.event_pump().unwrap();

        gl.enable(glow::DEBUG_OUTPUT);
        gl.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS);
        gl.debug_message_callback(|_ty, _id, _severity, _length, message| println!("{}", message));
        gl.debug_message_control(
            glow::DONT_CARE,
            glow::DONT_CARE,
            glow::DEBUG_SEVERITY_NOTIFICATION,
            &[],
            false,
        );

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);
        imgui.set_log_filename(None);
        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

        let mut imgui_platform = imgui_sdl2_support::SdlPlatform::init(&mut imgui);
        let mut imgui_textures = imgui::Textures::<glow::Texture>::default();
        let mut imgui_renderer =
            imgui_glow_renderer::Renderer::initialize(&gl, &mut imgui, &mut imgui_textures, false)
                .unwrap();

        gl.enable(glow::DEPTH_TEST);
        gl.clear_color(0.1, 0.2, 0.3, 1.0);

        let test_block_texture = {
            let test_block_image =
                image::load_from_memory(include_bytes!("../textures/test-block.png"))
                    .unwrap()
                    .to_rgb8();
            let test_block_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(test_block_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as _,
                test_block_image.width() as _,
                test_block_image.height() as _,
                0,
                glow::RGB as _,
                glow::UNSIGNED_BYTE,
                Some(test_block_image.into_iter().as_slice()),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
            test_block_texture
        };

        let program = create_shader(&gl);

        let mut blocks = std::iter::repeat(())
            .enumerate()
            .map(|(idx, _)| Block {
                position: Vec3::new(
                    (idx % 16) as i32 - 8,
                    (idx / (16 * 16)) as i32 - 16 + 1,
                    ((idx % (16 * 16)) / 16) as i32 - 8,
                ),
            })
            .take(16 * 16 * 16)
            .collect::<Vec<_>>();

        let (instance_buffer, vao) = create_cube(&gl, &blocks);

        let projection = Mat4::<f32>::infinite_perspective_rh(120_f32.to_radians(), 4. / 3., 0.1);

        let mut camera = Camera {
            position: Vec3::new(0.0, 2.0, 0.0),
            pitch: 0.0,
            yaw: 0.0,
        };

        let mut last = sdl.timer().unwrap().performance_counter();
        let mut prev_mouse_state = event_pump.mouse_state();
        let mut fps = 0.0;
        let mut running = true;
        while running {
            let now = sdl.timer().unwrap().performance_counter();
            let dt = (now - last) as f32 / sdl.timer().unwrap().performance_frequency() as f32;
            last = now;

            fps = fps * (1.0 - 0.1) + (1.0 / dt) * 0.1;

            imgui_platform.prepare_frame(&mut imgui, &window, &event_pump);
            let ui = imgui.new_frame();
            ui.window("Debug")
                .position([0.0, 0.0], imgui::Condition::Always)
                .always_auto_resize(true)
                .build(|| {
                    ui.text(format!("FPS: {:.0}", fps));
                    ui.text(format!("Position: {:.2}", camera.position));
                });

            for event in event_pump.poll_iter() {
                imgui_platform.handle_event(&mut imgui, &event);
                match event {
                    Event::Quit { .. } => running = false,
                    Event::MouseButtonDown { .. } if !imgui.io().want_capture_mouse => {
                        sdl.mouse().set_relative_mouse_mode(true)
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => sdl.mouse().set_relative_mouse_mode(false),
                    _ => {}
                }
            }

            let mouse_state = event_pump.mouse_state();

            let mouse_position = Vec2::new(
                event_pump.mouse_state().x() as f32,
                event_pump.mouse_state().y() as f32,
            );
            let mouse_movement = if sdl.mouse().relative_mouse_mode() {
                sdl.mouse().warp_mouse_in_window(
                    &window,
                    window.size().0 as i32 / 2,
                    window.size().1 as i32 / 2,
                );
                (mouse_position
                    - Vec2::new(window.size().0 as f32 / 2.0, window.size().1 as f32 / 2.0))
                    / 100.
            } else {
                Vec2::zero()
            };

            let mut keys: HashSet<_> = event_pump
                .keyboard_state()
                .pressed_scancodes()
                .filter_map(Keycode::from_scancode)
                .collect();
            if imgui.io().want_capture_keyboard {
                keys.clear();
            }

            let fwd_bck = keys.contains(&Keycode::W) as i8 - keys.contains(&Keycode::S) as i8;
            let rgh_lft = keys.contains(&Keycode::D) as i8 - keys.contains(&Keycode::A) as i8;
            let up_down =
                keys.contains(&Keycode::Space) as i8 - keys.contains(&Keycode::LShift) as i8;

            camera.rotate_horizontal(mouse_movement.x);
            camera.rotate_vertical(mouse_movement.y);
            camera.move_forward(fwd_bck as f32 * 2.0 * dt);
            camera.move_right(rgh_lft as f32 * 2.0 * dt);
            camera.move_up(up_down as f32 * 2.0 * dt);

            let highlighted = raycast(camera.position, camera.look_at(), 3.5, &blocks);
            if let Some(highlighted) = highlighted {
                if !imgui.io().want_capture_mouse && mouse_state.left() && !prev_mouse_state.left()
                {
                    blocks.retain(|b| b.position != highlighted.position);

                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(instance_buffer));
                    gl.buffer_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        bytemuck::cast_slice::<_, u8>(
                            blocks
                                .iter()
                                .map(|block| Instance {
                                    position: block.position.map(|e| e as f32),
                                })
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ),
                        glow::STATIC_DRAW,
                    );
                }
            }

            if let Some(highlighted) = highlighted {
                if !imgui.io().want_capture_mouse
                    && mouse_state.right()
                    && !prev_mouse_state.right()
                {
                    let block = Block {
                        position: highlighted.position + highlighted.face.map(|e| e as i32),
                    };
                    blocks.push(block);

                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(instance_buffer));
                    gl.buffer_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        bytemuck::cast_slice::<_, u8>(
                            blocks
                                .iter()
                                .map(|block| Instance {
                                    position: block.position.map(|e| e as f32),
                                })
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ),
                        glow::STATIC_DRAW,
                    );
                }
            }

            prev_mouse_state = mouse_state;

            let model = Mat4::<f32>::identity();
            let mvp = projection * camera.to_matrix() * model;

            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            gl.use_program(Some(program));
            gl.uniform_matrix_4_f32_slice(
                Some(&gl.get_uniform_location(program, "uniform_Mvp").unwrap()),
                false,
                mvp.as_col_slice(),
            );
            let uniform_highlighted = highlighted
                .map(|v| v.position.map(|e| e as f32))
                .unwrap_or(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
            gl.uniform_3_f32(
                Some(
                    &gl.get_uniform_location(program, "uniform_Highlighted")
                        .unwrap(),
                ),
                uniform_highlighted.x,
                uniform_highlighted.y,
                uniform_highlighted.z,
            );

            gl.bind_texture(glow::TEXTURE_2D, Some(test_block_texture));
            gl.bind_vertex_array(Some(vao));
            gl.draw_elements_instanced(
                glow::TRIANGLES,
                6 * 6,
                glow::UNSIGNED_BYTE,
                0,
                blocks.len() as _,
            );

            // imgui.io().want_capture_mouse
            imgui_renderer
                .render(&gl, &imgui_textures, imgui.render())
                .unwrap();

            window.gl_swap_window();

            if keys.contains(&Keycode::K) {
                sdl.timer().unwrap().delay(100);
            }
        }
    }
}

unsafe fn create_cube(gl: &glow::Context, blocks: &[Block]) -> (glow::Buffer, glow::VertexArray) {
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
            face_to_tri(&[0, 1, 2, 3]),
            // Front face
            face_to_tri(&[4, 5, 6, 7]),
            // Right face
            face_to_tri(&[8, 9, 10, 11]),
            // Left face
            face_to_tri(&[12, 13, 14, 15]),
            // Top face
            face_to_tri(&[16, 17, 18, 19]),
            // Bottom face
            face_to_tri(&[20, 21, 22, 23]),
        ]),
        glow::STATIC_DRAW,
    );

    let instance_buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(instance_buffer));
    gl.buffer_data_u8_slice(
        glow::ARRAY_BUFFER,
        bytemuck::cast_slice::<_, u8>(
            blocks
                .iter()
                .map(|block| Instance {
                    position: block.position.map(|e| e as f32),
                })
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        glow::STATIC_DRAW,
    );

    gl.enable_vertex_attrib_array(2);
    gl.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, false, 0, 0);
    gl.vertex_attrib_divisor(2, 1);

    (instance_buffer, vao)
}
