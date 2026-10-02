//! Procedural rendering for [`Game`].
//!
//! World drawing uses a camera centered on the player; overlays switch back to
//! screen coordinates. Drawing reads simulation state; the frame's [`Input`]
//! supplies device state.

mod text;
mod ui;

use std::f32::consts::TAU;

use macroquad::prelude::{
    Camera2D, Color, Rect, Vec2, clear_background, draw_circle, draw_circle_lines, draw_ellipse,
    draw_line, draw_poly, draw_poly_lines, draw_rectangle, draw_rectangle_lines, draw_triangle,
    set_camera, set_default_camera, vec2,
};

pub(crate) use text::TextCache;
pub(super) use ui::upgrade_at;

use text::draw_world_text_center;

use super::{
    ARCANE_VIOLET, BACKGROUND, BONE, DAMAGE_NUMBER_LIFE, DAMAGE_RED, DamageNumber, Enemy,
    EnemyKind, GEM_GREEN, Game, Gem, INK, Input, LIGHTNING_LIFE, Lightning, MOON_GOLD, Phase,
    STORM_CYAN, VIEW_HEIGHT, view_width,
};

const GROUND: Color = Color::new(0.060, 0.050, 0.105, 1.0);
const GROUND_ALT: Color = Color::new(0.070, 0.058, 0.120, 1.0);
const GROUND_LINE: Color = Color::new(0.16, 0.13, 0.24, 0.34);

impl Game {
    /// Draws the current frame without mutating simulation state.
    ///
    /// Prepare [`TextCache`] before drawing the first frame or changing DPI.
    pub(crate) fn draw(&self, input: &Input) {
        clear_background(BACKGROUND);

        if self.phase == Phase::Title {
            self.draw_title(input.viewport);
            return;
        }

        let shake = if self.shake > 0.0 {
            vec2(
                (self.visual_time * 83.0).sin(),
                (self.visual_time * 71.0).cos(),
            ) * self.shake
        } else {
            Vec2::ZERO
        };
        let camera_center = self.camera + shake;
        let width = view_width(input.viewport);
        let camera = world_camera(camera_center, width);
        set_camera(&camera);
        self.draw_world(camera_center, width);
        set_default_camera();
        draw_vignette(
            self.player.health / self.player.max_health,
            self.visual_time,
            input.viewport,
        );
        self.draw_hud(input.viewport);

        match self.phase {
            Phase::Paused => Self::draw_pause(input.viewport),
            Phase::LevelUp(offers) => Self::draw_level_up(offers, input.viewport, input.pointer),
            Phase::GameOver => self.draw_game_over(input.viewport),
            Phase::Title | Phase::Running => {},
        }
    }

    fn draw_world(&self, camera_center: Vec2, width: f32) {
        Self::draw_ground(camera_center, width);

        for gem in &self.gems {
            draw_gem(gem, self.visual_time);
        }
        for particle in &self.particles {
            let alpha = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let radius = particle.size * alpha.sqrt();
            draw_circle(
                particle.position.x,
                particle.position.y,
                radius * 2.4,
                Color {
                    a: alpha * 0.10,
                    ..particle.color
                },
            );
            draw_circle(
                particle.position.x,
                particle.position.y,
                radius,
                Color {
                    a: alpha,
                    ..particle.color
                },
            );
        }
        for enemy in &self.enemies {
            draw_enemy(enemy, self.visual_time);
        }
        self.draw_player();
        self.draw_moon_knives();
        for lightning in &self.lightning {
            draw_lightning(lightning, self.visual_time);
        }
        for number in &self.damage_numbers {
            draw_damage_number(number);
        }
    }

    fn draw_ground(camera_center: Vec2, width: f32) {
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
                } else if hash.is_multiple_of(5) {
                    let star = position + vec2((hash % 96) as f32, ((hash >> 7) % 96) as f32);
                    draw_circle(star.x, star.y, 1.5, Color::new(0.45, 0.38, 0.65, 0.38));
                }
            }
        }
    }

    fn draw_player(&self) {
        let bob = (self.visual_time * 7.5).sin() * 2.0;
        let lean = self.player.velocity.normalize_or_zero() * 2.5;
        let position = self.player.position + vec2(lean.x, bob + lean.y);
        let hurt_flash = self.player.invulnerability > 0.0
            && ((self.player.invulnerability * 18.0) as u32).is_multiple_of(2);
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

        self.draw_storm_lantern(position, bob, hood);

        draw_circle_lines(
            position.x,
            position.y,
            self.player.pickup_radius,
            1.0,
            Color::new(0.45, 0.84, 0.90, 0.055),
        );
    }

    fn draw_storm_lantern(&self, player_position: Vec2, bob: f32, hood: Color) {
        let lantern = self.player.lantern_position() + vec2(0.0, bob);
        let storm_charge = (1.0 - self.storm_timer / self.storm.cooldown).clamp(0.0, 1.0);
        let glow = 9.0 + storm_charge * 7.0;
        let pulse = (self.visual_time * 7.0).sin() * 2.0;
        draw_circle(
            lantern.x,
            lantern.y,
            glow + 8.0,
            Color {
                a: 0.04 + storm_charge * 0.08,
                ..STORM_CYAN
            },
        );
        for ring in 0..2 {
            draw_circle_lines(
                lantern.x,
                lantern.y,
                glow + pulse + ring as f32 * 7.0,
                1.0,
                Color {
                    a: (0.09 + storm_charge * 0.18) / (ring + 1) as f32,
                    ..STORM_CYAN
                },
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
                Color {
                    a: 0.08 + storm_charge * 0.20,
                    ..STORM_CYAN
                },
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
            Color {
                a: 0.18,
                ..MOON_GOLD
            },
        );
        draw_circle_lines(
            self.player.position.x,
            self.player.position.y,
            self.moon.radius - 6.0 - orbit_pulse,
            1.0,
            Color { a: 0.06, ..BONE },
        );
        for index in 0_usize..8 {
            let angle = -self.moon_angle * 0.18 + index as f32 * TAU / 8.0;
            let marker = self.player.position + Vec2::from_angle(angle) * self.moon.radius;
            draw_circle(
                marker.x,
                marker.y,
                if index.is_multiple_of(2) { 2.2 } else { 1.2 },
                Color {
                    a: 0.30,
                    ..MOON_GOLD
                },
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
            Color {
                a: 0.08,
                ..MOON_GOLD
            },
        );
        draw_line(
            trail.x,
            trail.y,
            position.x,
            position.y,
            3.0,
            Color {
                a: 0.24,
                ..MOON_GOLD
            },
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
                Color {
                    a: alpha,
                    ..MOON_GOLD
                },
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
            Color {
                a: 0.12,
                ..MOON_GOLD
            },
        );
        draw_triangle(
            position + direction * 21.0,
            position - direction * 14.0 + tangent * 8.0,
            position - direction * 14.0 - tangent * 8.0,
            Color {
                a: 0.32,
                ..MOON_GOLD
            },
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
fn hash_cell(x: i32, y: i32) -> u32 {
    let mut value =
        x.cast_unsigned().wrapping_mul(0x9E37_79B9) ^ y.cast_unsigned().wrapping_mul(0x85EB_CA6B);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value
}

fn draw_enemy(enemy: &Enemy, time: f32) {
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
                Color { a: 0.08, ..color },
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
            draw_circle(position.x, position.y, 23.0, Color { a: 0.08, ..color });
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
                Color { a: 0.45, ..color },
            );
            draw_circle(
                position.x,
                position.y,
                9.0,
                Color::new(0.72, 0.95, 0.94, 1.0),
            );
            draw_circle(position.x, position.y, 3.0, INK);
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
    }
}

fn draw_gem(gem: &Gem, time: f32) {
    let pulse = 1.0 + (time * 6.0 + gem.position.x * 0.02).sin() * 0.12;
    let size = 10.0 * pulse;
    draw_circle(
        gem.position.x,
        gem.position.y,
        size * 1.7,
        Color::new(0.25, 0.91, 0.68, 0.10),
    );
    draw_poly(gem.position.x, gem.position.y, 4, size, 0.0, GEM_GREEN);
    draw_poly_lines(gem.position.x, gem.position.y, 4, size, 0.0, 1.5, BONE);
}

fn draw_lightning(lightning: &Lightning, time: f32) {
    let alpha = (lightning.life / LIGHTNING_LIFE).clamp(0.0, 1.0);
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
            Color {
                a: 0.10 * alpha,
                ..STORM_CYAN
            },
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
    draw_world_text_center(
        &text,
        number.position.x + 2.0,
        number.position.y + 2.0,
        size,
        Color::new(0.0, 0.0, 0.0, alpha * 0.75),
    );
    draw_world_text_center(
        &text,
        number.position.x,
        number.position.y,
        size,
        Color {
            a: alpha,
            ..number.color
        },
    );
}

fn draw_vignette(health_ratio: f32, time: f32, viewport: Vec2) {
    for layer in 0..5 {
        let inset = layer as f32 * 13.0;
        let alpha = (5 - layer) as f32 * 0.018;
        draw_rectangle_lines(
            inset,
            inset,
            (viewport.x - inset * 2.0).max(0.0),
            (viewport.y - inset * 2.0).max(0.0),
            28.0,
            Color::new(0.01, 0.008, 0.025, alpha),
        );
    }

    if health_ratio < 0.35 {
        let pulse = 0.10 + (time * 6.0).sin().abs() * 0.08;
        draw_rectangle_lines(
            5.0,
            5.0,
            (viewport.x - 10.0).max(0.0),
            (viewport.y - 10.0).max(0.0),
            24.0,
            Color {
                a: pulse,
                ..DAMAGE_RED
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::world_camera;
    use macroquad::prelude::Vec2;

    #[test]
    fn world_camera_uses_screen_style_vertical_axis() {
        assert!(world_camera(Vec2::ZERO, 1280.0).zoom.y > 0.0);
    }
}
