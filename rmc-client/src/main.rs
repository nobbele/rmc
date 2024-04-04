use glow::HasContext;
use renderers::{ChunkRenderer, ScreenQuadRenderer};
use rmc_common::{game::InputState, Apply, Blend, Game};
use sdl2::{event::Event, keyboard::Keycode, mouse::MouseState};
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

        let mut gl =
            glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _);
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

        let mut game = Game::new();
        let mut prev_game = game.clone();

        let mut input_state = InputState {
            keys: HashSet::new(),
            mouse_state: event_pump.mouse_state(),
            blocked_mouse: imgui.io().want_capture_mouse,
            mouse_delta: Vec2::zero(),
        };
        let mut prev_input_state = input_state.clone();

        let mut chunk_renderer = ChunkRenderer::new(&gl);
        chunk_renderer.update_blocks(&gl, game.blocks.view());

        let projection =
            Mat4::<f32>::infinite_perspective_rh(120_f32.to_radians(), 4. / 3., 0.0001);

        const TICK_RATE: u32 = 20;
        const TICK_DELTA: f32 = 1.0 / TICK_RATE as f32;

        let mut last = sdl.timer().unwrap().performance_counter();
        let mut fps = 0.0;
        let mut running = true;
        let mut accumulator = 0.0;
        while running {
            let now = sdl.timer().unwrap().performance_counter();
            let dt = (now - last) as f32 / sdl.timer().unwrap().performance_frequency() as f32;
            accumulator += dt;
            last = now;

            fps = fps * (1.0 - 0.1) + (1.0 / dt) * 0.1;

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

            let mouse_position = Vec2::new(
                event_pump.mouse_state().x() as f32,
                event_pump.mouse_state().y() as f32,
            );
            let mouse_delta = if sdl.mouse().relative_mouse_mode() {
                sdl.mouse().warp_mouse_in_window(
                    &window,
                    window.size().0 as i32 / 2,
                    window.size().1 as i32 / 2,
                );
                (mouse_position
                    - Vec2::new(window.size().0 as f32 / 2.0, window.size().1 as f32 / 2.0))
                    / 50.
            } else {
                Vec2::zero()
            };

            let keys = event_pump
                .keyboard_state()
                .pressed_scancodes()
                .filter_map(Keycode::from_scancode)
                .collect::<HashSet<_>>()
                .apply(|keys| {
                    if imgui.io().want_capture_keyboard {
                        keys.clear();
                    }
                });

            input_state = InputState {
                keys: keys.clone(),
                mouse_state: event_pump.mouse_state(),
                blocked_mouse: imgui.io().want_capture_mouse,
                mouse_delta,
            };

            while accumulator >= TICK_DELTA {
                let mut new_game = game.clone();
                new_game.update(&game, &prev_input_state, &input_state);

                prev_game = game;
                game = new_game;

                prev_input_state = input_state.clone();

                input_state.keys.clear();
                input_state.mouse_state = MouseState::from_sdl_state(0);
                input_state.mouse_delta = Vec2::zero();

                if prev_game.blocks != game.blocks {
                    chunk_renderer.update_blocks(&gl, game.blocks.view());
                }

                accumulator -= TICK_DELTA;
            }

            imgui_platform.prepare_frame(&mut imgui, &window, &event_pump);
            let ui = imgui.new_frame();
            ui.window("Debug")
                .position([0.0, 0.0], imgui::Condition::Always)
                .always_auto_resize(true)
                .build(|| {
                    ui.text(format!("FPS: {:.0}", fps));
                    ui.text(format!("Position: {:.2}", game.camera.position));
                    ui.text(format!(
                        "Orientation: {:.2} {:.2} ({:.2})",
                        game.camera.yaw.0,
                        game.camera.pitch.0,
                        game.camera.look_at()
                    ));
                });

            let alpha = accumulator / TICK_DELTA;
            let interpolated_game = prev_game.blend(&game, alpha);

            let model = Mat4::<f32>::identity();
            let mvp = projection * interpolated_game.camera.to_matrix() * model;

            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            gl.use_program(Some(program));
            gl.uniform_matrix_4_f32_slice(
                Some(&gl.get_uniform_location(program, "uniform_Mvp").unwrap()),
                false,
                mvp.as_col_slice(),
            );
            let uniform_highlighted = interpolated_game
                .look_at_raycast
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
            chunk_renderer.draw(&gl);

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
