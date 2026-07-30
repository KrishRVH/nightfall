//! Procedural rendering for [`Game`].
//!
//! World drawing uses a camera centered on the player; overlays switch back to
//! screen coordinates. Every function takes immutable state so rendering cannot
//! alter the simulation.

mod ui;

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use macroquad::prelude::*;

pub(super) use ui::upgrade_at;

use super::{
    ABYSSAL_MINE, AETHER_MAUVE, ARCANE_VIOLET, AetherSpearProjectile, ArmedCharge, BACKGROUND,
    BONE, CINDER_ORANGE, CRESCENT_AUREATE, DAMAGE_NUMBER_LIFE, DAMAGE_RED, DamageNumber, EchoShard,
    Enemy, EnemyKind, FlarePulse, GEM_GREEN, GLOOM_PURPLE, GRAVE_MARROW, Game, Gem, GloomShard,
    GravityWell, HarrowDart, INK, ImpactRing, InfernoFireball, LANCE_AZURE, Lightning,
    LuminousLanceBolt, MOON_GOLD, PHANTOM_CERULEAN, PHANTOM_EMBER, PHANTOM_NET_GREEN,
    PhantomTether, Phase, PrismNeedle, RIFT_CYAN, RIFT_VIOLET, RITUAL_ACTIVE_SECS, RiftEcho,
    RunePulse, SOLAR_NOVA, STARFALL_GOLD, STORM_CYAN, SigilThread, TemporalRiftWell, VIEW_HEIGHT,
    VOID_BLOOM, WRAITH_AMETHYST, WraithTrace, view_width,
};

const GROUND: Color = Color::new(0.060, 0.050, 0.105, 1.0);
const GROUND_ALT: Color = Color::new(0.070, 0.058, 0.120, 1.0);
const GROUND_LINE: Color = Color::new(0.16, 0.13, 0.24, 0.34);
const SKY_STREAK: Color = Color::new(0.14, 0.10, 0.21, 1.0);
const GROUND_BLOOM: Color = Color::new(0.30, 0.16, 0.38, 0.10);
const DRIFT_SMOKE: Color = Color::new(0.24, 0.16, 0.31, 0.05);
const VEIL_STREAK: Color = Color::new(0.82, 0.75, 0.95, 0.10);
const RUNE_GLOW: Color = Color::new(0.50, 0.40, 0.72, 0.18);
const MOON_DUST: Color = Color::new(0.80, 0.78, 0.92, 0.10);
const MOON_RING_CORE: Color = Color::new(0.98, 0.95, 1.0, 0.30);
const MOON_RING_OUTER: Color = Color::new(0.72, 0.63, 0.92, 0.16);
const LENS_GLASS: Color = Color::new(0.78, 0.93, 1.0, 0.05);
const SOFT_EDGE: Color = Color::new(0.88, 0.82, 0.95, 0.08);
const FOG_CORE: Color = Color::new(0.17, 0.10, 0.22, 0.13);
const ECLIPSE_WHITE: Color = Color::new(0.98, 0.94, 1.0, 0.16);
const CHROMA_SPLIT: Color = Color::new(0.35, 0.95, 1.0, 0.06);
const AURORA_GREEN: Color = Color::new(0.49, 0.94, 0.92, 0.10);
const AURORA_ORANGE: Color = Color::new(0.98, 0.52, 0.28, 0.09);

struct StarLayer {
    parallax: f32,
    spacing: f32,
    modulo: u32,
    speed: f32,
}

impl Game {
    /// Draws the current frame without mutating simulation state.
    pub(crate) fn draw(&self) {
        clear_background(BACKGROUND);

        if self.phase == Phase::Title {
            self.draw_title();
            return;
        }
        let (_, _, accent, menace) = self.night_state();
        let ritual_warning = self.ritual_warning();
        let ritual_window = self.is_ritual_window();
        let ritual_remaining = self.ritual_time_remaining();

        let shake = if self.shake > 0.0 {
            vec2(
                (self.visual_time * 83.0).sin(),
                (self.visual_time * 71.0).cos(),
            ) * self.shake
        } else {
            Vec2::ZERO
        };
        let camera_center = self.camera + shake;
        let camera = world_camera(camera_center, view_width());
        set_camera(&camera);
        self.draw_world(
            camera_center,
            ritual_window,
            ritual_warning,
            ritual_remaining,
        );
        set_default_camera();
        draw_vignette(
            self.player.health / self.player.max_health,
            self.visual_time,
            menace,
            accent,
        );
        self.draw_phase_flash(accent);
        self.draw_post_fx(
            self.player.health / self.player.max_health,
            menace,
            accent,
            self.hit_stop,
        );
        self.draw_hud();

        match self.phase {
            Phase::Paused => Self::draw_pause(),
            Phase::LevelUp => self.draw_level_up(),
            Phase::GameOver => self.draw_game_over(),
            Phase::Title | Phase::Running => {},
        }
    }

    fn draw_phase_flash(&self, accent: Color) {
        if self.phase_flash <= 0.0 {
            return;
        }

        let power = (self.phase_flash / super::PHASE_FLASH_TIME).clamp(0.0, 1.0);
        let radius = screen_width().max(screen_height()) * 0.74;
        let sweep =
            (1.0 + (self.visual_time * 6.0).sin().abs() * 0.08) * (1.0 + (1.0 - power) * 0.2);
        draw_circle(
            screen_width() * 0.5,
            screen_height() * 0.5,
            radius * sweep,
            Color::new(accent.r, accent.g, accent.b, 0.14 * power),
        );
        draw_rectangle_lines(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            1.0,
            Color::new(
                1.0,
                1.0,
                1.0,
                (0.45 * power * (1.0 + (self.visual_time * 18.0).cos() * 0.1)).clamp(0.0, 0.45),
            ),
        );
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Post-processing FX intentionally packs cohesive visual effects in one phase"
    )]
    fn draw_post_fx(&self, health_ratio: f32, menace: f32, accent: Color, hit_stop: f32) {
        let width = screen_width();
        let height = screen_height();
        let health_ratio = health_ratio.clamp(0.0, 1.0);
        let color_bias = if health_ratio < 0.4 {
            Color::new(0.92, 0.34, 0.34, 0.09)
        } else {
            Color::new(0.22, 0.34, 0.95, 0.07)
        };
        let grade = 0.04 + menace * 0.13 + (1.0 - health_ratio) * 0.06;
        let hit_impact = (hit_stop / 0.2).clamp(0.0, 1.0);

        for strip in 0..22 {
            let t = strip as f32 / 22.0;
            let band = height * t;
            let band_height = 8.0 + (t * 2.0).sin().abs();
            let alpha = (1.0 - t).clamp(0.05, 0.18) * grade;
            draw_rectangle(
                0.0,
                band,
                width,
                band_height,
                Color::new(
                    accent.r * 0.14 + color_bias.r * 0.1,
                    accent.g * 0.14 + color_bias.g * 0.1,
                    accent.b * 0.14 + color_bias.b * 0.1,
                    alpha * 0.34,
                ),
            );
        }

        let glow = (1.0 + (self.visual_time * 0.9 + menace * 1.8).sin()) * 0.5;
        for layer in 0..7u8 {
            let radius = width.min(height) * (0.24 + layer as f32 * 0.09);
            let drift = (self.visual_time * 0.22 + layer as f32).cos() * 6.0 * (1.0 + menace);
            draw_circle_lines(
                width * 0.5 + drift,
                height * 0.54,
                radius + glow * 0.8,
                1.0,
                Color::new(
                    color_bias.r,
                    color_bias.g,
                    color_bias.b,
                    0.018 + 0.005 * (7 - layer) as f32 * glow / 2.0,
                ),
            );
        }

        let scan_seed = (self.visual_time * 130.0) as i32;
        for scan in 0..34u16 {
            let hash = hash_cell(scan as i32 + scan_seed, scan_seed ^ (scan as i32 * 11));
            let y = (hash % 4096) as f32 / 4096.0 * height;
            let alpha = ((scan as f32 / 34.0) * (0.5 + menace * 0.5)).min(0.18);
            let wave = (self.visual_time * 18.0 + hash as f32 * 0.014).sin().abs();
            draw_line(
                0.0,
                y,
                width,
                y,
                1.0,
                Color::new(
                    color_bias.r,
                    color_bias.g,
                    color_bias.b,
                    0.007 + alpha * wave * 0.5,
                ),
            );
        }

        let sweep = (self.visual_time * 0.4 + health_ratio * 0.3).sin() * 0.5 + 0.5;
        draw_rectangle(
            width * (0.45 + sweep * 0.18),
            0.0,
            width * 0.07,
            height,
            Color::new(
                CHROMA_SPLIT.r,
                CHROMA_SPLIT.g,
                CHROMA_SPLIT.b,
                0.03 + menace * 0.03,
            ),
        );
        draw_rectangle(
            width * (0.41 + sweep * 0.18),
            0.0,
            width * 0.005,
            height,
            Color::new(0.0, 0.0, 0.0, 0.12 + menace * 0.1),
        );
        self.draw_edge_pulse(width, height, health_ratio, menace, hit_impact);

        if hit_impact > 0.0 {
            let ring_alpha = 0.16 + hit_impact * 0.44;
            let pulse = (self.visual_time * 42.0).sin().abs() * 0.5 + 0.5;
            for layer in 0..6u8 {
                let ring_radius =
                    (layer as f32 + 1.0) * width.min(height) * 0.09 * (1.0 + hit_impact);
                draw_circle_lines(
                    width * 0.5,
                    height * 0.5,
                    ring_radius * (0.72 + pulse * 0.1),
                    1.4,
                    Color::new(1.0, 0.92, 0.88, (ring_alpha * 0.065) / (layer as f32 + 1.4)),
                );
            }

            let drift = (self.visual_time * 110.0).sin().abs() * 6.0 * hit_impact;
            draw_rectangle(
                width * 0.5 - drift,
                0.0,
                3.0 + hit_impact * 2.0,
                height,
                Color::new(1.0, 1.0, 1.0, 0.18 * hit_impact),
            );
            draw_rectangle(
                width * 0.5 + drift,
                0.0,
                4.0 + hit_impact,
                height,
                Color::new(1.0, 0.9, 0.9, 0.12 * hit_impact),
            );
        }
    }

    fn draw_edge_pulse(
        &self,
        width: f32,
        height: f32,
        health_ratio: f32,
        menace: f32,
        hit_impact: f32,
    ) {
        let health_pressure = 1.0 - health_ratio;
        let intensity =
            (0.08 + menace * 0.14 + health_pressure * 0.26 + hit_impact * 0.42).min(1.0);
        if intensity <= 0.0 {
            return;
        }

        let drift = (self.visual_time * 1.6 + health_pressure * 2.2).cos().abs() * 2.4;
        let edge_width = width * (0.0032 + (menace * 0.004) + (hit_impact * 0.01));
        let edge_tint = Color::new(1.0, 0.96, 0.86, (0.06 + health_pressure * 0.09) * intensity);
        let pulse_tint = Color::new(0.0, 0.0, 0.0, (0.05 + hit_impact * 0.12) * intensity);

        draw_rectangle(0.0 + drift, 0.0, edge_width, height, edge_tint);
        draw_rectangle(
            width - edge_width - drift,
            0.0,
            edge_width,
            height,
            edge_tint,
        );
        draw_rectangle(0.0, 0.0, width, edge_width * 1.8, edge_tint);
        draw_rectangle(
            0.0,
            height - edge_width * 1.8,
            width,
            edge_width * 1.8,
            edge_tint,
        );

        draw_line(
            width * 0.5 - edge_width * 2.0,
            0.0,
            width * 0.5 + edge_width * 2.0,
            0.0,
            1.0,
            pulse_tint,
        );
        draw_line(
            width * 0.5 - edge_width * 2.0,
            height,
            width * 0.5 + edge_width * 2.0,
            height,
            1.0,
            pulse_tint,
        );
    }

    #[allow(
        clippy::too_many_lines,
        reason = "World renderer composes a fixed draw-pass sequence and remains readable as one unit"
    )]
    fn draw_world(
        &self,
        camera_center: Vec2,
        ritual_window: bool,
        ritual_warning: f32,
        ritual_remaining: f32,
    ) {
        let (_, _, accent, menace) = self.night_state();

        self.draw_atmosphere(
            camera_center,
            accent,
            menace,
            ritual_window,
            ritual_warning,
            ritual_remaining,
        );
        self.draw_bloom_rings(camera_center, menace, self.is_ritual_window());
        Self::draw_ground(camera_center);

        for gem in &self.gems {
            draw_gem(*gem, self.visual_time);
        }
        for particle in &self.particles {
            let alpha = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let mut color = particle.color;
            color.a = alpha;
            draw_circle(
                particle.position.x,
                particle.position.y,
                particle.size * 2.4 * alpha.sqrt(),
                Color::new(color.r, color.g, color.b, alpha * 0.10),
            );
            draw_circle(
                particle.position.x,
                particle.position.y,
                particle.size * alpha.sqrt(),
                color,
            );
        }
        for enemy in &self.enemies {
            draw_enemy(*enemy, self.visual_time);
        }
        for shard in &self.grave_shards {
            draw_armed_charge(*shard, GRAVE_MARROW, 2.2, ChargeGlyph::Core);
        }
        for seed in &self.void_seeds {
            draw_armed_charge(*seed, VOID_BLOOM, 1.8, ChargeGlyph::Ringed);
        }
        for pulse in &self.flare_pulses {
            draw_flare_pulse(*pulse, self.visual_time);
        }
        for minelet in &self.abyssal_minelets {
            draw_armed_charge(*minelet, ABYSSAL_MINE, 1.4, ChargeGlyph::Haloed);
        }
        for meteor in &self.starfall_meteors {
            draw_armed_charge(*meteor, STARFALL_GOLD, 2.4, ChargeGlyph::Meteor);
        }
        for tether in &self.phantom_tethers {
            draw_phantom_tether(*tether, self.player.position, self.visual_time);
        }
        for bolt in &self.luminous_lance_bolts {
            draw_luminous_lance_bolt(*bolt);
        }
        for well in &self.temporal_rift_wells {
            draw_temporal_rift_well(*well, self.visual_time);
        }
        for shard in &self.gloom_shards {
            draw_gloom_shard(*shard);
        }
        for shard in &self.shard_storm_shards {
            draw_gloom_shard(*shard);
        }
        for trace in &self.wraith_traces {
            draw_wraith_trace(*trace);
        }
        for dart in &self.harrow_darts {
            draw_harrow_dart(*dart, self.visual_time);
        }
        for spear in &self.aether_projectiles {
            draw_aether_spear(*spear);
        }
        for echo in &self.rift_echoes {
            draw_rift_echo(*echo, self.visual_time);
        }
        for shard in &self.echo_shards {
            draw_echo_shard(*shard);
        }
        for needle in &self.prism_needles {
            draw_prism_needle(*needle);
        }
        for meteor in &self.inferno_meteors {
            draw_inferno_meteor(*meteor);
        }
        for well in &self.gravity_wells {
            draw_gravity_well(*well, self.visual_time);
        }
        for pulse in &self.rune_pulses {
            draw_rune_pulse(*pulse, self.visual_time);
        }
        for thread in &self.sigil_threads {
            draw_sigil_thread(*thread, self.player.position, self.visual_time);
        }
        for ring in &self.impact_rings {
            draw_impact_ring(*ring, self.visual_time);
        }
        self.draw_player();
        self.draw_moon_knives();
        for lightning in &self.lightning {
            draw_lightning(lightning, self.visual_time);
        }
        for number in &self.damage_numbers {
            draw_damage_number(number);
        }

        self.draw_offscreen_enemies(camera_center, menace, accent);
        self.draw_surface_sheen(camera_center, menace, ritual_window);
    }

    fn draw_offscreen_enemies(&self, camera_center: Vec2, menace: f32, accent: Color) {
        let half_width = view_width() * 0.5;
        let half_height = VIEW_HEIGHT * 0.5;
        let margin = 22.0;
        let safe_left = camera_center.x - half_width + margin;
        let safe_right = camera_center.x + half_width - margin;
        let safe_top = camera_center.y - half_height + margin;
        let safe_bottom = camera_center.y + half_height - margin;

        let mut drawn = 0u8;
        for enemy in &self.enemies {
            if enemy.health <= 0.0 || drawn >= 9 {
                continue;
            }

            let enemy_x = enemy.position.x;
            let enemy_y = enemy.position.y;
            let offset = enemy.position - camera_center;
            if enemy_x >= safe_left
                && enemy_x <= safe_right
                && enemy_y >= safe_top
                && enemy_y <= safe_bottom
            {
                continue;
            }

            let absolute_x = offset.x.abs().max(0.001);
            let absolute_y = offset.y.abs().max(0.001);
            let sx = (half_width - margin) / absolute_x;
            let sy = (half_height - margin) / absolute_y;
            let scale = (sx.min(sy) * 0.97).min(1.0);
            let direction = offset.normalize_or_zero();
            let marker = camera_center + offset * scale;

            let threat = (self.player.position.distance(enemy.position) / 3200.0).min(1.0);
            let pulse = 0.5 + ((self.visual_time * 10.0 + enemy.phase).sin().abs() * 0.5);
            let radius = (6.0 + menace * 12.0) * (1.0 - threat * 0.35) * pulse;
            let wing = radius * 1.35;
            let pointer = direction * radius;
            let tail = -direction * (radius * 0.85);
            let side = vec2(-direction.y, direction.x) * radius * 0.7;

            draw_circle(
                marker.x,
                marker.y,
                radius + 3.0,
                Color::new(accent.r, accent.g, accent.b, 0.28),
            );
            draw_circle(marker.x, marker.y, radius * 0.45, accent);
            draw_triangle(
                marker + pointer,
                marker + tail + side,
                marker + tail - side,
                Color::new(1.0, 1.0, 1.0, 0.72),
            );
            draw_triangle(
                marker + pointer * 0.8,
                marker + tail * 0.7 + side * 0.6,
                marker + tail * 0.7 - side * 0.6,
                Color::new(1.0, 1.0, 1.0, 0.3),
            );
            draw_ellipse(
                marker.x,
                marker.y - wing * 0.65,
                wing,
                2.1,
                0.0,
                Color::new(accent.r, accent.g, accent.b, 0.18),
            );

            drawn += 1;
        }
    }

    fn draw_atmosphere(
        &self,
        camera_center: Vec2,
        accent: Color,
        menace: f32,
        ritual_window: bool,
        ritual_warning: f32,
        ritual_remaining: f32,
    ) {
        Self::draw_sky_gradient(camera_center, accent, menace);
        Self::draw_parallax_stars(camera_center, self.visual_time, accent, menace);
        Self::draw_ambient_mist(camera_center, menace, self.visual_time);
        Self::draw_mote_fog(camera_center, self.visual_time, menace, accent);
        Self::draw_nebula_flow(camera_center, self.visual_time, accent, menace);
        Self::draw_veil_strands(camera_center, self.visual_time, accent, menace);
        Self::draw_moonlight_arch(camera_center, self.visual_time, accent, menace);
        Self::draw_aurora_veils(camera_center, self.visual_time, menace);
        Self::draw_ritual_horizon(
            camera_center,
            self.visual_time,
            menace,
            ritual_window,
            ritual_warning,
            ritual_remaining,
        );
    }

    fn draw_surface_sheen(&self, camera_center: Vec2, menace: f32, ritual_window: bool) {
        let width = view_width();
        let pulse = 1.0 + self.player.velocity.length() * 0.005;
        draw_ellipse(
            camera_center.x,
            camera_center.y + 20.0,
            width * 0.46,
            130.0,
            0.0,
            Color::new(DRIFT_SMOKE.r, DRIFT_SMOKE.g, DRIFT_SMOKE.b, 0.03 * pulse),
        );
        if self.player.invulnerability > 0.0 {
            let heat = (self.player.invulnerability * 7.0).sin().abs() * 0.15;
            draw_circle(
                self.player.position.x,
                self.player.position.y + 24.0,
                62.0,
                Color::new(1.0, 0.5, 0.38, heat * 0.08),
            );
        }

        let ritual_glow = if ritual_window {
            0.20 + (self.visual_time * 3.2).sin().abs() * 0.10
        } else {
            0.04
        };
        draw_ellipse(
            camera_center.x + 4.0 * menace.sin(),
            camera_center.y + 24.0,
            width * 0.30,
            80.0 + menace * 38.0,
            0.0,
            Color::new(1.0, 0.9, 0.7, ritual_glow * (0.15 + menace * 0.08)),
        );
        let lens = (self.visual_time * 1.6).sin().abs();
        draw_ellipse(
            camera_center.x - 4.0 * menace.sin(),
            camera_center.y + 20.0,
            width * (0.12 + menace * 0.10),
            36.0 + menace * 20.0,
            0.0,
            Color::new(LENS_GLASS.r, LENS_GLASS.g, LENS_GLASS.b, 0.03 + lens * 0.04),
        );
    }

    fn draw_bloom_rings(&self, camera_center: Vec2, menace: f32, ritual_window: bool) {
        let moon_ring = 190.0 + self.player.velocity.length() * 0.12;
        let glow = if ritual_window { 0.22 } else { 0.11 };
        let layers = 6usize;
        for band in 0..layers {
            let spread = band as f32 / layers as f32;
            let radius = moon_ring * (1.0 + spread * 0.36);
            let alpha = (1.0 - spread) * glow * (0.35 + menace * 0.4);
            draw_circle(
                camera_center.x,
                camera_center.y + 40.0 + spread * 16.0,
                radius * (1.0 + (self.visual_time * 0.2).sin().abs() * 0.015),
                Color::new(
                    ECLIPSE_WHITE.r,
                    ECLIPSE_WHITE.g,
                    ECLIPSE_WHITE.b,
                    alpha * 0.22,
                ),
            );
            if band > 0 && spread < 0.55 {
                draw_circle_lines(
                    camera_center.x + spread * 2.0,
                    camera_center.y + 40.0,
                    radius * 0.95,
                    0.9,
                    Color::new(CHROMA_SPLIT.r, CHROMA_SPLIT.g, CHROMA_SPLIT.b, alpha * 0.14),
                );
            }
        }
    }

    fn draw_nebula_flow(camera_center: Vec2, time: f32, accent: Color, menace: f32) {
        let width = view_width();
        for band in 0..7 {
            let local = band as f32 / 7.0;
            let wave = (time * 0.4 + band as f32 * 1.8 + menace * 3.0).sin();
            let y = camera_center.y - 150.0 + wave * 4.8 + local * 56.0;
            let alpha = (0.02 + local * 0.01 + menace * 0.03) * (1.0 - local * 0.15);
            let tint = Color::new(
                FOG_CORE.r + accent.r * 0.15,
                FOG_CORE.g + accent.g * 0.15,
                FOG_CORE.b + accent.b * 0.15,
                alpha,
            );
            for segment in 0..18 {
                let phase = band as f32 * 0.9 + segment as f32 * 1.7 + time * (0.15 + local);
                let start = vec2(
                    camera_center.x - width * 0.5 + segment as f32 * width / 18.0
                        - (time * 2.0 + phase).sin() * 8.0,
                    y + (phase * 1.3).sin() * 6.0 + (wave * 16.0) * (0.2 + local),
                );
                let end = start + vec2(16.0 + local * 11.0, 24.0 * (0.4 + (band as f32 * 0.08)));
                draw_line(start.x, start.y, end.x, end.y, 2.0, tint);
            }
        }
    }

    fn draw_veil_strands(camera_center: Vec2, time: f32, accent: Color, menace: f32) {
        for band in 0..7 {
            let height = (camera_center.y - 290.0) + band as f32 * 72.0;
            let drift = (time * 0.45 + band as f32 * 0.95 + menace * 4.0).sin() * 6.0;
            let jitter = (time * 2.2 + band as f32 * 1.3).sin() * 24.0;
            let stripe = 0.4 + band as f32 * 0.6;
            let alpha = (0.06 + menace * 0.12) * (1.0 - band as f32 / 9.0);
            let pulse = 1.0 + (time * 0.8 + band as f32).sin().abs() * 0.2;

            draw_rectangle(
                camera_center.x - view_width() * 0.66 + jitter,
                height + drift,
                view_width() * 1.32,
                stripe * pulse,
                Color::new(VEIL_STREAK.r, VEIL_STREAK.g, VEIL_STREAK.b, alpha),
            );

            draw_rectangle_lines(
                camera_center.x - view_width() * 0.66 + jitter * 0.4,
                height + drift + stripe,
                view_width() * 1.32,
                stripe * 0.2,
                1.0,
                Color::new(accent.r, accent.g, accent.b, alpha * 0.5),
            );
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "ground tiling is intentionally explicit to keep texture variation deterministic"
    )]
    fn draw_ground(camera_center: Vec2) {
        let width = view_width();
        let cell = 128.0;
        let start_x = ((camera_center.x - width * 0.5) / cell).floor() as i32 - 1;
        let end_x = ((camera_center.x + width * 0.5) / cell).ceil() as i32 + 1;
        let start_y = ((camera_center.y - VIEW_HEIGHT * 0.5) / cell).floor() as i32 - 1;
        let end_y = ((camera_center.y + VIEW_HEIGHT * 0.5) / cell).ceil() as i32 + 1;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let position = vec2(x as f32 * cell, y as f32 * cell);
                let hash = hash_cell(x, y);
                let tile = if hash.is_multiple_of(4) {
                    GROUND_ALT
                } else {
                    GROUND
                };
                draw_rectangle(position.x, position.y, cell, cell, tile);
                draw_line(
                    position.x,
                    position.y,
                    position.x + cell,
                    position.y,
                    1.0,
                    GROUND_LINE,
                );
                draw_line(
                    position.x,
                    position.y,
                    position.x,
                    position.y + cell,
                    1.0,
                    GROUND_LINE,
                );

                if hash.is_multiple_of(7) {
                    let offset = vec2((hash % 48) as f32, ((hash >> 6) % 48) as f32);
                    let grave = position + vec2(35.0, 31.0) + offset;
                    draw_ellipse(
                        grave.x + 3.0,
                        grave.y + 17.0,
                        10.0,
                        4.0,
                        0.0,
                        Color::new(0.0, 0.0, 0.0, 0.18),
                    );
                    draw_rectangle(
                        grave.x - 6.0,
                        grave.y,
                        12.0,
                        17.0,
                        Color::new(0.19, 0.17, 0.28, 0.58),
                    );
                    draw_circle(grave.x, grave.y, 6.0, Color::new(0.19, 0.17, 0.28, 0.58));
                    draw_line(
                        grave.x,
                        grave.y + 5.0,
                        grave.x,
                        grave.y + 12.0,
                        1.5,
                        Color::new(0.40, 0.35, 0.50, 0.35),
                    );
                } else if hash.is_multiple_of(9) {
                    let offset = vec2((hash % 16) as f32 * 0.7, ((hash >> 6) % 16) as f32 * 0.7);
                    let crack_a = position + vec2(20.0, 20.0) + offset;
                    let crack_b = crack_a + vec2(82.0 - (hash % 32) as f32, 36.0);
                    draw_line(
                        crack_a.x,
                        crack_a.y,
                        crack_b.x,
                        crack_b.y,
                        0.9,
                        Color::new(0.08, 0.04, 0.10, 0.28),
                    );
                    draw_line(
                        crack_a.x + 11.0,
                        crack_a.y + 3.0,
                        crack_a.x - 20.0,
                        crack_a.y + 24.0,
                        0.7,
                        Color::new(0.24, 0.16, 0.31, 0.2),
                    );
                } else if hash.is_multiple_of(11) {
                    let rune = position + vec2((hash & 63) as f32, ((hash >> 7) & 63) as f32);
                    draw_circle_lines(
                        rune.x + 30.0,
                        rune.y + 30.0,
                        13.0,
                        1.0,
                        Color::new(0.36, 0.27, 0.55, 0.22),
                    );
                    draw_circle(
                        rune.x + 30.0,
                        rune.y + 30.0,
                        2.0,
                        Color::new(0.56, 0.44, 0.78, 0.30),
                    );
                    draw_circle(
                        rune.x + 30.0,
                        rune.y + 30.0,
                        5.0 + (hash % 4) as f32 * 0.7,
                        Color::new(0.38, 0.28, 0.56, 0.24),
                    );
                    draw_line(
                        rune.x + 29.0,
                        rune.y + 25.0,
                        rune.x + 31.0,
                        rune.y + 35.0,
                        0.8,
                        RUNE_GLOW,
                    );
                } else if hash.is_multiple_of(5) {
                    let star = position + vec2((hash % 96) as f32, ((hash >> 7) % 96) as f32);
                    draw_circle(star.x, star.y, 1.5, Color::new(0.45, 0.38, 0.65, 0.38));
                }
            }
        }
    }

    fn draw_sky_gradient(camera_center: Vec2, accent: Color, menace: f32) {
        let width = view_width() * 1.35;
        let top = camera_center.y - VIEW_HEIGHT * 0.55;
        let height = VIEW_HEIGHT * 0.95;
        let mut y = top;
        let bands = 14;

        for layer in 0..bands {
            let depth = layer as f32 / bands as f32;
            let alpha = (0.14 - depth * 0.09) + menace * 0.28;
            let color = Color::new(
                SKY_STREAK.r + accent.r * (1.0 - depth) * 0.18,
                SKY_STREAK.g + accent.g * (1.0 - depth) * 0.14,
                SKY_STREAK.b + accent.b * (1.0 - depth) * 0.18,
                alpha.clamp(0.02, 0.22),
            );
            draw_rectangle(
                camera_center.x - width * 0.5,
                y,
                width,
                height / bands as f32,
                color,
            );
            y += height / bands as f32;
        }

        let moon_y = camera_center.y + 40.0;
        for layer in 0..3 {
            let drift = (moon_y * 0.006 + layer as f32 * 0.4 + menace * 8.0).sin()
                * 0.5
                * (3.0 + layer as f32);
            draw_ellipse(
                camera_center.x - 190.0,
                moon_y + drift,
                width * 0.35,
                90.0 - layer as f32 * 11.5,
                0.0,
                Color::new(0.0, 0.0, 0.0, 0.015 + layer as f32 * 0.005 + menace * 0.04),
            );
        }
    }

    fn draw_parallax_stars(camera_center: Vec2, time: f32, tint: Color, menace: f32) {
        Self::draw_star_layer(
            camera_center,
            time,
            tint,
            menace,
            StarLayer {
                parallax: 0.55,
                spacing: 280.0,
                modulo: 5,
                speed: 1.2,
            },
        );
        Self::draw_star_layer(
            camera_center,
            time,
            tint,
            menace,
            StarLayer {
                parallax: 0.36,
                spacing: 190.0,
                modulo: 3,
                speed: 0.8,
            },
        );
        Self::draw_star_layer(
            camera_center,
            time,
            tint,
            menace,
            StarLayer {
                parallax: 0.22,
                spacing: 132.0,
                modulo: 2,
                speed: 0.5,
            },
        );
    }

    fn draw_star_layer(camera_center: Vec2, time: f32, tint: Color, menace: f32, layer: StarLayer) {
        let StarLayer {
            parallax,
            spacing,
            modulo,
            speed,
        } = layer;
        let width = view_width() * 1.4;
        let start_x = ((camera_center.x * parallax - width * 0.7) / spacing).floor() as i32 - 1;
        let end_x = ((camera_center.x * parallax + width * 0.7) / spacing).ceil() as i32 + 1;
        let start_y = ((camera_center.y - VIEW_HEIGHT * 0.55) / spacing).floor() as i32 - 1;
        let end_y = ((camera_center.y + VIEW_HEIGHT * 0.45) / spacing).ceil() as i32 + 1;
        let pulse = (time * 0.5).sin().abs();

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let hash = hash_cell(x, y);
                if !hash.is_multiple_of(modulo) {
                    continue;
                }

                let jitter = vec2(
                    (hash & 15) as f32 * 0.8 - 6.0,
                    ((hash >> 4) & 15) as f32 * 0.8 - 6.0,
                );
                let position = vec2(
                    x as f32 * spacing + jitter.x + (time * speed * 4.0),
                    y as f32 * spacing + jitter.y,
                );

                let flicker = 0.4
                    + (((hash % 100) as f32 * 0.06 + time * 3.8 + pulse * 7.0)
                        .sin()
                        .abs())
                        * 0.6;
                let size = 1.1 + (hash % 4) as f32 * 0.35;
                let alpha = (0.28 + menace * 0.2) * (flicker / 2.0);
                draw_circle(
                    position.x,
                    position.y,
                    size,
                    Color::new(tint.r * 0.82, tint.g * 0.82, tint.b * 0.88, alpha * 0.7),
                );

                if hash.is_multiple_of(19) {
                    draw_line(
                        position.x,
                        position.y,
                        position.x - 10.0,
                        position.y - 16.0 + hash as f32 % 4.0,
                        1.0,
                        Color::new(1.0, 0.98, 0.95, alpha * 0.3),
                    );
                }
            }
        }
    }

    fn draw_ambient_mist(camera_center: Vec2, menace: f32, time: f32) {
        let mut base = Vec2::new(
            camera_center.x,
            camera_center.y + 30.0 + (time * 5.0).cos() * 3.0,
        );
        for layer in 0..5 {
            let scale = 1.0 + layer as f32 * 0.2;
            let drift = (time * 0.32 + layer as f32 * 0.9).sin() * 7.0;
            base.x += drift * (if layer % 2 == 0 { 1.0 } else { -1.0 });
            base.y += (layer as f32 - 2.0) * 48.0;
            draw_ellipse(
                base.x,
                base.y,
                260.0 * scale,
                86.0 + layer as f32 * 14.0,
                0.0,
                Color::new(
                    GROUND_BLOOM.r,
                    GROUND_BLOOM.g,
                    GROUND_BLOOM.b,
                    0.02 + 0.012 * scale + menace * 0.12,
                ),
            );
        }
    }

    fn draw_ritual_horizon(
        camera_center: Vec2,
        time: f32,
        menace: f32,
        ritual_window: bool,
        ritual_warning: f32,
        ritual_remaining: f32,
    ) {
        if !(ritual_window || ritual_warning > 0.0) {
            return;
        }

        let intensity = if ritual_window {
            0.36 + (RITUAL_ACTIVE_SECS - ritual_remaining).clamp(0.0, 1.0) * 0.3
        } else {
            0.15 + ritual_warning * 0.25
        };
        let width = view_width() * 0.95;
        let height = 210.0 + menace * 60.0;
        let horizon = camera_center.y + 190.0;

        for index in 0..8 {
            let band = index as f32 / 8.0;
            let drift = (time * 1.8 + index as f32 * 0.7 + menace * 4.0).sin() * 7.0;
            let swell = 0.70 + band * 0.36 * intensity;
            let alpha = 0.028 * (1.0 - band) * intensity;
            draw_ellipse(
                camera_center.x + drift,
                horizon + band * 18.0,
                width * (0.88 + band * 0.2),
                height * (0.12 + band * 0.16),
                0.0,
                Color::new(0.38, 0.07, 0.13, alpha * (0.8 + band * 0.2)),
            );
            draw_ellipse(
                camera_center.x + drift * 0.38,
                horizon + band * 16.0,
                width * (0.86 + band * 0.15) * swell,
                height * (0.06 + band * 0.06),
                0.0,
                Color::new(1.0, 0.22, 0.34, 0.03 * (1.0 - band)),
            );
        }

        if ritual_window {
            let spark_width = width * 0.65;
            let spark_alpha = 0.08 + intensity * 0.14;
            draw_rectangle(
                camera_center.x - spark_width * 0.5,
                horizon + 6.0,
                spark_width,
                3.0,
                Color::new(1.0, 0.32, 0.20, spark_alpha),
            );
            draw_rectangle_lines(
                camera_center.x - spark_width * 0.5,
                horizon + 6.0,
                spark_width,
                3.0,
                1.5,
                Color::new(1.0, 0.64, 0.54, spark_alpha + 0.1),
            );
        }
    }

    fn draw_moonlight_arch(camera_center: Vec2, time: f32, accent: Color, menace: f32) {
        let moon_x = camera_center.x - 190.0 + (time * 0.15).cos() * 18.0;
        let moon_y = camera_center.y + 48.0 + (time * 0.25).sin() * 6.0;
        let base_radius = 210.0 + menace * 28.0;

        for ring in 0..8 {
            let phase = time * 0.8 + ring as f32 * 0.42;
            let bloom = (phase.sin().abs() * 0.18 + 0.82) * (1.0 + menace * 0.2);
            let line_width = 10.0 - ring as f32 * 0.9;
            draw_ellipse(
                moon_x,
                moon_y + (ring as f32 - 3.5) * 2.0,
                base_radius + ring as f32 * 20.0 + bloom * 5.0,
                32.0 + ring as f32 * 3.2,
                0.0,
                Color::new(
                    MOON_RING_OUTER.r + accent.r * 0.2,
                    MOON_RING_OUTER.g + accent.g * 0.2,
                    MOON_RING_OUTER.b + accent.b * 0.2,
                    (0.02 + ring as f32 * 0.004) * (1.0 + menace * 0.4),
                ),
            );

            if ring < 4 {
                draw_circle(
                    moon_x,
                    moon_y,
                    base_radius * (0.30 + ring as f32 * 0.08) * bloom,
                    Color::new(
                        MOON_RING_CORE.r,
                        MOON_RING_CORE.g,
                        MOON_RING_CORE.b,
                        (0.028 + menace * 0.02) / (ring as f32 + 1.3),
                    ),
                );
                draw_circle_lines(
                    moon_x,
                    moon_y,
                    base_radius * (0.30 + ring as f32 * 0.08) * bloom,
                    line_width,
                    Color::new(1.0, 1.0, 1.0, (0.09 - ring as f32 * 0.01) * bloom * 0.6),
                );
            }
        }

        let flare = (time * 2.2).sin().abs() * 0.5 + 0.5;
        for index in 0..8 {
            let phase = time * (1.2 + index as f32 * 0.15) + index as f32 * 0.9;
            let width = 170.0 + index as f32 * 20.0 + menace * 50.0;
            let spread = flare * 2.2 * phase.cos();
            let alpha = (0.014 + index as f32 * 0.0018).min(0.03);
            draw_line(
                moon_x - width,
                moon_y + spread,
                moon_x + width,
                moon_y + spread * 0.9,
                1.0,
                Color::new(
                    MOON_RING_OUTER.r,
                    MOON_RING_OUTER.g,
                    MOON_RING_OUTER.b,
                    alpha * (1.0 - index as f32 * 0.09),
                ),
            );
        }
    }

    fn draw_aurora_veils(camera_center: Vec2, time: f32, menace: f32) {
        let width = view_width();
        for layer in 0..9 {
            let depth = layer as f32 / 9.0;
            let drift = (time * 0.35 + layer as f32).sin() * (6.0 + depth * 8.0);
            let y = camera_center.y - 200.0 + depth * 500.0 + (time * 1.25).cos() * 6.0;
            let ribbon_width = width * (0.45 + depth * 0.34);
            let alpha = (0.015 + menace * 0.02) * (1.0 - depth * 0.16);

            for sweep in 0..14 {
                let step = (sweep as f32 / 14.0) * ribbon_width * 2.0 - ribbon_width;
                let oscillate = (time * 0.48 + layer as f32 + sweep as f32 * 0.52).sin()
                    * (18.0 + depth * 32.0);
                let x = camera_center.x + step + drift + oscillate * 0.2;
                let thickness = 1.8 + layer as f32 * 0.14;
                let tint = if layer % 2 == 0 {
                    AURORA_GREEN
                } else {
                    AURORA_ORANGE
                };

                draw_line(
                    x,
                    y + oscillate * 0.18,
                    x + 28.0,
                    y + oscillate * 0.22 + 24.0,
                    thickness,
                    Color::new(tint.r, tint.g, tint.b, alpha),
                );
                if sweep % 5 == 0 {
                    draw_line(
                        x - width * 0.03,
                        y + oscillate * 0.14,
                        x + width * 0.03,
                        y + oscillate * 0.16 + 12.0,
                        thickness * 0.55,
                        Color::new(1.0, 1.0, 1.0, alpha * 0.28),
                    );
                }
            }
        }
    }

    fn draw_player(&self) {
        let (_, _, accent, menace) = self.night_state();
        let bob = (self.visual_time * 7.5).sin() * 2.0;
        let lean = self.player.velocity.normalize_or_zero() * 2.5;
        let position = self.player.position + vec2(lean.x, bob + lean.y);
        let hurt_flash = self.player.invulnerability > 0.0
            && (self.player.invulnerability * 18.0) as i32 % 2 == 0;
        let cloak = if hurt_flash { BONE } else { ARCANE_VIOLET };
        let hood = if hurt_flash {
            BONE
        } else {
            Color::new(0.20, 0.12, 0.33, 1.0)
        };

        draw_ellipse(
            position.x,
            position.y + 22.0,
            24.0,
            8.0,
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.36),
        );
        draw_triangle(
            position + vec2(-21.0, 22.0),
            position + vec2(21.0, 22.0),
            position + vec2(-self.player.facing * 5.0, -7.0),
            hood,
        );
        draw_triangle(
            position + vec2(-15.0, 19.0),
            position + vec2(15.0, 19.0),
            position + vec2(self.player.facing * 3.0, -5.0),
            cloak,
        );
        draw_circle(
            position.x + self.player.facing * 2.0,
            position.y - 13.0,
            14.5,
            hood,
        );
        draw_triangle(
            position + vec2(-13.0, -14.0),
            position + vec2(13.0, -14.0),
            position + vec2(-self.player.facing * 4.0, -33.0),
            hood,
        );
        draw_circle(position.x, position.y - 13.0, 8.5, BONE);
        draw_circle(
            position.x + self.player.facing * 3.0,
            position.y - 14.0,
            1.5,
            INK,
        );
        self.draw_player_presence(position, accent, menace);
        self.draw_weapon_sigils(position, menace);
        self.draw_motion_sheen(position);

        self.draw_storm_lantern(position, bob, hood);

        draw_circle_lines(
            position.x,
            position.y,
            self.player.pickup_radius,
            1.0,
            Color::new(0.45, 0.84, 0.90, 0.055),
        );
    }

    fn draw_motion_sheen(&self, position: Vec2) {
        let speed = self.player.velocity.length();
        if speed < 48.0 {
            return;
        }

        let direction = self.player.velocity.normalize_or_zero();
        let menace = (1.0 - self.player.health / self.player.max_health).clamp(0.0, 1.0);
        let segments = ((speed - 48.0) / 48.0).floor().clamp(0.0, 5.0) as usize;
        let trail_limit = 6.0;
        for segment in 0..=segments {
            let progress = (segment as f32 + 1.0) / (segments as f32 + 2.0);
            let offset = direction * -55.0 * progress;
            let alpha = (0.10 + menace * 0.20) * (1.0 - progress);
            let width = 24.0 - progress * 6.0;
            let height = 7.0 - progress * 1.8;
            draw_ellipse(
                position.x + offset.x,
                position.y + 24.0 + progress * 2.0,
                width.max(4.0),
                height.max(1.2),
                0.0,
                Color::new(0.80, 0.85, 1.0, alpha * 0.35),
            );
            draw_ellipse(
                position.x + offset.x * 0.85,
                position.y + 24.0,
                (width * 0.62).max(2.0),
                (height * 0.62).max(0.8),
                0.0,
                Color::new(1.0, 0.92, 0.95, alpha * 0.12),
            );
            if segment % 2 == 0 {
                draw_ellipse(
                    position.x + offset.x * 1.4,
                    position.y + 23.0 + progress * 3.0,
                    (width * 0.38).max(1.6),
                    (height * 1.65).max(0.8),
                    0.0,
                    Color::new(0.45, 0.72, 0.98, alpha * 0.20),
                );
            }
        }

        draw_line(
            position.x + direction.x * -trail_limit,
            position.y + 24.0,
            position.x + direction.x * -trail_limit * 0.6,
            position.y + 24.0 + 2.0,
            1.0,
            Color::new(1.0, 0.98, 1.0, 0.10 + menace * 0.15),
        );
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Sigil renderer is intentionally explicit for synchronized orbital readability"
    )]
    fn draw_weapon_sigils(&self, position: Vec2, menace: f32) {
        let weapon_charge = |unlocked: bool, timer: f32, cooldown: f32| -> f32 {
            if !unlocked || cooldown <= 0.0 {
                0.0
            } else {
                (1.0 - (timer.max(0.0) / cooldown)).clamp(0.0, 1.0)
            }
        };

        let storm_charge = weapon_charge(true, self.storm_timer, self.storm.cooldown);
        let grave_charge = weapon_charge(
            self.grave_mines.unlocked(),
            self.grave_mines.timer,
            self.grave_mines.cooldown,
        );
        let flare_charge = weapon_charge(
            self.astral_flare.unlocked(),
            self.astral_flare.timer,
            self.astral_flare.cooldown,
        );
        let wraith_charge = weapon_charge(
            self.wraith_lash.unlocked(),
            self.wraith_lash.timer,
            self.wraith_lash.cooldown,
        );
        let harrow_charge = weapon_charge(
            self.harrow_volley.unlocked(),
            self.harrow_volley.timer,
            self.harrow_volley.cooldown,
        );
        let aether_charge = weapon_charge(
            self.aether_spears.unlocked(),
            self.aether_spears.timer,
            self.aether_spears.cooldown,
        );
        let rift_charge = weapon_charge(
            self.rift_pulse.unlocked(),
            self.rift_pulse.timer,
            self.rift_pulse.cooldown,
        );
        let echo_charge = weapon_charge(
            self.echo_cannon.unlocked(),
            self.echo_cannon.timer,
            self.echo_cannon.cooldown,
        );
        let ward_charge = weapon_charge(
            self.rune_wards.unlocked(),
            self.rune_wards.timer,
            self.rune_wards.cooldown,
        );
        let halo_charge = weapon_charge(
            self.crescent_halo.unlocked(),
            self.crescent_halo.timer,
            self.crescent_halo.cooldown,
        );
        let bloom_charge = weapon_charge(
            self.void_bloom.unlocked(),
            self.void_bloom.timer,
            self.void_bloom.cooldown,
        );
        let solar_charge = weapon_charge(
            self.solar_nova.unlocked(),
            self.solar_nova.timer,
            self.solar_nova.cooldown,
        );
        let abyssal_charge = weapon_charge(
            self.abyssal_mines.unlocked(),
            self.abyssal_mines.timer,
            self.abyssal_mines.cooldown,
        );
        let sigils: &[(Color, f32, bool)] = &[
            (MOON_GOLD, 1.0_f32, true),
            (STORM_CYAN, storm_charge, self.storm.level > 0),
            (GRAVE_MARROW, grave_charge, self.grave_mines.unlocked()),
            (CINDER_ORANGE, flare_charge, self.astral_flare.unlocked()),
            (WRAITH_AMETHYST, wraith_charge, self.wraith_lash.unlocked()),
            (ARCANE_VIOLET, harrow_charge, self.harrow_volley.unlocked()),
            (AETHER_MAUVE, aether_charge, self.aether_spears.unlocked()),
            (RIFT_VIOLET, rift_charge, self.rift_pulse.unlocked()),
            (PHANTOM_CERULEAN, echo_charge, self.echo_cannon.unlocked()),
            (PHANTOM_EMBER, ward_charge, self.rune_wards.unlocked()),
            (CRESCENT_AUREATE, halo_charge, self.crescent_halo.unlocked()),
            (VOID_BLOOM, bloom_charge, self.void_bloom.unlocked()),
            (SOLAR_NOVA, solar_charge, self.solar_nova.unlocked()),
            (ABYSSAL_MINE, abyssal_charge, self.abyssal_mines.unlocked()),
        ];
        let ring_base = 31.0 + menace * 7.0;
        for (index, &(color, charge, active)) in sigils.iter().enumerate() {
            let angle =
                self.visual_time * 1.25 + index as f32 * 1.05 + position.y * 0.001 + charge * 0.5;
            let orbit = ring_base + (index as f32 * 1.7) % 9.0;
            let marker = position + vec2(angle.cos() * orbit, angle.sin() * orbit * 0.72);
            let inner_alpha = if active {
                (0.15 + charge * 0.35).min(0.58)
            } else {
                0.09
            };
            let ring_alpha = if active { 0.26 + menace * 0.12 } else { 0.10 };
            let marker_radius = if active { 3.2 + charge * 1.6 } else { 2.0 };
            let fill = if active { charge } else { 0.35 };
            draw_circle(
                marker.x,
                marker.y,
                marker_radius,
                Color::new(color.r, color.g, color.b, inner_alpha * fill),
            );
            draw_circle_lines(
                marker.x,
                marker.y,
                marker_radius + 4.0,
                1.0,
                Color::new(1.0, 1.0, 1.0, ring_alpha * fill),
            );
            draw_line(
                position.x,
                position.y,
                marker.x,
                marker.y,
                1.0 + charge,
                Color::new(color.r, color.g, color.b, inner_alpha),
            );
        }
    }

    fn draw_player_presence(&self, position: Vec2, accent: Color, menace: f32) {
        let ritual = self.is_ritual_window();
        let breath = if ritual {
            1.0 + (self.visual_time * 14.0).sin().abs() * 0.18
        } else {
            1.0
        };
        let aura = 46.0 + menace * 26.0;
        let ring = aura * breath;
        let halo = 0.09 + menace * 0.18;
        let flicker = (self.visual_time * 6.0 + self.player.health).sin().abs() * 0.04;
        draw_circle(
            position.x,
            position.y + 24.0,
            ring * 0.75,
            Color::new(accent.r, accent.g, accent.b, halo * 0.45 + flicker),
        );
        draw_circle_lines(
            position.x,
            position.y + 24.0,
            ring,
            1.6,
            Color::new(1.0, 0.95, 0.92, (halo * 2.0).min(0.32)),
        );

        let flare = if ritual {
            Color::new(1.0, 0.24, 0.12, 0.14)
        } else {
            Color::new(0.45, 0.84, 0.90, 0.10)
        };
        for index in 0..3 {
            let phase = self.visual_time * 1.2 + index as f32 * 1.7;
            draw_line(
                position.x + phase.cos() * (ring * 0.3),
                position.y + 24.0 + phase.sin() * (ring * 0.25),
                position.x + phase.cos() * ring * 0.08,
                position.y + 24.0 + phase.sin() * ring * 0.08,
                0.8,
                flare,
            );
        }
    }

    fn draw_mote_fog(camera_center: Vec2, time: f32, menace: f32, accent: Color) {
        let x = ((camera_center.x - view_width() * 0.65) / 64.0).floor() as i32 - 2;
        let max_x = ((camera_center.x + view_width() * 0.65) / 64.0).ceil() as i32 + 2;
        let mut y = ((camera_center.y - VIEW_HEIGHT * 0.58) / 64.0).floor() as i32 - 2;
        let max_y = ((camera_center.y + VIEW_HEIGHT * 0.45) / 64.0).ceil() as i32 + 2;
        let pulse = (time * 0.9).sin().abs();

        while y <= max_y {
            let mut cursor_x = x;
            while cursor_x <= max_x {
                let hash = hash_cell(cursor_x, y * 11);
                if hash.is_multiple_of(9) {
                    let jitter = vec2((hash & 15) as f32 - 7.5, ((hash >> 4) & 15) as f32 - 7.5);
                    let drift = (time * 1.2 + hash as f32 * 0.003).cos() * 5.0;
                    let start = vec2(
                        cursor_x as f32 * 64.0 + jitter.x + drift,
                        y as f32 * 64.0 + jitter.y,
                    );
                    let glow = 0.24 + 0.14 * menace + pulse * 0.08;
                    let alpha = glow * (0.22 + 0.18 * (hash % 13) as f32 / 13.0);
                    draw_line(
                        start.x,
                        start.y,
                        start.x - 22.0,
                        start.y + 8.0,
                        1.2,
                        Color::new(
                            MOON_DUST.r,
                            MOON_DUST.g,
                            MOON_DUST.b,
                            alpha * (0.12 + hash as f32 * 0.0008).min(0.42),
                        ),
                    );
                    draw_circle(
                        start.x + drift,
                        start.y - 6.0,
                        1.8 + (hash % 6) as f32 * 0.24,
                        Color::new(accent.r, accent.g, accent.b, alpha * 0.32),
                    );
                }
                cursor_x += 1;
            }
            y += 1;
        }

        for index in 0..52i32 {
            let hash = hash_cell(index, ((time * 8.0) as i32 + index * 17) % 1024);
            let x =
                (hash % 512) as f32 / 512.0 * view_width() + camera_center.x - view_width() * 0.5;
            let y = ((hash >> 9) % 420) as f32 / 420.0 * VIEW_HEIGHT + camera_center.y
                - VIEW_HEIGHT * 0.45;
            let length = 11.0 + (hash % 7) as f32;
            let sweep = (time * 0.5 + hash as f32 * 0.01).sin();
            draw_line(
                x,
                y,
                x + length * sweep,
                y - length * 0.7,
                0.7,
                Color::new(
                    SOFT_EDGE.r,
                    SOFT_EDGE.g,
                    SOFT_EDGE.b,
                    (0.05 + menace * 0.05).min(0.19),
                ),
            );
        }
    }

    fn draw_storm_lantern(&self, player_position: Vec2, bob: f32, hood: Color) {
        let lantern = self.player.lantern_position() + vec2(0.0, bob);
        let storm_charge = (1.0 - self.storm_timer.max(0.0) / self.storm.cooldown).clamp(0.0, 1.0);
        let glow = 9.0 + storm_charge * 7.0;
        let pulse = (self.visual_time * 7.0).sin() * 2.0;
        draw_circle(
            lantern.x,
            lantern.y,
            glow + 8.0,
            Color::new(
                STORM_CYAN.r,
                STORM_CYAN.g,
                STORM_CYAN.b,
                0.04 + storm_charge * 0.08,
            ),
        );
        for ring in 0..2 {
            draw_circle_lines(
                lantern.x,
                lantern.y,
                glow + pulse + ring as f32 * 7.0,
                1.0,
                Color::new(
                    STORM_CYAN.r,
                    STORM_CYAN.g,
                    STORM_CYAN.b,
                    (0.09 + storm_charge * 0.18) / (ring + 1) as f32,
                ),
            );
        }
        for spark in 0..3 {
            let angle = self.visual_time * (2.8 + spark as f32 * 0.35) + spark as f32 * TAU / 3.0;
            let spark_position = lantern + Vec2::from_angle(angle) * (11.0 + storm_charge * 10.0);
            draw_line(
                lantern.x,
                lantern.y,
                spark_position.x,
                spark_position.y,
                1.0,
                Color::new(
                    STORM_CYAN.r,
                    STORM_CYAN.g,
                    STORM_CYAN.b,
                    0.08 + storm_charge * 0.20,
                ),
            );
            draw_circle(
                spark_position.x,
                spark_position.y,
                1.2 + storm_charge,
                STORM_CYAN,
            );
        }
        draw_rectangle(lantern.x - 5.0, lantern.y - 5.0, 10.0, 12.0, INK);
        draw_rectangle_lines(
            lantern.x - 5.0,
            lantern.y - 5.0,
            10.0,
            12.0,
            1.5,
            STORM_CYAN,
        );
        draw_circle(lantern.x, lantern.y + 1.0, 2.5 + storm_charge, STORM_CYAN);
        draw_line(
            player_position.x - self.player.facing * 8.0,
            player_position.y - 1.0,
            lantern.x,
            lantern.y - 5.0,
            2.0,
            hood,
        );
    }

    fn draw_moon_knives(&self) {
        let orbit_pulse = (self.visual_time * 3.5).sin();
        draw_circle_lines(
            self.player.position.x,
            self.player.position.y,
            self.moon.radius + orbit_pulse * 2.0,
            1.0,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, 0.18),
        );
        draw_circle_lines(
            self.player.position.x,
            self.player.position.y,
            self.moon.radius - 6.0 - orbit_pulse,
            1.0,
            Color::new(BONE.r, BONE.g, BONE.b, 0.06),
        );
        for index in 0_usize..8 {
            let angle = -self.moon_angle * 0.18 + index as f32 * TAU / 8.0;
            let marker = self.player.position + Vec2::from_angle(angle) * self.moon.radius;
            draw_circle(
                marker.x,
                marker.y,
                if index.is_multiple_of(2) { 2.2 } else { 1.2 },
                Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, 0.30),
            );
        }

        for index in 0..self.moon.count {
            let angle = self.moon_angle + index as f32 * TAU / self.moon.count as f32;
            self.draw_moon_knife(angle);
        }
    }

    fn draw_moon_knife(&self, angle: f32) {
        let direction = Vec2::from_angle(angle);
        let position = self.player.position + direction * self.moon.radius;
        let tangent = vec2(-direction.y, direction.x);
        let trail_direction = Vec2::from_angle(angle - self.moon.speed * 0.16);
        let trail = self.player.position + trail_direction * self.moon.radius;
        draw_line(
            trail.x,
            trail.y,
            position.x,
            position.y,
            11.0,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, 0.08),
        );
        draw_line(
            trail.x,
            trail.y,
            position.x,
            position.y,
            3.0,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, 0.24),
        );

        for echo in (1..=3).rev() {
            let echo_direction = Vec2::from_angle(angle - self.moon.speed * 0.045 * echo as f32);
            let echo_position = self.player.position + echo_direction * self.moon.radius;
            let echo_tangent = vec2(-echo_direction.y, echo_direction.x);
            let alpha = 0.14 / echo as f32;
            draw_triangle(
                echo_position + echo_direction * 16.0,
                echo_position - echo_direction * 10.0 + echo_tangent * 5.0,
                echo_position - echo_direction * 10.0 - echo_tangent * 5.0,
                Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, alpha),
            );
        }

        draw_circle(
            position.x,
            position.y,
            25.0,
            Color::new(1.0, 0.68, 0.20, 0.08),
        );
        draw_circle(
            position.x,
            position.y,
            13.0,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, 0.12),
        );
        draw_triangle(
            position + direction * 21.0,
            position - direction * 14.0 + tangent * 8.0,
            position - direction * 14.0 - tangent * 8.0,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, 0.32),
        );
        draw_triangle(
            position + direction * 18.0,
            position - direction * 12.0 + tangent * 6.0,
            position - direction * 12.0 - tangent * 6.0,
            BONE,
        );
        draw_line(
            position.x - direction.x * 12.0,
            position.y - direction.y * 12.0,
            position.x - direction.x * 17.0,
            position.y - direction.y * 17.0,
            4.0,
            MOON_GOLD,
        );
        draw_circle(
            position.x + direction.x * 8.0,
            position.y + direction.y * 8.0,
            1.8,
            Color::new(1.0, 1.0, 1.0, 0.90),
        );
    }
}

fn world_camera(center: Vec2, width: f32) -> Camera2D {
    let mut camera = Camera2D::from_display_rect(Rect::new(
        center.x - width * 0.5,
        center.y - VIEW_HEIGHT * 0.5,
        width,
        VIEW_HEIGHT,
    ));
    // Input and sprite geometry use screen-style coordinates: positive Y is down.
    camera.zoom.y = camera.zoom.y.abs();
    camera
}

// Stable cell hashing decorates the world without consuming gameplay RNG state.
pub(super) fn hash_cell(x: i32, y: i32) -> u32 {
    let mut value = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value
}

#[allow(
    clippy::too_many_lines,
    reason = "visual effect branch is intentionally rich for readability"
)]
fn draw_enemy(enemy: Enemy, time: f32) {
    let bob = (time * 5.0 + enemy.phase).sin() * 2.5;
    let position = enemy.position + vec2(0.0, bob);
    let flash = enemy.flash > 0.0;
    let shadow = Color::new(0.0, 0.0, 0.0, 0.28);
    draw_ellipse(
        position.x,
        position.y + enemy.radius * 0.75,
        enemy.radius,
        enemy.radius * 0.42,
        0.0,
        shadow,
    );

    match enemy.kind {
        EnemyKind::Shade => {
            let color = if flash {
                BONE
            } else {
                Color::new(0.31, 0.18, 0.44, 1.0)
            };
            draw_circle(
                position.x,
                position.y,
                enemy.radius * 1.45,
                Color::new(color.r, color.g, color.b, 0.08),
            );
            draw_triangle(
                position + vec2(-enemy.radius - 3.0, 6.0),
                position + vec2(enemy.radius + 3.0, 6.0),
                position + vec2(0.0, enemy.radius + 13.0),
                Color::new(color.r * 0.65, color.g * 0.65, color.b * 0.65, 1.0),
            );
            draw_circle(position.x, position.y, enemy.radius, color);
            draw_circle(position.x - 5.0, position.y - 2.0, 2.2, MOON_GOLD);
            draw_circle(position.x + 5.0, position.y - 2.0, 2.2, MOON_GOLD);
        },
        EnemyKind::Wisp => {
            let color = if flash {
                BONE
            } else {
                Color::new(0.20, 0.72, 0.82, 1.0)
            };
            draw_circle(
                position.x,
                position.y,
                23.0,
                Color::new(color.r, color.g, color.b, 0.08),
            );
            draw_triangle(
                position + vec2(0.0, -17.0),
                position + vec2(-12.0, 11.0),
                position + vec2(12.0, 11.0),
                color,
            );
            draw_triangle(
                position + vec2(-5.0, 7.0),
                position + vec2(5.0, 7.0),
                position + vec2((time * 8.0 + enemy.phase).sin() * 5.0, 23.0),
                Color::new(color.r, color.g, color.b, 0.45),
            );
            draw_circle(
                position.x,
                position.y,
                9.0,
                Color::new(0.72, 0.95, 0.94, 1.0),
            );
            draw_circle(position.x, position.y, 3.0, INK);
        },
        EnemyKind::Howl => {
            let color = if flash {
                BONE
            } else {
                Color::new(0.42, 0.70, 0.95, 1.0)
            };
            draw_circle(
                position.x,
                position.y,
                enemy.radius * 1.25,
                Color::new(color.r, color.g, color.b, 0.1),
            );
            draw_ellipse(
                position.x,
                position.y + 2.0,
                enemy.radius * 0.8,
                enemy.radius * 1.25,
                0.0,
                Color::new(color.r, color.g, color.b, 0.65),
            );
            draw_circle(position.x, position.y, enemy.radius * 0.68, color);
            draw_triangle(
                position + vec2(0.0, -enemy.radius - 1.0),
                position + vec2(-enemy.radius * 0.6, enemy.radius * 0.75),
                position + vec2(enemy.radius * 0.6, enemy.radius * 0.75),
                Color::new(color.r * 0.65, color.g * 0.65, color.b * 0.65, 1.0),
            );
            draw_line(
                position.x - 3.0,
                position.y - 1.0,
                position.x - 3.0,
                position.y + 9.0,
                1.5,
                STORM_CYAN,
            );
            draw_line(
                position.x + 3.0,
                position.y - 1.0,
                position.x + 3.0,
                position.y + 9.0,
                1.5,
                STORM_CYAN,
            );
            draw_circle_lines(
                position.x,
                position.y + 9.0 + (time * 18.0 + enemy.phase).sin() * 0.6,
                6.0,
                1.0,
                Color::new(0.9, 1.0, 1.0, 0.9),
            );
        },
        EnemyKind::Brute => {
            let color = if flash {
                BONE
            } else {
                Color::new(0.43, 0.14, 0.25, 1.0)
            };
            draw_triangle(
                position + vec2(-enemy.radius + 2.0, -15.0),
                position + vec2(-enemy.radius - 9.0, -28.0),
                position + vec2(-enemy.radius + 12.0, -19.0),
                BONE,
            );
            draw_triangle(
                position + vec2(enemy.radius - 2.0, -15.0),
                position + vec2(enemy.radius + 9.0, -28.0),
                position + vec2(enemy.radius - 12.0, -19.0),
                BONE,
            );
            draw_circle(position.x, position.y, enemy.radius, color);
            draw_circle_lines(position.x, position.y, enemy.radius, 3.0, DAMAGE_RED);
            draw_rectangle(position.x - 15.0, position.y - 6.0, 30.0, 12.0, INK);
            draw_circle(position.x - 8.0, position.y, 2.5, MOON_GOLD);
            draw_circle(position.x + 8.0, position.y, 2.5, MOON_GOLD);
            draw_line(
                position.x - 11.0,
                position.y + 13.0,
                position.x + 11.0,
                position.y + 13.0,
                3.0,
                Color::new(0.16, 0.05, 0.09, 1.0),
            );
        },
        EnemyKind::Revenant => {
            let color = if flash {
                BONE
            } else {
                Color::new(0.62, 0.06, 0.11, 1.0)
            };
            draw_circle(
                position.x,
                position.y,
                enemy.radius * 1.28,
                Color::new(color.r, color.g, color.b, 0.08),
            );
            draw_circle(position.x, position.y, enemy.radius, color);
            draw_triangle(
                position + vec2(-enemy.radius * 0.9, -enemy.radius),
                position + vec2(enemy.radius * 0.9, -enemy.radius),
                position + vec2(0.0, -enemy.radius * 1.9),
                Color::new(0.90, 0.42, 0.52, 1.0),
            );
            draw_ellipse(
                position.x,
                position.y + 2.0,
                enemy.radius * 0.8,
                enemy.radius * 0.3,
                0.2,
                ARCANE_VIOLET,
            );
            let eyes = (time * 5.0 + enemy.phase).sin().abs();
            draw_circle(
                position.x - 4.0,
                position.y - 2.0,
                2.3,
                Color::new(1.0, 0.9, 0.75, eyes + 0.2),
            );
            draw_circle(
                position.x + 4.0,
                position.y - 2.0,
                2.3,
                Color::new(1.0, 0.9, 0.75, eyes + 0.2),
            );
            draw_circle_lines(
                position.x,
                position.y - 6.0,
                enemy.radius * 0.58,
                1.8,
                Color::new(0.94, 0.70, 0.84, 0.8),
            );
        },
    }
}

#[derive(Clone, Copy)]
enum ChargeGlyph {
    Core,
    Ringed,
    Haloed,
    Meteor,
}

fn draw_armed_charge(charge: ArmedCharge, color: Color, life_scale: f32, glyph: ChargeGlyph) {
    let life_ratio = if charge.timer.is_finite() {
        (charge.timer / life_scale).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let glow = if charge.arming > 0.0 {
        0.18 + charge.arming * 1.3
    } else {
        1.0
    }
    .min(1.0);
    let radius = match glyph {
        ChargeGlyph::Meteor => 4.2,
        ChargeGlyph::Haloed => 6.0,
        ChargeGlyph::Ringed => 7.5,
        ChargeGlyph::Core => 7.0,
    } + (1.0 - life_ratio) * 2.0;

    draw_circle(
        charge.position.x,
        charge.position.y,
        radius,
        Color::new(color.r, color.g, color.b, 0.38 * life_ratio),
    );

    match glyph {
        ChargeGlyph::Core | ChargeGlyph::Haloed => {
            draw_circle(
                charge.position.x,
                charge.position.y,
                radius * 0.45,
                Color::new(color.r, color.g, color.b, glow),
            );
            if matches!(glyph, ChargeGlyph::Haloed) {
                draw_circle_lines(
                    charge.position.x,
                    charge.position.y,
                    radius * 1.2,
                    1.3,
                    Color::new(1.0, 1.0, 1.0, 0.16 * life_ratio),
                );
            }
        },
        ChargeGlyph::Ringed => {
            draw_circle_lines(
                charge.position.x,
                charge.position.y,
                radius * 0.55,
                1.4,
                Color::new(color.r, color.g, color.b, glow),
            );
            draw_circle_lines(
                charge.position.x,
                charge.position.y,
                radius + 2.4,
                1.0 + charge.arming,
                Color::new(color.r, color.g, color.b, 0.16 * life_ratio),
            );
        },
        ChargeGlyph::Meteor => {
            draw_circle_lines(
                charge.position.x,
                charge.position.y,
                radius * 0.78,
                1.8,
                Color::new(color.r, color.g, color.b, 0.82),
            );
        },
    }
}

fn draw_prism_needle(needle: PrismNeedle) {
    let trail = needle.velocity * 0.06;
    let previous = needle.position - trail;
    draw_line(
        previous.x,
        previous.y,
        needle.position.x,
        needle.position.y,
        1.5,
        Color::new(
            PHANTOM_CERULEAN.r,
            PHANTOM_CERULEAN.g,
            PHANTOM_CERULEAN.b,
            0.18,
        ),
    );
    draw_line(
        previous.x,
        previous.y,
        needle.position.x,
        needle.position.y,
        (needle.radius * 0.8).max(1.0),
        PHANTOM_CERULEAN,
    );
    draw_circle(
        needle.position.x,
        needle.position.y,
        needle.radius * 1.4,
        PHANTOM_CERULEAN,
    );
    draw_circle_lines(
        needle.position.x,
        needle.position.y,
        needle.radius * 1.9,
        0.8,
        Color::new(1.0, 1.0, 1.0, 0.25),
    );
}

fn draw_inferno_meteor(meteor: InfernoFireball) {
    let fall = (meteor.arming * 0.25).clamp(0.1, 0.8);
    let flame = if meteor.arming <= 0.0 {
        CINDER_ORANGE
    } else {
        Color::new(
            CINDER_ORANGE.r,
            CINDER_ORANGE.g,
            CINDER_ORANGE.b,
            0.45 + fall,
        )
    };

    let length = meteor.velocity.length().max(1.0);
    let tail = if length > 0.0 {
        meteor.velocity / length * (meteor.timer * 1.5 + 6.0)
    } else {
        Vec2::ZERO
    };
    draw_line(
        meteor.position.x - tail.x,
        meteor.position.y - tail.y,
        meteor.position.x,
        meteor.position.y,
        2.2,
        Color::new(CINDER_ORANGE.r, CINDER_ORANGE.g, CINDER_ORANGE.b, 0.14),
    );
    draw_circle(
        meteor.position.x,
        meteor.position.y,
        4.4 + fall * 1.8,
        flame,
    );
    draw_circle_lines(meteor.position.x, meteor.position.y, 6.6, 1.2, flame);
}

fn draw_gravity_well(well: GravityWell, time: f32) {
    let ratio = (well.life / well.max_life).clamp(0.0, 1.0);
    let radius = well.max_radius * (0.18 + 0.82 * (1.0 - ratio));
    let pulse = (time * 4.0 + well.center.x * 0.003).sin().abs() * 0.16 + 0.84;
    draw_circle(
        well.center.x,
        well.center.y,
        radius * pulse,
        Color::new(
            RIFT_CYAN.r,
            RIFT_CYAN.g,
            RIFT_CYAN.b,
            0.08 + (1.0 - ratio) * 0.12,
        ),
    );
    draw_circle_lines(
        well.center.x,
        well.center.y,
        radius * 0.74,
        1.8,
        Color::new(
            RIFT_CYAN.r,
            RIFT_CYAN.g,
            RIFT_CYAN.b,
            0.9 * (1.0 - ratio).max(0.12),
        ),
    );
    for ring in 0..4u8 {
        let ring_radius = radius * (0.66 + ring as f32 * 0.26);
        draw_circle_lines(
            well.center.x,
            well.center.y,
            ring_radius,
            0.8,
            Color::new(
                RIFT_CYAN.r,
                RIFT_CYAN.g,
                RIFT_CYAN.b,
                0.05 + (ring as f32 * 0.012),
            ),
        );
    }
    let spin = (well.center.x * 0.013 + well.center.y * 0.009 + time).sin() * 0.3;
    for petal in 0..5u8 {
        let angle = spin + petal as f32 * 2.0 * PI / 5.0;
        draw_line(
            well.center.x + angle.cos() * (radius * 0.45),
            well.center.y + angle.sin() * (radius * 0.45),
            well.center.x + angle.cos() * (radius * 1.05),
            well.center.y + angle.sin() * (radius * 1.05),
            0.8,
            Color::new(1.0, 1.0, 1.0, 0.22),
        );
    }
}

fn draw_sigil_thread(thread: SigilThread, center: Vec2, time: f32) {
    let position = center + Vec2::from_angle(thread.angle + time * 0.08) * thread.orbit_radius;
    draw_line(
        center.x,
        center.y,
        position.x,
        position.y,
        1.1,
        Color::new(
            PHANTOM_NET_GREEN.r,
            PHANTOM_NET_GREEN.g,
            PHANTOM_NET_GREEN.b,
            0.24,
        ),
    );
    draw_circle(
        position.x,
        position.y,
        thread.arming * 0.8 + 3.2,
        Color::new(
            PHANTOM_NET_GREEN.r,
            PHANTOM_NET_GREEN.g,
            PHANTOM_NET_GREEN.b,
            0.52,
        ),
    );
    draw_circle_lines(
        position.x,
        position.y,
        3.0 + thread.blast_radius * 0.12,
        0.9,
        Color::new(
            PHANTOM_NET_GREEN.r,
            PHANTOM_NET_GREEN.g,
            PHANTOM_NET_GREEN.b,
            0.28,
        ),
    );
    draw_circle_lines(
        center.x,
        center.y,
        thread.orbit_radius,
        1.0,
        Color::new(
            PHANTOM_NET_GREEN.r,
            PHANTOM_NET_GREEN.g,
            PHANTOM_NET_GREEN.b,
            0.09,
        ),
    );
}

fn draw_phantom_tether(tether: PhantomTether, center: Vec2, time: f32) {
    let angle = tether.angle + time * 0.02;
    let position = center + Vec2::from_angle(angle) * tether.orbit_radius;
    draw_circle_lines(
        center.x,
        center.y,
        tether.orbit_radius,
        1.2,
        Color::new(
            PHANTOM_NET_GREEN.r,
            PHANTOM_NET_GREEN.g,
            PHANTOM_NET_GREEN.b,
            (0.16 + tether.arming * 0.6).min(0.7) * 0.55,
        ),
    );
    draw_line(
        center.x,
        center.y,
        position.x,
        position.y,
        1.1,
        Color::new(
            PHANTOM_NET_GREEN.r,
            PHANTOM_NET_GREEN.g,
            PHANTOM_NET_GREEN.b,
            0.26,
        ),
    );
    draw_circle(
        position.x,
        position.y,
        4.8,
        Color::new(
            PHANTOM_NET_GREEN.r,
            PHANTOM_NET_GREEN.g,
            PHANTOM_NET_GREEN.b,
            0.48,
        ),
    );
    draw_circle_lines(
        position.x,
        position.y,
        4.8 + (time * 2.0 + angle).sin().abs() * 1.8,
        1.0,
        Color::new(1.0, 1.0, 1.0, 0.4),
    );
}

fn draw_luminous_lance_bolt(bolt: LuminousLanceBolt) {
    let direction = bolt.velocity.normalize_or_zero();
    let tail = direction * (bolt.radius * 5.4);
    let start = bolt.position - tail;
    draw_line(
        start.x,
        start.y,
        bolt.position.x,
        bolt.position.y,
        1.8,
        Color::new(LANCE_AZURE.r, LANCE_AZURE.g, LANCE_AZURE.b, 0.17),
    );
    draw_line(
        start.x,
        start.y,
        bolt.position.x,
        bolt.position.y,
        (bolt.radius * 0.9).max(1.0),
        LANCE_AZURE,
    );
    draw_circle(
        bolt.position.x,
        bolt.position.y,
        bolt.radius * 1.4,
        LANCE_AZURE,
    );
}

fn draw_temporal_rift_well(well: TemporalRiftWell, time: f32) {
    let ratio = (well.life / well.max_life).clamp(0.0, 1.0);
    let radius = well.max_radius * (0.18 + 0.82 * (1.0 - ratio));
    let pulse = (time * 3.0 + well.center.x * 0.004).sin().abs() * 0.12 + 0.9;
    draw_circle(
        well.center.x,
        well.center.y,
        radius * pulse,
        Color::new(
            RIFT_CYAN.r,
            RIFT_CYAN.g,
            RIFT_CYAN.b,
            (0.05 + (1.0 - ratio) * 0.08).min(0.2),
        ),
    );
    draw_circle_lines(
        well.center.x,
        well.center.y,
        radius * 0.74,
        2.1,
        Color::new(
            RIFT_CYAN.r,
            RIFT_CYAN.g,
            RIFT_CYAN.b,
            0.75 * (1.0 - ratio).max(0.1),
        ),
    );
    for ring in 0..3u8 {
        let ring_radius = radius * (0.64 + ring as f32 * 0.24);
        draw_circle_lines(
            well.center.x,
            well.center.y,
            ring_radius,
            1.0,
            Color::new(
                RIFT_CYAN.r,
                RIFT_CYAN.g,
                RIFT_CYAN.b,
                0.08 * (1.0 - ring as f32 * 0.2),
            ),
        );
    }
}

fn draw_gloom_shard(shard: GloomShard) {
    let direction = shard.velocity.normalize_or_zero();
    let trail = direction * (shard.radius * 4.3);
    let start = shard.position - trail;
    draw_line(
        start.x,
        start.y,
        shard.position.x,
        shard.position.y,
        1.4,
        Color::new(GLOOM_PURPLE.r, GLOOM_PURPLE.g, GLOOM_PURPLE.b, 0.16),
    );
    draw_circle(
        shard.position.x,
        shard.position.y,
        shard.radius * 1.6,
        GLOOM_PURPLE,
    );
    draw_circle_lines(
        shard.position.x,
        shard.position.y,
        shard.radius * 2.0,
        0.8,
        Color::new(1.0, 1.0, 1.0, 0.42),
    );
}

fn draw_flare_pulse(pulse: FlarePulse, time: f32) {
    let life = pulse.life.max(0.0);
    let ratio = (life / 0.34).clamp(0.0, 1.0);
    let radius = pulse.max_radius * (1.0 - ratio);
    let flicker = (time * 40.0 + pulse.position.y * 0.02).sin().abs();
    draw_circle(
        pulse.position.x,
        pulse.position.y,
        radius,
        Color::new(
            CINDER_ORANGE.r,
            CINDER_ORANGE.g,
            CINDER_ORANGE.b,
            0.24 * ratio * flicker,
        ),
    );
    draw_circle_lines(
        pulse.position.x,
        pulse.position.y,
        radius,
        2.5,
        Color::new(
            CINDER_ORANGE.r,
            CINDER_ORANGE.g,
            CINDER_ORANGE.b,
            0.85 * ratio,
        ),
    );
}

fn draw_wraith_trace(trace: WraithTrace) {
    let alpha = trace.life.clamp(0.0, 1.0);
    draw_line(
        trace.start.x,
        trace.start.y,
        trace.end.x,
        trace.end.y,
        trace.width,
        Color::new(
            WRAITH_AMETHYST.r,
            WRAITH_AMETHYST.g,
            WRAITH_AMETHYST.b,
            0.14 * alpha,
        ),
    );
    draw_line(
        trace.start.x,
        trace.start.y,
        trace.end.x,
        trace.end.y,
        (trace.width * 0.4).max(1.5),
        Color::new(
            WRAITH_AMETHYST.r,
            WRAITH_AMETHYST.g,
            WRAITH_AMETHYST.b,
            0.75 * alpha,
        ),
    );
}

fn draw_impact_ring(ring: ImpactRing, time: f32) {
    let ratio = (ring.life / super::IMPACT_RING_LIFE).clamp(0.0, 1.0);
    let radius = ring.max_radius * (1.0 - ratio);
    let pulse = (time * 16.0 + ring.position.y * 0.02).sin().abs() * 0.12 + 1.0;
    draw_circle(
        ring.position.x,
        ring.position.y,
        radius * pulse,
        Color::new(
            ring.color.r,
            ring.color.g,
            ring.color.b,
            0.08 * ratio * ratio,
        ),
    );
    draw_circle_lines(
        ring.position.x,
        ring.position.y,
        radius * 0.72 * pulse,
        1.8,
        Color::new(
            ring.color.r,
            ring.color.g,
            ring.color.b,
            0.95 * (1.0 - ratio),
        ),
    );
}

fn draw_aether_spear(projectile: AetherSpearProjectile) {
    let direction = projectile.velocity.normalize_or_zero();
    let tail = projectile.radius * 3.4;
    let previous = projectile.position - direction * tail;
    draw_line(
        previous.x,
        previous.y,
        projectile.position.x,
        projectile.position.y,
        projectile.radius * 2.0,
        Color::new(AETHER_MAUVE.r, AETHER_MAUVE.g, AETHER_MAUVE.b, 0.20),
    );
    draw_line(
        previous.x,
        previous.y,
        projectile.position.x,
        projectile.position.y,
        (projectile.radius * 0.8).max(1.0),
        AETHER_MAUVE,
    );
    draw_circle(
        projectile.position.x,
        projectile.position.y,
        projectile.radius * 1.5,
        AETHER_MAUVE,
    );
}

fn draw_rift_echo(echo: RiftEcho, time: f32) {
    let progress = (1.0 - echo.life / echo.max_life).clamp(0.0, 1.0);
    let radius = echo.max_radius * (echo.range_scale * progress);
    let flicker = (time * 18.0 + echo.center.x * 0.02).sin().abs() * 0.2 + 1.0;
    draw_circle(
        echo.center.x,
        echo.center.y,
        radius,
        Color::new(
            RIFT_VIOLET.r,
            RIFT_VIOLET.g,
            RIFT_VIOLET.b,
            0.06 * progress * (1.0 - progress) * flicker,
        ),
    );
    draw_circle_lines(
        echo.center.x,
        echo.center.y,
        radius,
        1.7,
        Color::new(
            RIFT_VIOLET.r,
            RIFT_VIOLET.g,
            RIFT_VIOLET.b,
            0.65 * (1.0 - progress),
        ),
    );
}

fn draw_echo_shard(shard: EchoShard) {
    let velocity = shard.velocity.length().max(1.0);
    let tail = shard.velocity.normalize_or_zero() * (velocity * 0.08);
    let start = shard.position - tail;
    draw_line(
        start.x,
        start.y,
        shard.position.x,
        shard.position.y,
        1.6,
        Color::new(
            PHANTOM_CERULEAN.r,
            PHANTOM_CERULEAN.g,
            PHANTOM_CERULEAN.b,
            0.20,
        ),
    );
    draw_circle(
        shard.position.x,
        shard.position.y,
        shard.radius * 1.2,
        PHANTOM_CERULEAN,
    );
    draw_circle_lines(
        shard.position.x,
        shard.position.y,
        shard.radius * 2.1,
        0.9,
        Color::new(
            PHANTOM_CERULEAN.r,
            PHANTOM_CERULEAN.g,
            PHANTOM_CERULEAN.b,
            0.7,
        ),
    );
    draw_circle_lines(
        shard.position.x,
        shard.position.y,
        shard.radius * 2.1 + (1.0 - shard.bounces_left as f32 * 0.16),
        0.7,
        Color::new(1.0, 1.0, 1.0, 0.16),
    );
}

fn draw_rune_pulse(pulse: RunePulse, time: f32) {
    let ratio = (pulse.life / pulse.max_life).clamp(0.0, 1.0);
    let radius = pulse.radius * (1.0 + (1.0 - ratio) * 0.55);
    let flicker = (time * 14.0 + pulse.position.x * 0.03).sin().abs() * 0.2 + 1.0;
    draw_circle(
        pulse.position.x,
        pulse.position.y,
        radius,
        Color::new(
            PHANTOM_EMBER.r,
            PHANTOM_EMBER.g,
            PHANTOM_EMBER.b,
            0.06 * ratio * flicker,
        ),
    );
    draw_circle_lines(
        pulse.position.x,
        pulse.position.y,
        radius * 0.76,
        2.0,
        Color::new(
            PHANTOM_EMBER.r,
            PHANTOM_EMBER.g,
            PHANTOM_EMBER.b,
            0.62 * (1.0 - ratio),
        ),
    );
}

fn draw_harrow_dart(dart: HarrowDart, time: f32) {
    let pulse = (time * 90.0 + dart.life * 100.0).sin().abs();
    let tail = dart.radius * 2.6;
    let previous = dart.position - dart.velocity.normalize_or_zero() * tail;
    draw_line(
        previous.x,
        previous.y,
        dart.position.x,
        dart.position.y,
        dart.radius * 2.0,
        Color::new(
            WRAITH_AMETHYST.r,
            WRAITH_AMETHYST.g,
            WRAITH_AMETHYST.b,
            0.16,
        ),
    );
    draw_line(
        previous.x,
        previous.y,
        dart.position.x,
        dart.position.y,
        (dart.radius * 0.7).max(1.0),
        Color::new(
            WRAITH_AMETHYST.r,
            WRAITH_AMETHYST.g,
            WRAITH_AMETHYST.b,
            0.35 + pulse * 0.35,
        ),
    );
    draw_circle(
        dart.position.x,
        dart.position.y,
        dart.radius * 1.3,
        WRAITH_AMETHYST,
    );
    draw_circle_lines(
        dart.position.x,
        dart.position.y,
        dart.radius * 1.8,
        0.8,
        Color::new(1.0, 1.0, 1.0, 0.4),
    );
}

fn draw_gem(gem: Gem, time: f32) {
    let pulse = 1.0 + (time * 6.0 + gem.position.x * 0.02).sin() * 0.12;
    let size = if gem.value > 2 { 10.0 } else { 7.0 } * pulse;
    draw_circle(
        gem.position.x,
        gem.position.y,
        size * 1.7,
        Color::new(0.25, 0.91, 0.68, 0.10),
    );
    draw_poly(
        gem.position.x,
        gem.position.y,
        4,
        size,
        FRAC_PI_2,
        GEM_GREEN,
    );
    draw_poly_lines(
        gem.position.x,
        gem.position.y,
        4,
        size,
        FRAC_PI_2,
        1.5,
        BONE,
    );
}

fn draw_lightning(lightning: &Lightning, time: f32) {
    let alpha = (lightning.life / 0.18).clamp(0.0, 1.0);
    for (segment, pair) in lightning.points.windows(2).enumerate() {
        let start = pair[0];
        let end = pair[1];
        let direction = end - start;
        let perpendicular = vec2(-direction.y, direction.x).normalize_or_zero();
        let mut previous = start;
        for step in 1_usize..=7 {
            let progress = step as f32 / 7.0;
            let jitter = if step == 7 {
                0.0
            } else {
                (time * 97.0 + step as f32 * 13.7 + start.x * 0.03).sin() * 10.0
            };
            let next = start + direction * progress + perpendicular * jitter;
            draw_line(
                previous.x,
                previous.y,
                next.x,
                next.y,
                10.0,
                Color::new(0.40, 0.35, 1.0, 0.08 * alpha),
            );
            draw_line(
                previous.x,
                previous.y,
                next.x,
                next.y,
                3.0,
                Color::new(0.35, 0.90, 1.0, alpha),
            );
            draw_line(
                previous.x,
                previous.y,
                next.x,
                next.y,
                1.0,
                Color::new(1.0, 1.0, 1.0, alpha),
            );
            if step < 7 && step.is_multiple_of(2) {
                let side = if (segment + step).is_multiple_of(2) {
                    1.0
                } else {
                    -1.0
                };
                let branch = next + perpendicular * side * (13.0 + step as f32 * 1.5);
                draw_line(
                    next.x,
                    next.y,
                    branch.x,
                    branch.y,
                    5.0,
                    Color::new(0.35, 0.75, 1.0, 0.08 * alpha),
                );
                draw_line(
                    next.x,
                    next.y,
                    branch.x,
                    branch.y,
                    1.2,
                    Color::new(0.72, 0.96, 1.0, 0.75 * alpha),
                );
            }
            previous = next;
        }
    }

    for point in lightning.points.iter().skip(1) {
        draw_circle(
            point.x,
            point.y,
            11.0 * alpha,
            Color::new(STORM_CYAN.r, STORM_CYAN.g, STORM_CYAN.b, 0.10 * alpha),
        );
        draw_circle_lines(
            point.x,
            point.y,
            5.0 + 7.0 * alpha,
            1.5,
            Color::new(0.80, 0.98, 1.0, 0.75 * alpha),
        );
    }
}

fn draw_damage_number(number: &DamageNumber) {
    let alpha = (number.life / DAMAGE_NUMBER_LIFE).clamp(0.0, 1.0);
    let size = 18.0 + alpha * 5.0;
    let text = number.value.to_string();
    draw_text_center(
        &text,
        number.position.x + 2.0,
        number.position.y + 2.0,
        size,
        Color::new(0.0, 0.0, 0.0, alpha * 0.75),
    );
    draw_text_center(
        &text,
        number.position.x,
        number.position.y,
        size,
        Color::new(number.color.r, number.color.g, number.color.b, alpha),
    );
}

fn draw_vignette(health_ratio: f32, time: f32, menace: f32, color: Color) {
    for layer in 0..5 {
        let inset = layer as f32 * 13.0;
        let alpha = (5 - layer) as f32 * 0.018;
        draw_rectangle_lines(
            inset,
            inset,
            screen_width() - inset * 2.0,
            screen_height() - inset * 2.0,
            28.0,
            Color::new(0.01, 0.008, 0.025, alpha),
        );
    }

    if health_ratio < 0.35 {
        let pulse = 0.10 + (time * 6.0).sin().abs() * 0.08;
        draw_rectangle_lines(
            5.0,
            5.0,
            screen_width() - 10.0,
            screen_height() - 10.0,
            24.0,
            Color::new(DAMAGE_RED.r, DAMAGE_RED.g, DAMAGE_RED.b, pulse),
        );
    }

    let focus = screen_width().max(screen_height()) * 0.62;
    let bloom = 0.06 + menace * 0.20;
    let film = bloom * (0.3 + (time * 1.2).sin().abs() * 0.6);
    draw_circle(
        screen_width() * 0.5,
        screen_height() * 0.5,
        focus,
        Color::new(color.r, color.g, color.b, film * 0.16),
    );

    let seed = (time * 70.0) as i32;
    for grain in 0..56 {
        let hash = hash_cell(seed + grain, seed ^ grain);
        let x = (hash & 0xFFFF) as f32 / 65_535.0 * screen_width();
        let y = ((hash >> 16) & 0xFFFF) as f32 / 65_535.0 * screen_height();
        draw_circle(
            x,
            y,
            0.9,
            Color::new(1.0, 1.0, 1.0, (hash % 23) as f32 / 260.0 * 0.2),
        );
    }
}

fn draw_text_center(text: &str, center_x: f32, baseline: f32, size: f32, color: Color) {
    let dimensions = measure_text(text, None, size as u16, 1.0);
    draw_text(
        text,
        center_x - dimensions.width * 0.5,
        baseline,
        size,
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_camera_uses_screen_style_vertical_axis() {
        assert!(world_camera(Vec2::ZERO, 1280.0).zoom.y > 0.0);
    }
}
