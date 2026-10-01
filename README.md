# Nightfall

An asset-free, Vampire Survivors-style teaching game built with Rust 2024 and
Macroquad 0.4.16. Every pixel is generated in code — procedural shaders rather
than image files — so the repository stays focused on readable game-loop and
gameplay code.

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

## Windows

```sh
mise run build:windows
```

Cross-compiles to `target/windows/nightfall.exe` with the MinGW toolchain. The
build links as a GUI application, so it opens no console window; that also means
the shader-compilation error has nowhere to print, so run the console build
(`cargo build --release --target x86_64-pc-windows-gnu`) from a terminal when
you need to see it.

Move with WASD or the arrow keys. The Moon Knives orbit automatically and the
Storm Lantern chains lightning through nearby enemies. Choose upgrades with
1–3 or the mouse; every level offers one Moon Knife, one Storm Lantern, and one
Witchcraft upgrade. Pause with Escape and survive as long as possible.

## Code map

- `src/main.rs` owns the window loop, holds the renderer, and translates device
  state into `Input` — including the pointer hit test for upgrade cards, so the
  simulation never needs to know how the interface is laid out.
- `src/game.rs` owns state and applies one ordered simulation step per frame.
- `src/game/render/` draws the world without being able to change simulation
  state.

Inside the renderer:

- `mod.rs` owns the GPU resources and the frame's passes.
- `shaders.rs` holds the GLSL ES 1.00 sources.
- `sprite.rs` batches quads for the signed-distance sprite material.
- `scene.rs` builds the world's contents and collects its lights.
- `ui.rs` draws the screen-space interface.

Read `main.rs` first, then follow `Game::update` through `update_running`. The
small data types above `Game` define the complete model; rendering lives in a
child module so it can inspect the model without making any fields public.

The loop uses a capped variable timestep. Movement, steering, damping, and
enemy separation use time-based rates so higher refresh rates improve smoothness
without changing the intended simulation.

## How a frame is drawn

The renderer is a small deferred pipeline, in five stages:

1. **Scene** — the world renders into a multisampled colour target. The ground is
   a single quad whose shader derives every flagstone, crack and patch of moss
   from world position, and creatures, gems and blades are signed-distance sprites
   batched into a handful of draw calls.
2. **Light** — the frame's emitters accumulate additively into a half-resolution
   buffer. One quad evaluates every light analytically, so dozens of them still
   cost a single draw call.
3. **Bloom** — a bright pass and two separable blurs.
4. **Composite** — the scene is multiplied by `ambient + light`, so the world is
   genuinely lit rather than decorated, then tonemapped, graded, and given a
   vignette and fine grain.
5. **Interface** — drawn last, at native resolution, so text and panel edges
   never pass through bloom or grain.

Macroquad's 2D camera maps a vertex's `z` straight into normalised device
coordinates, so sprites carry `z == 0` and depth comes from submission order
instead. Two consequences are worth knowing before editing the renderer:

- A `Camera2D` attached to a render target stores world +Y (down) at the *bottom*
  of the image, the opposite of the on-screen mapping. Passes that sample the
  world targets therefore read them through `1.0 - uv.y`.
- Miniquad binds `color0` with normalisation disabled, so it reaches the vertex
  stage as raw `0..255` bytes and the shader scales it.

## Standards

Run `mise run standards:check` for formatting, Clippy, tests, docs, packaging,
dependency policy, and secret scanning.