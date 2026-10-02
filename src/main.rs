//! Nightfall executable.

mod game;

use game::{Control, Game, Input, TextCache};
use macroquad::{
    input::{
        KeyCode, MouseButton, is_key_down, is_key_pressed, is_mouse_button_pressed, mouse_position,
    },
    math::{Vec2, vec2},
    miniquad::{conf::Conf, date},
    time::get_frame_time,
    window::{next_frame, screen_height, screen_width},
};

const MAX_FRAME_TIME: f32 = 1.0 / 30.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Nightfall".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

fn read_input() -> Input {
    let horizontal = f32::from(is_key_down(KeyCode::D) || is_key_down(KeyCode::Right))
        - f32::from(is_key_down(KeyCode::A) || is_key_down(KeyCode::Left));
    let vertical = f32::from(is_key_down(KeyCode::S) || is_key_down(KeyCode::Down))
        - f32::from(is_key_down(KeyCode::W) || is_key_down(KeyCode::Up));
    let movement = vec2(horizontal, vertical).normalize_or_zero();

    let choice = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3]
        .into_iter()
        .position(is_key_pressed);
    Input {
        movement,
        choice,
        viewport: vec2(screen_width(), screen_height()),
        dpi_scale: macroquad::miniquad::window::dpi_scale(),
        pointer: Vec2::from(mouse_position()),
        click: is_mouse_button_pressed(MouseButton::Left),
        accept: is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space),
        pause: is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P),
        retry: is_key_pressed(KeyCode::R),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Seed at the application seam; the simulation owns its random streams.
    let mut game = Game::new(date::now().to_bits());
    let mut text_cache = TextCache::default();

    loop {
        // Ignore clock rollback and cap frame time to limit collision tunneling.
        let dt = get_frame_time().clamp(0.0, MAX_FRAME_TIME);
        let input = read_input();
        // Some platforms report a zero-sized surface while minimized.
        if input.viewport.min_element() <= 0.0 {
            next_frame().await;
            continue;
        }
        // Warm graphics resources before any draw queues geometry for this frame.
        text_cache.prepare(input.dpi_scale);
        if game.update(dt, input) == Control::Restart {
            game = Game::new_running(date::now().to_bits());
        }

        game.draw(&input);
        next_frame().await;
    }
}
