use glow::HasContext;
use ndarray::Array3;
use renderers::{ChunkRenderer, ScreenQuadRenderer};
use rmc_common::{
    world::{raycast, Block},
    Camera,
};
use sdl2::{event::Event, keyboard::Keycode};
use shader::create_shader;
use std::collections::HashSet;
use texture::{load_array_texture, load_texture, DataSource};
use vek::{Mat3, Mat4, Vec2, Vec3};

pub mod renderers;
pub mod shader;
pub mod texture;

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
        gl.enable(glow::CULL_FACE);
        gl.clear_color(0.1, 0.2, 0.3, 1.0);

        let crosshair_texture = load_texture(
            &gl,
            DataSource::Inline(include_bytes!("../textures/crosshair.png")),
        );
        let render_crosshair = ScreenQuadRenderer::new(&gl);

        let block_array_texture = load_array_texture(
            &gl,
            &[
                DataSource::Inline(include_bytes!("../textures/test.png")),
                DataSource::Inline(include_bytes!("../textures/grass.png")),
            ],
        );

        let program = create_shader(
            &gl,
            include_str!("../shaders/cube.vert"),
            include_str!("../shaders/cube.frag"),
        );

        let screen_program = create_shader(
            &gl,
            include_str!("../shaders/screen.vert"),
            include_str!("../shaders/screen.frag"),
        );

        let mut blocks: Array3<Option<Block>> = Array3::default((16, 16, 16));
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    blocks[(x, y, z)] = Some(Block {
                        position: Vec3::new(x as _, y as _, z as _),
                        id: 1,
                    });
                }
            }
        }

        let mut render_chunk = ChunkRenderer::new(&gl);
        render_chunk.update_blocks(&gl, blocks.view());

        let projection = Mat4::<f32>::infinite_perspective_rh(120_f32.to_radians(), 4. / 3., 0.1);

        let mut camera = Camera {
            position: Vec3::new(8.0, 18.0, 8.0),
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
                    ui.text(format!(
                        "Orientation: {:.2} {:.2} ({:.2})",
                        camera.yaw,
                        camera.pitch,
                        camera.look_at()
                    ));
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
            camera.move_forward(fwd_bck as f32 * 3.0 * dt);
            camera.move_right(rgh_lft as f32 * 3.0 * dt);
            camera.move_up(up_down as f32 * 3.0 * dt);

            let highlighted = raycast(camera.position, camera.look_at(), 7.5, blocks.view());
            if let Some(highlighted) = highlighted {
                if !imgui.io().want_capture_mouse && mouse_state.left() && !prev_mouse_state.left()
                {
                    blocks[highlighted.position.map(|e| e as _).into_tuple()] = None;
                    render_chunk.update_blocks(&gl, blocks.view());
                }
            }

            if let Some(highlighted) = highlighted {
                if !imgui.io().want_capture_mouse
                    && mouse_state.right()
                    && !prev_mouse_state.right()
                {
                    let position = highlighted.position + highlighted.normal.map(|e| e as i32);

                    if let Some(entry) = blocks.get_mut(position.map(|e| e as _).into_tuple()) {
                        *entry = Some(Block { position, id: 0 });
                        render_chunk.update_blocks(&gl, blocks.view());
                    }
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

            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(block_array_texture));
            render_chunk.draw(&gl);

            let size = Vec2::new(48.0, 48.0);
            let screen_mat = Mat3::<f32>::scaling_3d((size / Vec2::new(1024.0, 768.0)).with_z(1.0))
                * Mat3::<f32>::translation_2d(Vec2::new(-1.0, -1.0))
                * Mat3::<f32>::scaling_3d(Vec2::broadcast(2.0).with_z(1.0));

            gl.use_program(Some(screen_program));
            gl.uniform_matrix_3_f32_slice(
                Some(
                    &gl.get_uniform_location(screen_program, "uniform_Mat")
                        .unwrap(),
                ),
                false,
                screen_mat.as_col_slice(),
            );

            gl.bind_texture(glow::TEXTURE_2D, Some(crosshair_texture));
            render_crosshair.draw(&gl);

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
