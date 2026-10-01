//! Builds the world contents: creatures, gems, blades, effects, and the lights
//! they cast.
//!
//! Everything here submits into the renderer's shared batches, so a frame of
//! several hundred shaded creatures still costs a handful of draw calls.
//!
//! Depth is resolved by submission order rather than a depth buffer: macroquad's
//! 2D camera maps a vertex's `z` straight into normalised device coordinates, so
//! any non-zero value would clip the triangle away. Creatures are therefore sorted
//! by world Y and drawn back to front.

use std::f32::consts::TAU;

use macroquad::prelude::*;

use super::shaders::{
    SHAPE_BLADE, SHAPE_BRUTE, SHAPE_GEM, SHAPE_SHADE, SHAPE_SOFT, SHAPE_WARDEN, SHAPE_WISP,
};
use super::sprite::Sprite;
use super::{
    ARCANE_VIOLET, BONE, DAMAGE_RED, GEM_GREEN, Game, Light, MOON_GOLD, Renderer, STORM_CYAN,
};
use crate::game::{Enemy, EnemyKind};

/// How much of the light budget the enemies may claim once the player's own
/// lights are placed.
const ENEMY_LIGHT_SLOTS: usize = 5;
/// Enemies further than this from the player do not emit light.
const ENEMY_LIGHT_RANGE: f32 = 520.0;
/// Storm bolts live for this long, matching the simulation's lifetime.
const BOLT_LIFE: f32 = 0.18;

impl Renderer {
    /// Draws the lit-able world: shadows, pickups, creatures, blades, effects.
    pub(super) fn draw_world(&mut self, game: &Game) {
        // Depth-sorted so a wisp crossing behind a brute occludes correctly.
        // Indices are reused between frames to keep the sort allocation-free.
        self.order.clear();
        self.order.extend(0..game.enemies.len());
        self.order.sort_by(|&left, &right| {
            game.enemies[left]
                .position
                .y
                .total_cmp(&game.enemies[right].position.y)
        });

        for gem in &game.gems {
            let size = if gem.value > 2 { 12.0 } else { 9.0 };
            let pulse = 1.0 + (game.visual_time * 5.0 + gem.position.x * 0.03).sin() * 0.10;
            let gem_sprite = Sprite::at(
                gem.position,
                vec2(size, size * 1.4) * pulse,
                GEM_GREEN,
                SHAPE_GEM,
                size,
            )
            .emitting(0.6)
            .seeded(gem.position.x * 0.1);
            self.sprites.push(&gem_sprite);
            self.spill();
        }

        for index in 0..self.order.len() {
            let enemy = &game.enemies[self.order[index]];
            self.push_shadow(enemy.position, enemy.radius * 1.9, 0.55);
            let (shape, tint) = match enemy.kind {
                EnemyKind::Shade => (SHAPE_SHADE, Color::new(0.52, 0.28, 0.74, 1.0)),
                EnemyKind::Wisp => (SHAPE_WISP, Color::new(0.30, 1.00, 1.15, 1.0)),
                EnemyKind::Brute => (SHAPE_BRUTE, Color::new(0.78, 0.22, 0.38, 1.0)),
            };
            // A struck enemy flashes toward bone white.
            let struck = enemy.flash > 0.0;
            let size = enemy.radius * 2.6;
            let enemy_sprite = Sprite::at(
                enemy.position,
                vec2(size, size),
                if struck { BONE } else { tint },
                shape,
                size,
            )
            .emitting(if struck { 0.7 } else { 0.0 })
            .seeded(enemy.phase);
            self.sprites.push(&enemy_sprite);
            self.spill();
        }

        // The player draws after the enemy pass so a surround stays readable;
        // the contact shadow keeps the depth cue honest.
        let player = &game.player;
        self.push_shadow(player.position, 26.0, 0.72);
        let bob = (game.visual_time * 7.5).sin() * 2.0;
        let lean = player.velocity.normalize_or_zero() * 2.5;
        let size = 78.0;
        let hurt = player.invulnerability > 0.0 && (player.invulnerability * 18.0) as i32 % 2 == 0;
        let warden = Sprite::at(
            player.position + vec2(lean.x, bob + lean.y),
            vec2(size, size),
            if hurt { BONE } else { ARCANE_VIOLET },
            SHAPE_WARDEN,
            size,
        )
        .seeded(player.facing);
        self.sprites.push(&warden);
        self.spill();

        self.draw_blades(game);
        self.draw_bolts(game);
        self.sprites.submit();
    }

    /// Draws the additive halos layered on top of the lit scene.
    pub(super) fn draw_glows(&mut self, game: &Game) {
        // The lantern corona brightens as the spell recharges.
        let charge = game.storm_charge();
        let corona = 44.0 + charge * 34.0;
        self.glows.push(&Sprite::at(
            game.player.lantern_position(),
            vec2(corona, corona),
            STORM_CYAN.with_alpha(0.26 + charge * 0.24),
            0.0,
            corona,
        ));

        for blade in Self::blade_positions(game) {
            self.glows.push(&Sprite::at(
                blade,
                vec2(38.0, 38.0),
                MOON_GOLD.with_alpha(0.26),
                0.0,
                38.0,
            ));
        }

        for index in 0..game.enemies.len() {
            let enemy = &game.enemies[index];
            let tint = match enemy.kind {
                EnemyKind::Shade => MOON_GOLD,
                EnemyKind::Wisp => STORM_CYAN,
                EnemyKind::Brute => DAMAGE_RED,
            };
            self.glows.push(&Sprite::at(
                enemy.position + vec2(0.0, -enemy.radius * 0.3),
                vec2(28.0, 28.0),
                tint.with_alpha(0.30),
                0.0,
                28.0,
            ));
        }

        for gem in &game.gems {
            let size = if gem.value > 2 { 36.0 } else { 28.0 };
            self.glows.push(&Sprite::at(
                gem.position,
                vec2(size, size),
                GEM_GREEN.with_alpha(0.30),
                0.0,
                size,
            ));
        }

        for bolt in &game.lightning {
            let fade = (bolt.life / BOLT_LIFE).clamp(0.0, 1.0);
            for point in bolt.points.iter().skip(1) {
                let size = 90.0 + fade * 40.0;
                self.glows.push(&Sprite::at(
                    *point,
                    vec2(size, size),
                    STORM_CYAN.with_alpha(0.26 + fade * 0.24),
                    0.0,
                    size,
                ));
            }
        }

        // Particles: bright embers that fade with their remaining life.
        for particle in &game.particles {
            let fade = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let size = particle.size * 4.2 * (0.4 + fade * 0.8);
            self.glows.push(&Sprite::at(
                particle.position,
                vec2(size, size),
                particle.color.with_alpha(0.38 * fade),
                0.0,
                size,
            ));
        }

        self.glows.submit();
    }

    /// Gathers this frame's emitters, most important first.
    ///
    /// The player's own lights lead because the scene reads as lit or unlit
    /// depending on them; enemies only receive what budget is left.
    pub(super) fn collect_lights(&mut self, game: &Game) {
        self.lights.clear();

        // The Storm Lantern anchors the scene.
        self.lights.push(Light {
            position: game.player.lantern_position(),
            radius: 330.0,
            color: STORM_CYAN,
            intensity: 0.50 + game.storm_charge() * 0.22,
        });

        for bolt in &game.lightning {
            for point in bolt.points.iter().skip(1) {
                self.lights.push(Light {
                    position: *point,
                    radius: 300.0,
                    color: STORM_CYAN,
                    intensity: 0.85,
                });
            }
        }

        for blade in Self::blade_positions(game) {
            self.lights.push(Light {
                position: blade,
                radius: 195.0,
                color: MOON_GOLD,
                intensity: 0.30,
            });
        }

        self.lights.push(Light {
            position: game.player.position,
            radius: 250.0,
            color: ARCANE_VIOLET,
            intensity: 0.24,
        });

        // The nearest enemies take what remains of the budget.
        let mut visible: Vec<&Enemy> = game
            .enemies
            .iter()
            .filter(|enemy| {
                enemy.position.distance_squared(game.player.position) < ENEMY_LIGHT_RANGE.powi(2)
            })
            .collect();
        visible.sort_by(|left, right| {
            left.position
                .distance_squared(game.player.position)
                .total_cmp(&right.position.distance_squared(game.player.position))
        });
        for enemy in visible.iter().take(ENEMY_LIGHT_SLOTS) {
            let color = match enemy.kind {
                EnemyKind::Shade => ARCANE_VIOLET,
                EnemyKind::Wisp => STORM_CYAN,
                EnemyKind::Brute => DAMAGE_RED,
            };
            self.lights.push(Light {
                position: enemy.position,
                radius: 155.0,
                color,
                intensity: 0.28,
            });
        }

        self.lights.truncate(super::MAX_LIGHTS);
    }

    /// World positions of the orbiting Moon Knives.
    ///
    /// The knife count grows with every Moon upgrade, so this stays small; it is
    /// collected into a vector to keep the callers free of borrow gymnastics.
    fn blade_positions(game: &Game) -> Vec<Vec2> {
        let moon = game.moon;
        let phase = game.moon_angle;
        let center = game.player.position;
        (0..moon.count)
            .map(|index| {
                let angle = phase + index as f32 * TAU / moon.count as f32;
                center + Vec2::from_angle(angle) * moon.radius
            })
            .collect()
    }

    /// A soft contact shadow, sitting just beneath the entity.
    fn push_shadow(&mut self, position: Vec2, size: f32, strength: f32) {
        let shadow = Sprite::at(
            position + vec2(0.0, size * 0.30),
            vec2(size, size),
            Color::new(0.0, 0.0, 0.0, strength),
            SHAPE_SOFT,
            size,
        );
        self.sprites.push(&shadow);
        self.spill();
    }

    /// The orbiting Moon Knives: gold crescents that track their own angle.
    fn draw_blades(&mut self, game: &Game) {
        let blades = Self::blade_positions(game);
        for (index, position) in blades.iter().enumerate() {
            let angle = game.moon_angle + index as f32 * TAU / game.moon.count as f32;
            let size = 42.0;
            self.sprites.push_rotated(
                &Sprite::at(*position, vec2(size, size), MOON_GOLD, SHAPE_BLADE, size)
                    .rotated(angle)
                    .emitting(1.0)
                    .seeded(index as f32),
            );
            self.spill();
        }
    }

    /// The Storm Lantern chain, drawn as kinked segments between chain points.
    fn draw_bolts(&mut self, game: &Game) {
        for bolt in &game.lightning {
            let alpha = (bolt.life / BOLT_LIFE).clamp(0.0, 1.0);
            let thickness = 6.0 + alpha * 5.0;
            for window in bolt.points.windows(2) {
                let (start, end) = (window[0], window[1]);
                // A time-varying offset bends each span into a believable arc.
                let seed = start.x * 0.013 + start.y * 0.021;
                let bend = vec2(
                    (game.visual_time * 83.0 + seed).sin(),
                    (game.visual_time * 61.0 + seed * 1.7).cos(),
                ) * 11.0;
                let midpoint = (start + end) * 0.5 + bend;
                let tint = STORM_CYAN.with_alpha(alpha);
                let arc = Sprite::at(
                    midpoint,
                    vec2(thickness, thickness),
                    tint,
                    SHAPE_BLADE,
                    thickness,
                )
                .emitting(1.0)
                .seeded(seed);
                self.sprites.push_segment(&arc, start, midpoint);
                self.sprites.push_segment(&arc, midpoint, end);
                self.spill();
            }
        }
    }

    /// Submits the sprite batch once it fills, keeping draws inside the index
    /// cap and the memory allocator out of the frame.
    fn spill(&mut self) {
        if self.sprites.is_full() {
            self.sprites.submit();
        }
    }
}
