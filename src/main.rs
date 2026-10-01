//! Nightfall executable.

mod game;

use game::{Control, Game, Input};
use macroquad::prelude::*;

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
        .iter()
        .position(|key| is_key_pressed(*key));
    let pointer = is_mouse_button_pressed(MouseButton::Left).then(|| Vec2::from(mouse_position()));

    Input {
        movement,
        choice,
        pointer,
        accept: is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space),
        pause: is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P),
        retry: is_key_pressed(KeyCode::R),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();

    loop {
        // Clamp stalls instead of letting one delayed frame tunnel through collisions.
        let dt = get_frame_time().min(1.0 / 30.0);
        if game.update(dt, read_input()) == Control::Restart {
            game = Game::new_running();
        }

        game.draw();
        next_frame().await;
    }
}
