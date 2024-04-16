#![feature(more_float_constants)]
use glow::HasContext;
use renderers::{screen_quad_renderer::DrawParams, IsometricBlockRenderer, ScreenQuadRenderer};
use rmc_common::{
    game::{BlockOrItem, TICK_DELTA, TICK_SPEED},
    input::{ButtonBuffer, ButtonStateEvent, InputState, KeyboardEvent, MouseButtonEvent},
    world::CHUNK_SIZE,
    Blend, Game, LookBack,
};
use sdl2::{event::Event, keyboard::Keycode};
use std::{collections::HashMap, process::exit, time::Instant};
use texture::{load_image, DataSource};
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

        // gl.enable(glow::CULL_FACE);
        gl.clear_color(0.1, 0.2, 0.3, 1.0);

        let crosshair_image = load_image(
            &gl,
            DataSource::Inline(include_bytes!("../textures/crosshair.png")),
        );
        let slot_image = load_image(
            &gl,
            DataSource::Inline(include_bytes!("../textures/slot.png")),
        );
        let active_slot_image = load_image(
            &gl,
            DataSource::Inline(include_bytes!("../textures/active-slot.png")),
        );

        let screen_quad_renderer = ScreenQuadRenderer::new(&gl);
        let isometric_block_renderer = IsometricBlockRenderer::new(&gl);

        let mut game = LookBack::new_identical(Game::new());

        let mut input_state = InputState {
            keys: HashMap::new(),
            mouse_buttons: HashMap::new(),
            mouse_delta: Vec2::zero(),
            scroll_delta: 0,
        };

        let mut game_renderer = GameRenderer::new(&gl, game.curr.world.shape);
        for (pos, chunk) in game.curr.world.chunks_iter() {
            game_renderer.update_chunk(
                &gl,
                game.curr.world.chunk_to_index(pos).unwrap().into_tuple(),
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
                        &Event::KeyDown {
                            keycode: Some(keycode),
                            ..
                        } => {
                            keyboard_buffer.push(KeyboardEvent {
                                key: keycode,
                                state: ButtonStateEvent::Press,
                            });
                        }
                        &Event::MouseButtonDown { mouse_btn, .. } => {
                            mouse_button_buffer.push(MouseButtonEvent {
                                button: mouse_btn,
                                state: ButtonStateEvent::Press,
                            });
                        }
                        &Event::KeyUp {
                            keycode: Some(keycode),
                            ..
                        } => {
                            keyboard_buffer.push(KeyboardEvent {
                                key: keycode,
                                state: ButtonStateEvent::Release,
                            });
                        }
                        &Event::MouseButtonUp { mouse_btn, .. } => {
                            mouse_button_buffer.push(MouseButtonEvent {
                                button: mouse_btn,
                                state: ButtonStateEvent::Release,
                            });
                        }
                        &Event::MouseWheel { y, .. } => {
                            input_state.scroll_delta += y;
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
                input_state.scroll_delta = 0;

                if game.curr.world.origin() != game.prev.world.origin() {
                    for (pos, _chunk) in game.prev.world.chunks_iter() {
                        game_renderer.clear_chunk(
                            &gl,
                            game.prev.world.chunk_to_index(pos).unwrap().into_tuple(),
                        );
                    }

                    for (pos, chunk) in game.curr.world.chunks_iter() {
                        game_renderer.update_chunk(
                            &gl,
                            game.curr.world.chunk_to_index(pos).unwrap().into_tuple(),
                            pos,
                            &chunk,
                        );
                    }
                } else {
                    for (pos, chunk) in game.curr.world.chunks_iter() {
                        if game
                            .prev
                            .world
                            .chunk_at_world(pos * CHUNK_SIZE as i32)
                            .map(|c| c.blocks)
                            != game
                                .curr
                                .world
                                .chunk_at_world(pos * CHUNK_SIZE as i32)
                                .map(|c| c.blocks)
                        {
                            game_renderer.update_chunk(
                                &gl,
                                game.curr.world.chunk_to_index(pos).unwrap().into_tuple(),
                                pos,
                                &chunk,
                            );
                        }
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
                    ui.text(format!("Block Position: {}", game.curr.block_coordinate()));
                    ui.text(format!(
                        "Chunk Position: {} ({:?}) (loaded: {})",
                        game.curr.chunk_coordinate(),
                        game.curr.world.chunk_to_index(game.curr.chunk_coordinate()),
                        game.curr
                            .world
                            .chunk_at_world(game.curr.block_coordinate())
                            .is_some()
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

            imgui_renderer
                .render(&gl, &imgui_textures, imgui.render())
                .unwrap();

            screen_quad_renderer.draw(
                &gl,
                &crosshair_image,
                DrawParams::default()
                    .scale(Vec2::one() * 4.0)
                    .position(Vec2::new(1024.0, 768.0) / 2.0)
                    .origin(Vec2::one() / 2.0),
            );

            // Hotbar
            {
                let scale = Vec2::one() * 5.0;
                let x_max = 9 * slot_image.size.x;
                let x_start = 1024.0 / 2.0 - x_max as f32 * scale.x / 2.0;
                for i in 0..9 {
                    let x_offset = i * slot_image.size.x;

                    let x = x_start + x_offset as f32 * scale.x;
                    let y = 768.0 - 32.0;

                    screen_quad_renderer.draw(
                        &gl,
                        if i as usize == game.curr.hotbar.active {
                            &active_slot_image
                        } else {
                            &slot_image
                        },
                        DrawParams::default()
                            .scale(scale)
                            .position(Vec2::new(x, y))
                            .origin(Vec2::new(0.0, 1.0)),
                    );

                    if let Some(block_or_item) = game.curr.hotbar.slots[i as usize] {
                        if let BlockOrItem::Block(block_ty) = block_or_item {
                            gl.bind_texture(
                                glow::TEXTURE_2D_ARRAY,
                                Some(game_renderer.block_array_texture),
                            );
                            isometric_block_renderer.draw(
                                &gl,
                                block_ty,
                                DrawParams::default()
                                    .scale(scale / 2.5)
                                    .position(
                                        Vec2::new(x, y)
                                            + slot_image.size.as_() * scale / 2.0
                                                * Vec2::new(1.0, -1.0),
                                    )
                                    .origin(Vec2::new(0.5, 0.5)),
                            );
                        }
                    }
                }
            }

            window.gl_swap_window();

            if input_state.get_key(Keycode::K).pressed() {
                sdl.timer().unwrap().delay(100);
            }
        }
    }
}

#[test]
fn test_terrain_sampler() {
    let terrain = rmc_common::game::TerrainSampler::new(6543);
    let mut image = image::GrayImage::new(16 * 7, 16 * 7);
    for chunk_x in 0..7 {
        for chunk_z in 0..7 {
            for local_x in 0..16 {
                for local_z in 0..16 {
                    let world_coord = Vec2::new(
                        chunk_x * CHUNK_SIZE + local_x,
                        chunk_z * CHUNK_SIZE + local_z,
                    );
                    image.put_pixel(
                        world_coord.x as u32,
                        world_coord.y as u32,
                        image::Luma::from([terrain.sample(world_coord.as_()) as u8 * 10]),
                    )
                }
            }
        }
    }
    image.save("../terrain.png").unwrap();
}
