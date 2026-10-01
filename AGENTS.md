# Agent Guide

Read `README.md` before making gameplay or architectural changes.

## Principles

- Keep game state local to `Game`; input enters through `Input`, application
  transitions leave through `Control`, and drawing does not mutate simulation
  state.
- Keep the renderer out of the simulation. `Game::update` takes an already-resolved
  `Input`, so window pixels, hit tests and GPU resources never reach it.
- Everything visible is generated in code. Prefer procedural shaders over asset
  files, and keep shaders free of derivative functions so every backend compiles
  them.
- Prefer direct data flow and ordinary Rust data types over frameworks or
  speculative abstractions.
- Everything a developer does goes through `mise run`.
- Keep Clippy, formatting, tests, docs, packaging, and dependency policy green.
- Do not add asset-pipeline or ECS dependencies without a measured need.

## Commands

- `mise run play`: run the game.
- `mise run play:web`: run the game in a browser.
- `mise run build:windows`: cross-compile the Windows executable.
- `mise run standards`: format the project.
- `mise run standards:check`: run the complete local CI gate.

Generated output belongs in `target/`, `.cargo-tools/`, or `sbom/` and must not
be committed.
