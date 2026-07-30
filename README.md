# Nightfall

An asset-free, Vampire Survivors-style teaching game built with Rust 2024 and
Macroquad 0.4.15. Everything on screen is generated in code, so the repository
stays focused on readable game-loop and gameplay code.

## Play

```sh
mise run play
```

When WSLg cannot present OpenGL through Remote Desktop, run the same game in
the Windows browser instead:

```sh
mise run play:web
```

The browser build pairs the WASM binary with the WebGL runtime from the locked
Miniquad package, then serves both locally.

Move with WASD or the arrow keys. Moon Knives orbit automatically and the Storm
Lantern chains lightning; the rest of the arsenal unlocks through level-up
offers. Choose upgrades with 1–3 or the mouse. Pause with Escape.

Survive as long as possible. Night phases deepen the pressure, and periodic
ritual windows spike spawns toward heavier enemies.

## Code map

- `src/main.rs` owns the window loop and translates device state into `Input`.
- `src/game.rs` owns state and applies one ordered simulation step per frame.
- `src/game/render.rs` draws the world without changing simulation state.
- `src/game/render/ui.rs` owns screen-space HUD and menu presentation.

Read `main.rs` first, then follow `Game::update` through `update_running`.
Weapon stats and projectiles live as ordinary structs on `Game`; shared charge
motion goes through `ArmedCharge` / `ChargeMotion`. Rendering is a child module
so it can inspect the model without making fields public.

The loop uses a capped variable timestep. Movement, steering, damping, and
enemy separation use time-based rates so higher refresh rates improve smoothness
without changing the intended simulation.

Run `mise run standards:check` for formatting, Clippy, tests, docs, packaging,
dependency policy, and secret scanning.
