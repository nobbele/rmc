use glow::HasContext;
use renderers::ScreenQuadRenderer;
use rmc_common::{
    game::{TICK_DELTA, TICK_SPEED},
    input::{ButtonBuffer, ButtonStateEvent, InputState, KeyboardEvent, MouseButtonEvent},
    world::CHUNK_SIZE,
    Blend, Game, LookBack,
};
use sdl2::{event::Event, keyboard::Keycode};
use std::{collections::HashMap, process::exit, time::Instant};
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

        // sdl.video().unwrap().gl_set_swap_interval(0).unwrap();

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

        let mut input_state = InputState {
            keys: HashMap::new(),
            mouse_buttons: HashMap::new(),
            mouse_delta: Vec2::zero(),
        };

        let mut game_renderer = GameRenderer::new(&gl);
        for (pos, chunk) in game.curr.world.chunks_iter() {
            game_renderer.update_chunk(
                &gl,
                game.curr
                    .world
                    .chunk_index(pos * CHUNK_SIZE as i32)
                    .unwrap()
                    .into_tuple(),
                pos,
                &chunk,
            );
        }

        let mut sdl_time = LookBack::new_identical(sdl.timer().unwrap().performance_counter());

        let mut keyboard_buffer = ButtonBuffer::new();
        let mut mouse_button_buffer = ButtonBuffer::new();

        let mut running = true;
        let mut accumulator = 0.0;
        while running {
            sdl_time.push(sdl.timer().unwrap().performance_counter());
            let dt = (sdl_time.curr - sdl_time.prev) as f32
                / sdl.timer().unwrap().performance_frequency() as f32;
            accumulator += dt * TICK_SPEED;

            let fps = 1.0 / dt;

            for event in event_pump.poll_iter() {
                imgui_platform.handle_event(&mut imgui, &event);

                if !imgui.io().want_capture_keyboard && !imgui.io().want_capture_mouse {
                    match &event {
                        Event::KeyDown {
                            keycode: Some(keycode),
                            ..
                        } => {
                            keyboard_buffer.push(KeyboardEvent {
                                key: *keycode,
                                state: ButtonStateEvent::Press,
                            });
                        }
                        Event::MouseButtonDown { mouse_btn, .. } => {
                            mouse_button_buffer.push(MouseButtonEvent {
                                button: *mouse_btn,
                                state: ButtonStateEvent::Press,
                            });
                        }
                        Event::KeyUp {
                            keycode: Some(keycode),
                            ..
                        } => {
                            keyboard_buffer.push(KeyboardEvent {
                                key: *keycode,
                                state: ButtonStateEvent::Release,
                            });
                        }
                        Event::MouseButtonUp { mouse_btn, .. } => {
                            mouse_button_buffer.push(MouseButtonEvent {
                                button: *mouse_btn,
                                state: ButtonStateEvent::Release,
                            });
                        }
                        _ => {}
                    }
                }

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
            input_state.mouse_delta += if sdl.mouse().relative_mouse_mode() {
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

            imgui_platform.prepare_frame(&mut imgui, &window, &event_pump);
            let ui = imgui.new_frame();

            while accumulator >= TICK_DELTA {
                let start_of_tick = Instant::now();

                input_state.update_held_status();
                for keycode in keyboard_buffer.keys().collect::<Vec<_>>() {
                    if let Some(event) = keyboard_buffer.pull(keycode) {
                        input_state.push_keyboard_event(event);
                    }
                }
                for mouse_button in mouse_button_buffer.keys().collect::<Vec<_>>() {
                    if let Some(event) = mouse_button_buffer.pull(mouse_button) {
                        input_state.push_mouse_button_event(event);
                    }
                }

                game.push_from(|_prev, game| game.update(&input_state));

                input_state.mouse_delta = Vec2::zero();

                for (pos, chunk) in game.curr.world.chunks_iter() {
                    if game
                        .prev
                        .world
                        .chunk_at(pos * CHUNK_SIZE as i32)
                        .unwrap()
                        .blocks
                        .view()
                        != game
                            .curr
                            .world
                            .chunk_at(pos * CHUNK_SIZE as i32)
                            .unwrap()
                            .blocks
                            .view()
                    {
                        game_renderer.update_chunk(
                            &gl,
                            game.curr
                                .world
                                .chunk_index(pos * CHUNK_SIZE as i32)
                                .unwrap()
                                .into_tuple(),
                            pos,
                            &chunk,
                        );
                    }
                }

                accumulator -= TICK_DELTA;

                let end_of_tick = Instant::now();
                if end_of_tick.duration_since(start_of_tick).as_secs_f32() > 1.0 {
                    println!("Game is running too slow!");
                    exit(-1);
                }
            }

            ui.window("Debug")
                .position([0.0, 0.0], imgui::Condition::Always)
                .always_auto_resize(true)
                .build(|| {
                    ui.text(format!("FPS: {:.0} ({:.0}ms)", fps, (1.0 / fps) * 1000.0));
                    ui.text(format!(
                        "Updates: {} / {}",
                        game.curr.block_update_count,
                        game.curr.dirty_blocks.len()
                    ));
                    ui.text(format!("Position: {:.2}", game.curr.camera.position));
                    ui.text(format!(
                        "Block Position: {:.2}",
                        game.curr.block_coordinate()
                    ));
                    ui.text(format!(
                        "Chunk Position: {:.2}",
                        game.curr.chunk_coordinate()
                    ));
                    ui.text(format!(
                        "Highlight: {:?} ({:?}) (light: {})",
                        game.curr
                            .look_at_raycast
                            .map(|r| r.position)
                            .unwrap_or_default(),
                        game.curr
                            .look_at_raycast
                            .map(|r| game.curr.world.get_block(r.position))
                            .flatten()
                            .unwrap_or_default(),
                        game.curr
                            .look_at_raycast
                            .map(|r| game.curr.world.get_block(r.position + r.normal.as_()))
                            .flatten()
                            .map(|b| b.light)
                            .unwrap_or_default(),
                    ));
                    ui.text(format!(
                        "Orientation: {:.2} {:.2} ({:.2})",
                        game.curr.camera.yaw.0,
                        game.curr.camera.pitch.0,
                        game.curr.camera.look_at()
                    ));
                    ui.text(format!("On Ground: {}", game.curr.on_ground));
                });

            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            game_renderer.draw(&gl, &game.prev.blend(&game.curr, accumulator / TICK_DELTA));
            screen_quad_renderer.draw(&gl, crosshair_texture);
            imgui_renderer
                .render(&gl, &imgui_textures, imgui.render())
                .unwrap();

            window.gl_swap_window();

            if input_state.get_key(Keycode::K).pressed() {
                sdl.timer().unwrap().delay(100);
            }
        }
    }
}
