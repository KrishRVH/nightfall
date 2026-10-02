# Nightfall

An asset-free, Vampire Survivors-style teaching game built with Rust 2024,
Macroquad 0.4.16, and Miniquad 0.4.11. Everything on screen is generated in code,
so the repository stays focused on readable game-loop and gameplay code.

## Play

```sh
mise run play
```

Move with WASD or the arrow keys. The Moon Knives orbit automatically and the
Storm Lantern chains lightning through nearby enemies. Choose upgrades with
1–3 or the mouse; every level offers one Moon Knife, one Storm Lantern, and one
Witchcraft upgrade. Pause with Escape or P and survive as long as possible.

When WSLg cannot present OpenGL through Remote Desktop, run the same game in
the Windows browser instead:

```sh
mise run play:web
```

The browser build pairs the WASM binary with the WebGL runtime from the locked
Miniquad package, then serves both locally.

Miniquad 0.4.11's browser backend caches display density at startup. The loader
keeps its canvas and pointer coordinates at that same density for the session,
so browser zoom or moving between monitors preserves the UI's proportions.
Reload to adopt the current display's raster density.

## Windows

```sh
mise run build:windows
```

Cross-compiles to `target/windows/nightfall.exe` with the MinGW toolchain. The
build links as a GUI application, so it opens no console window.

## Code map

- `src/main.rs` owns the window loop and translates device state into `Input`.
- `src/game.rs` owns state and random streams, and applies one ordered simulation
  step per frame.
- `src/game/render.rs` draws the world without changing simulation state.
- `src/game/render/ui.rs` owns screen-space HUD and menu presentation.
- `src/game/render/text.rs` prepares font glyphs and draws text at bounded raster sizes.

Read `main.rs` first, then follow `Game::update` through `update_running`. The
small data types above `Game` define the complete model; rendering is kept in a
child module so it can inspect the model without making fields public.

`Input` carries movement, pointer state, actions, viewport dimensions, and DPI.
The application supplies a seed when constructing `Game`, so the entire update
loop can run in ordinary Rust tests without a window or graphics context.
Gameplay and cosmetic effects use separate random streams: changing a particle
burst does not change future enemies or upgrades. Drawing borrows `Game` immutably
and uses the same sampled input as the simulation. Upgrade cards share their
layout with pointer hit testing.

`TextCache` lives in the application and prepares graphics resources before
drawing starts. Text uses a small set of raster sizes and `font_scale` for layout,
so resizing and animated labels use a bounded glyph cache. Fitting remeasures
text when its raster tier changes, accounting for display-density rounding.
Glyphs are cached at startup and on DPI changes, before any geometry is queued.
This also avoids an upstream font-atlas growth bug that can delete a texture
still used by the current batch.

The loop uses a capped variable timestep, ignoring negative clock deltas.
Movement, steering, damping, and enemy separation use time-based rates so higher
refresh rates improve smoothness without changing the intended simulation.
Long stalls discard excess time to bound catch-up work and keep input responsive.
Runs are reproducible for a given seed and input sequence, including frame times.
Hit stop consumes only the stopped portion of a frame before gameplay resumes.

Storage uses `Vec`s and direct iteration, with a bound on enemy count. A pairwise
separation pass uses `split_at_mut` to borrow two enemies safely, and backward
`swap_remove` loops handle deaths and pickups without preserving an unnecessary
ordering. Weapons resolve before contact damage, and defeated enemies cannot
hurt the player. These choices keep ownership and frame ordering visible.
`Phase` represents mutually exclusive modes, with level-up offers stored directly
in that enum variant. Independent button actions remain ordinary booleans.

Run `mise run standards:check` for formatting, Clippy, tests, docs, packaging,
dependency policy, and secret scanning.

The gate also checks the browser and Windows targets. `mise run build:windows`
verifies final Windows linking, while `mise run rust:doc` generates documentation
including private implementation items in `target/doc/nightfall/index.html`.
The tests cover simulation transitions, seeded runs, combat ordering, pickup
timing, frame-rate behavior, camera orientation, and menu hit testing.

`mise run sbom` inventories locked dependencies in `sbom/sbom.cdx.json`, excluding
generated builds, caches, and installed development tools.

Macroquad provides and re-exports its pinned Miniquad dependency. The browser
runtime comes from that exact locked package. `deny.toml` records two upstream
advisory exceptions with no patched release: Macroquad's soundness advisory and
the unmaintained `ttf-parser` dependency used by `fontdue`.
