//! Nightfall executable.

mod game;

use game::{Control, Game, Input, Renderer, upgrade_at};
use macroquad::prelude::*;

/// Window settings, including the batching budget the renderer relies on.
fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: miniquad::conf::Conf {
            window_title: "Nightfall".to_owned(),
            window_width: 1280,
            window_height: 720,
            high_dpi: true,
            sample_count: 4,
            window_resizable: true,
            ..Default::default()
        },
        // The renderer submits thousands of batched quads per frame; a larger
        // batching buffer keeps the sprite passes to a handful of draw calls.
        draw_call_vertex_capacity: 65_536,
        draw_call_index_capacity: 65_536,
        ..Default::default()
    }
}

/// Translates device state into one frame of game input.
///
/// Pointer selection of an upgrade card is resolved here, at the boundary where
/// window pixels still exist, so the simulation only ever sees the resulting
/// choice index and never needs to know how the cards are laid out.
fn read_input() -> Input {
    let horizontal = f32::from(is_key_down(KeyCode::D) || is_key_down(KeyCode::Right))
        - f32::from(is_key_down(KeyCode::A) || is_key_down(KeyCode::Left));
    let vertical = f32::from(is_key_down(KeyCode::S) || is_key_down(KeyCode::Down))
        - f32::from(is_key_down(KeyCode::W) || is_key_down(KeyCode::Up));
    let movement = vec2(horizontal, vertical).normalize_or_zero();

    let key_choice = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3]
        .iter()
        .position(|key| is_key_pressed(*key));
    let pointer = is_mouse_button_pressed(MouseButton::Left).then(|| Vec2::from(mouse_position()));
    let choice = key_choice.or_else(|| pointer.and_then(upgrade_at));

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
    // Shader compilation is the one step that can legitimately fail; report it
    // and stop rather than opening a blank window.
    let Some(mut renderer) = Renderer::new() else {
        return;
    };
    let mut game = Game::new();

    loop {
        // Clamp stalls instead of letting one delayed frame tunnel through collisions.
        let dt = get_frame_time().min(1.0 / 30.0);
        if game.update(dt, read_input()) == Control::Restart {
            game = Game::new_running();
        }

        renderer.draw(&game);
        next_frame().await;
    }
}
