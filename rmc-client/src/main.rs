use glow::HasContext;
use renderers::ScreenQuadRenderer;
use rmc_common::{game::InputState, lerp, Apply, Blend, Game, LookBack};
use sdl2::{event::Event, keyboard::Keycode, mouse::MouseState};
use std::collections::HashSet;
use texture::{load_texture, DataSource};
use vek::Vec2;

use crate::renderers::GameRenderer;

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
        let screen_quad_renderer = ScreenQuadRenderer::new(&gl);

        let mut game = LookBack::new_identical(Game::new());

        let mut input_state = LookBack::new_identical(InputState {
            keys: HashSet::new(),
            mouse_state: event_pump.mouse_state(),
            blocked_mouse: imgui.io().want_capture_mouse,
            mouse_delta: Vec2::zero(),
        });

        let mut game_renderer = GameRenderer::new(&gl);
        game_renderer
            .chunk_renderer
            .update_blocks(&gl, game.curr.blocks.view());

        const TICK_RATE: u32 = 20;
        const TICK_DELTA: f32 = 1.0 / TICK_RATE as f32;

        let mut sdl_time = LookBack::new_identical(sdl.timer().unwrap().performance_counter());

        let mut fps = 0.0;
        let mut running = true;
        let mut accumulator = 0.0;
        while running {
            sdl_time.push(sdl.timer().unwrap().performance_counter());
            let dt = (sdl_time.curr - sdl_time.prev) as f32
                / sdl.timer().unwrap().performance_frequency() as f32;
            accumulator += dt;

            fps = lerp(fps, 1.0 / dt, 0.1);

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

            input_state.push(InputState {
                keys: keys.clone(),
                mouse_state: event_pump.mouse_state(),
                blocked_mouse: imgui.io().want_capture_mouse,
                mouse_delta,
            });

            // TODO Fix input handling being skipped.
            while accumulator >= TICK_DELTA {
                game.push_from(|_prev, game| game.update(&input_state.prev, &input_state.curr));

                input_state.push_from(|_prev, input_state| {
                    input_state.keys.clear();
                    input_state.mouse_state = MouseState::from_sdl_state(0);
                    input_state.mouse_delta = Vec2::zero();
                });

                if game.prev.blocks != game.curr.blocks {
                    game_renderer
                        .chunk_renderer
                        .update_blocks(&gl, game.curr.blocks.view());
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
                    ui.text(format!("Position: {:.2}", game.curr.camera.position));
                    ui.text(format!(
                        "Orientation: {:.2} {:.2} ({:.2})",
                        game.curr.camera.yaw.0,
                        game.curr.camera.pitch.0,
                        game.curr.camera.look_at()
                    ));
                });

            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            game_renderer.draw(&gl, &game.prev.blend(&game.curr, accumulator / TICK_DELTA));
            screen_quad_renderer.draw(&gl, crosshair_texture);
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
