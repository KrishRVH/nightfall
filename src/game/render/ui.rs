//! Screen-space HUD, menus, and upgrade presentation.

use macroquad::prelude::{
    Color, Rect, Vec2, draw_circle, draw_circle_lines, draw_line, draw_rectangle,
    draw_rectangle_lines, draw_triangle, vec2,
};

use super::{
    hash_cell,
    text::{draw_text, draw_text_center, draw_text_right, fitted_text_size},
};
use crate::game::{
    ARCANE_VIOLET, BACKGROUND, BONE, DAMAGE_RED, GEM_GREEN, Game, INK, MOON_GOLD,
    PICKUP_FLASH_LIFE, STORM_CYAN, Upgrade, format_time,
};

const PANEL: Color = Color::new(0.075, 0.060, 0.125, 0.94);
const MUTED: Color = Color::new(0.60, 0.55, 0.70, 1.0);

impl Upgrade {
    const fn name(self) -> &'static str {
        match self {
            Self::ExtraKnife => "Another Moon",
            Self::SharpenedMoon => "Honed Crescent",
            Self::WiderOrbit => "Long Eclipse",
            Self::FastStorm => "Quickened Spark",
            Self::ForkedStorm => "Forked Omen",
            Self::PotentStorm => "Thunderheart",
            Self::Fleet => "Fleetfoot",
            Self::Vitality => "Blood Ward",
            Self::Magnet => "Grave Pull",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::ExtraKnife => "+1 orbiting moon knife",
            Self::SharpenedMoon => "+50% moon knife damage",
            Self::WiderOrbit => "+14 orbit radius and +18% speed",
            Self::FastStorm => "28% shorter storm cooldown",
            Self::ForkedStorm => "+2 chains and +40 reach",
            Self::PotentStorm => "+50% storm lantern damage",
            Self::Fleet => "+18% movement speed",
            Self::Vitality => "+35 max health and healing",
            Self::Magnet => "+45% pickup reach",
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::ExtraKnife | Self::SharpenedMoon | Self::WiderOrbit => MOON_GOLD,
            Self::FastStorm | Self::ForkedStorm | Self::PotentStorm => STORM_CYAN,
            Self::Fleet | Self::Vitality | Self::Magnet => GEM_GREEN,
        }
    }

    const fn school(self) -> &'static str {
        match self {
            Self::ExtraKnife | Self::SharpenedMoon | Self::WiderOrbit => "MOON KNIVES",
            Self::FastStorm | Self::ForkedStorm | Self::PotentStorm => "STORM LANTERN",
            Self::Fleet | Self::Vitality | Self::Magnet => "WITCHCRAFT",
        }
    }
}

impl Game {
    pub(super) fn draw_hud(&self, viewport: Vec2) {
        self.draw_run_header(viewport);
        self.draw_loadout(viewport);
    }

    fn draw_run_header(&self, viewport: Vec2) {
        let width = viewport.x;
        let compact = width < 720.0;
        let hud_text_size = if compact { 16.0 } else { 20.0 };
        let experience_ratio = self.player.experience as f32 / self.player.next_level as f32;
        draw_panel(Rect::new(14.0, 14.0, width - 28.0, 47.0));
        draw_bar(
            18.0,
            18.0,
            width - 36.0,
            8.0,
            experience_ratio,
            ARCANE_VIOLET,
            Color::new(0.0, 0.0, 0.0, 0.50),
        );
        if self.pickup_flash > 0.0 {
            draw_rectangle_lines(
                18.0,
                18.0,
                width - 36.0,
                8.0,
                2.0,
                Color {
                    a: self.pickup_flash / PICKUP_FLASH_LIFE,
                    ..GEM_GREEN
                },
            );
        }
        draw_text(
            format!("LEVEL {:02}", self.player.level),
            22.0,
            50.0,
            hud_text_size,
            BONE,
        );
        draw_text_center(
            &format_time(self.elapsed),
            width * 0.5,
            51.0,
            if compact { 20.0 } else { 25.0 },
            BONE,
        );
        draw_text_right(
            &if compact {
                format!("{:03} KOs", self.kills)
            } else {
                format!("{:03} BANISHED", self.kills)
            },
            width - 22.0,
            50.0,
            hud_text_size,
            BONE,
        );
    }

    fn draw_loadout(&self, viewport: Vec2) {
        let width = viewport.x;
        let height = viewport.y;
        let compact = width < 720.0;
        let (health, weapon_y, weapon_width, moon_x, storm_x) = if compact {
            let weapon_width = (width - 42.0) * 0.5;
            (
                Rect::new(12.0, height - 72.0, width - 24.0, 58.0),
                height - 139.0,
                weapon_width,
                12.0,
                24.0 + weapon_width,
            )
        } else {
            (
                Rect::new(16.0, height - 73.0, 252.0, 57.0),
                height - 73.0,
                140.0,
                width - 306.0,
                width - 156.0,
            )
        };
        draw_panel(health);
        draw_text("VITALITY", health.x + 9.0, health.y + 22.0, 14.0, MUTED);
        let health_ratio = self.player.health / self.player.max_health;
        draw_bar(
            health.x + 8.0,
            health.y + 30.0,
            health.w - 16.0,
            17.0,
            health_ratio,
            DAMAGE_RED,
            Color::new(0.0, 0.0, 0.0, 0.55),
        );
        draw_text(
            format!("{:.0} / {:.0}", self.player.health, self.player.max_health),
            health.x + 15.0,
            health.y + 44.0,
            14.0,
            BONE,
        );

        let storm_charge = 1.0 - self.storm_timer / self.storm.cooldown;
        draw_weapon_badge(
            moon_x,
            weapon_y,
            weapon_width,
            "MOON KNIVES",
            self.moon.level,
            MOON_GOLD,
            1.0,
        );
        draw_weapon_badge(
            storm_x,
            weapon_y,
            weapon_width,
            "STORM LANTERN",
            self.storm.level,
            STORM_CYAN,
            storm_charge,
        );
    }

    pub(super) fn draw_title(&self, viewport: Vec2) {
        let (center, scale) = title_layout(viewport);
        draw_title_background(viewport, center, self.visual_time, scale);
        draw_text_center(
            "NIGHTFALL",
            center.x,
            center.y + 15.0 * scale,
            72.0 * scale,
            BONE,
        );
        draw_text_center(
            "A tiny survival spellbook",
            center.x,
            center.y + 50.0 * scale,
            23.0 * scale,
            Color::new(0.68, 0.61, 0.80, 1.0),
        );

        let pulse = 0.75 + (self.visual_time * 3.0).sin() * 0.15;
        let prompt_width = (viewport.x - 32.0).min(420.0 * scale);
        draw_rectangle(
            center.x - prompt_width * 0.5,
            center.y + 91.0 * scale,
            prompt_width,
            54.0 * scale,
            Color::new(0.09, 0.07, 0.15, 0.72),
        );
        draw_rectangle_lines(
            center.x - prompt_width * 0.5,
            center.y + 91.0 * scale,
            prompt_width,
            54.0 * scale,
            1.5,
            Color {
                a: pulse * 0.65,
                ..MOON_GOLD
            },
        );
        draw_text_center(
            "PRESS ENTER OR CLICK TO BEGIN",
            center.x,
            center.y + 127.0 * scale,
            fitted_text_size(
                "PRESS ENTER OR CLICK TO BEGIN",
                25.0 * scale,
                prompt_width - 20.0 * scale,
            ),
            Color {
                a: pulse,
                ..MOON_GOLD
            },
        );
        if viewport.x < 720.0 {
            draw_text_center(
                "WASD / ARROWS TO MOVE",
                center.x,
                viewport.y - 62.0,
                fitted_text_size("WASD / ARROWS TO MOVE", 15.0, viewport.x - 24.0),
                MUTED,
            );
            draw_text_center(
                "WEAPONS AUTO-FIRE  ·  ESC PAUSES",
                center.x,
                viewport.y - 36.0,
                fitted_text_size("WEAPONS AUTO-FIRE  ·  ESC PAUSES", 15.0, viewport.x - 24.0),
                MUTED,
            );
        } else {
            draw_text_center(
                "WASD / ARROWS TO MOVE  ·  WEAPONS FIRE AUTOMATICALLY  ·  ESC TO PAUSE",
                center.x,
                viewport.y - 42.0,
                fitted_text_size(
                    "WASD / ARROWS TO MOVE  ·  WEAPONS FIRE AUTOMATICALLY  ·  ESC TO PAUSE",
                    17.0,
                    viewport.x - 32.0,
                ),
                MUTED,
            );
        }
    }

    pub(super) fn draw_pause(viewport: Vec2) {
        draw_scrim(viewport);
        let scale = menu_scale(viewport, vec2(1.0, 180.0));
        let panel_width = (viewport.x - 32.0).max(1.0).min(490.0 * scale);
        let panel = Rect::new(
            viewport.x * 0.5 - panel_width * 0.5,
            viewport.y * 0.5 - 90.0 * scale,
            panel_width,
            160.0 * scale,
        );
        draw_panel(panel);
        draw_text_center(
            "PAUSED",
            viewport.x * 0.5,
            viewport.y * 0.5 - 20.0 * scale,
            54.0 * scale,
            BONE,
        );
        draw_text_center(
            "Press Escape, P, Enter, or Space to continue",
            viewport.x * 0.5,
            viewport.y * 0.5 + 28.0 * scale,
            fitted_text_size(
                "Press Escape, P, Enter, or Space to continue",
                20.0 * scale,
                panel_width - 24.0 * scale,
            ),
            MOON_GOLD,
        );
    }

    pub(super) fn draw_level_up(offers: [Upgrade; 3], viewport: Vec2, pointer: Vec2) {
        draw_scrim(viewport);
        let layout = UpgradeLayout::new(viewport);
        draw_text_center(
            "THE NIGHT ANSWERS",
            viewport.x * 0.5,
            layout.title_y,
            (if layout.compact { 29.0 } else { 38.0 }) * layout.scale,
            BONE,
        );
        draw_text_center(
            "Choose one power to continue",
            viewport.x * 0.5,
            layout.subtitle_y,
            (if layout.compact { 14.0 } else { 17.0 }) * layout.scale,
            MUTED,
        );

        for (index, upgrade) in offers.into_iter().enumerate() {
            draw_upgrade_card(index, upgrade, &layout, pointer);
        }
    }

    pub(super) fn draw_game_over(&self, viewport: Vec2) {
        draw_scrim(viewport);
        let center_x = viewport.x * 0.5;
        let center_y = viewport.y * 0.5;
        let compact = viewport.x < 720.0;
        let scale = menu_scale(viewport, vec2(1.0, 310.0));
        let panel_width = (viewport.x - 32.0).max(1.0).min(580.0 * scale);
        draw_panel(Rect::new(
            center_x - panel_width * 0.5,
            center_y - 155.0 * scale,
            panel_width,
            270.0 * scale,
        ));
        draw_text_center(
            "DAWN FOUND YOU",
            center_x,
            center_y - 105.0 * scale,
            fitted_text_size("DAWN FOUND YOU", 47.0 * scale, panel_width - 24.0 * scale),
            BONE,
        );
        draw_text_center(
            &format!("You endured {}", format_time(self.elapsed)),
            center_x,
            center_y - 48.0 * scale,
            23.0 * scale,
            MOON_GOLD,
        );
        let result = if compact {
            format!("{} banished  ·  level {}", self.kills, self.player.level)
        } else {
            format!(
                "{} enemies banished  ·  level {} reached",
                self.kills, self.player.level
            )
        };
        draw_text_center(
            &result,
            center_x,
            center_y - 12.0 * scale,
            fitted_text_size(&result, 20.0 * scale, panel_width - 24.0 * scale),
            MUTED,
        );
        draw_text_center(
            "PRESS R, ENTER, OR CLICK TO TRY AGAIN",
            center_x,
            center_y + 72.0 * scale,
            fitted_text_size(
                "PRESS R, ENTER, OR CLICK TO TRY AGAIN",
                23.0 * scale,
                panel_width - 24.0 * scale,
            ),
            STORM_CYAN,
        );
    }
}

fn title_layout(viewport: Vec2) -> (Vec2, f32) {
    // The moon reaches 224 units above center; the prompt ends 145 below it.
    // Reserve the footer's fixed-size text before fitting that artwork.
    let footer_height = if viewport.x < 720.0 { 88.0 } else { 64.0 };
    let available_height = (viewport.y - footer_height - 32.0).max(1.0);
    let scale = (available_height / (224.0 + 145.0))
        .min((viewport.x - 32.0).max(1.0) / 320.0)
        .min(1.0);
    let center_y = (viewport.y * 0.5)
        .min(viewport.y - footer_height - 16.0 - 145.0 * scale)
        .max(16.0 + 224.0 * scale);
    (vec2(viewport.x * 0.5, center_y), scale)
}

fn draw_title_background(viewport: Vec2, center: Vec2, time: f32, scale: f32) {
    for index in 0..90 {
        let hash = hash_cell(index, index * 19);
        let position = vec2(
            (hash & 0xFFFF) as f32 / 65_535.0 * viewport.x,
            ((hash >> 16) & 0xFFFF) as f32 / 65_535.0 * viewport.y,
        );
        let twinkle = 0.25 + (time * 2.0 + index as f32).sin().abs() * 0.45;
        draw_circle(
            position.x,
            position.y,
            1.4,
            Color::new(0.75, 0.68, 0.95, twinkle),
        );
    }

    draw_circle(
        center.x,
        center.y - 126.0 * scale,
        82.0 * scale,
        Color::new(0.96, 0.84, 0.55, 0.10),
    );
    draw_circle_lines(
        center.x,
        center.y - 126.0 * scale,
        (95.0 + (time * 1.4).sin() * 3.0) * scale,
        1.0,
        Color {
            a: 0.18,
            ..MOON_GOLD
        },
    );
    draw_circle(center.x, center.y - 126.0 * scale, 59.0 * scale, BONE);
    draw_circle(
        center.x + 24.0 * scale,
        center.y - 145.0 * scale,
        59.0 * scale,
        BACKGROUND,
    );
}

/// Drawing and input use the same screen-space card bounds.
pub(in crate::game) fn upgrade_at(point: Vec2, viewport: Vec2) -> Option<usize> {
    UpgradeLayout::new(viewport)
        .cards
        .iter()
        .position(|card| card.contains(point))
}

fn draw_upgrade_card(index: usize, upgrade: Upgrade, layout: &UpgradeLayout, pointer: Vec2) {
    let card = layout.cards[index];
    let hovered = card.contains(pointer);
    let scale = layout.scale;
    let color = upgrade.color();

    draw_rectangle(
        card.x + 7.0 * scale,
        card.y + 9.0 * scale,
        card.w,
        card.h,
        Color::new(0.0, 0.0, 0.0, 0.30),
    );
    draw_rectangle(
        card.x,
        card.y,
        card.w,
        card.h,
        if hovered {
            Color::new(0.15, 0.12, 0.23, 0.99)
        } else {
            Color::new(0.085, 0.068, 0.145, 0.98)
        },
    );
    draw_rectangle(card.x, card.y, card.w, 5.0 * scale, color);
    draw_rectangle_lines(
        card.x,
        card.y,
        card.w,
        card.h,
        (if hovered { 3.0 } else { 1.5 }) * scale,
        Color {
            a: if hovered { 1.0 } else { 0.65 },
            ..color
        },
    );
    draw_rectangle(
        card.x + card.w - 40.0 * scale,
        card.y + 15.0 * scale,
        25.0 * scale,
        25.0 * scale,
        color,
    );
    draw_text_center(
        &(index + 1).to_string(),
        card.x + card.w - 27.5 * scale,
        card.y + 35.0 * scale,
        17.0 * scale,
        INK,
    );

    if layout.compact {
        draw_compact_upgrade(upgrade, card, scale);
    } else {
        draw_wide_upgrade(upgrade, card, scale);
    }
}

fn draw_compact_upgrade(upgrade: Upgrade, card: Rect, scale: f32) {
    draw_upgrade_icon(
        upgrade,
        vec2(card.x + 48.0 * scale, card.y + card.h * 0.5),
        scale,
    );
    let text_x = card.x + 86.0 * scale;
    let text_width = card.w - 104.0 * scale;
    draw_text(
        upgrade.school(),
        text_x,
        card.y + 32.0 * scale,
        12.0 * scale,
        upgrade.color(),
    );
    draw_text(
        upgrade.name(),
        text_x,
        card.y + 61.0 * scale,
        fitted_text_size(upgrade.name(), 19.0 * scale, text_width),
        BONE,
    );
    draw_text(
        upgrade.description(),
        text_x,
        card.y + 88.0 * scale,
        fitted_text_size(upgrade.description(), 14.0 * scale, text_width),
        MUTED,
    );
}

fn draw_wide_upgrade(upgrade: Upgrade, card: Rect, scale: f32) {
    let center_x = card.x + card.w * 0.5;
    let text_width = card.w - 24.0 * scale;
    draw_upgrade_icon(upgrade, vec2(center_x, card.y + 57.0 * scale), scale);
    draw_text_center(
        upgrade.school(),
        center_x,
        card.y + 96.0 * scale,
        13.0 * scale,
        upgrade.color(),
    );
    draw_text_center(
        upgrade.name(),
        center_x,
        card.y + 127.0 * scale,
        fitted_text_size(upgrade.name(), 21.0 * scale, text_width),
        BONE,
    );
    draw_text_center(
        upgrade.description(),
        center_x,
        card.y + 163.0 * scale,
        fitted_text_size(upgrade.description(), 15.0 * scale, text_width),
        MUTED,
    );
}

/// A pure layout keeps card hit testing independent of the graphics context.
struct UpgradeLayout {
    cards: [Rect; 3],
    title_y: f32,
    subtitle_y: f32,
    scale: f32,
    compact: bool,
}

impl UpgradeLayout {
    fn new(viewport: Vec2) -> Self {
        let compact = viewport.x < 720.0;
        let available_width = (viewport.x - 28.0).max(1.0);
        let available_height = (viewport.y - 48.0).max(1.0);
        let menu_height = if compact { 430.0 } else { 312.0 };
        let minimum_width = if compact { 320.0 } else { 684.0 };
        let scale = (available_height / menu_height)
            .min(available_width / minimum_width)
            .min(1.0);
        let top = (viewport.y - menu_height * scale) * 0.5;
        let cards = if compact {
            let width = available_width.min(520.0 * scale);
            std::array::from_fn(|index| {
                Rect::new(
                    (viewport.x - width) * 0.5,
                    top + (86.0 + index as f32 * 118.0) * scale,
                    width,
                    108.0 * scale,
                )
            })
        } else {
            let gap = 18.0 * scale;
            let total_width = viewport.x.min(930.0 * scale);
            let width = (total_width - gap * 4.0) / 3.0;
            let start_x = (viewport.x - total_width) * 0.5 + gap;
            std::array::from_fn(|index| {
                Rect::new(
                    start_x + index as f32 * (width + gap),
                    top + 122.0 * scale,
                    width,
                    190.0 * scale,
                )
            })
        };
        Self {
            cards,
            title_y: top + (if compact { 29.0 } else { 38.0 }) * scale,
            subtitle_y: top + (if compact { 57.0 } else { 72.0 }) * scale,
            scale,
            compact,
        }
    }
}

fn menu_scale(viewport: Vec2, size: Vec2) -> f32 {
    ((viewport.x - 32.0).max(1.0) / size.x)
        .min((viewport.y - 48.0).max(1.0) / size.y)
        .min(1.0)
}

fn draw_upgrade_icon(upgrade: Upgrade, center: Vec2, scale: f32) {
    let color = upgrade.color();
    draw_circle(center.x, center.y, 27.0 * scale, Color { a: 0.10, ..color });

    match upgrade {
        Upgrade::ExtraKnife | Upgrade::SharpenedMoon | Upgrade::WiderOrbit => {
            draw_circle(center.x, center.y, 16.0 * scale, color);
            draw_circle(
                center.x + 7.0 * scale,
                center.y - 5.0 * scale,
                16.0 * scale,
                PANEL,
            );
            draw_line(
                center.x - 11.0 * scale,
                center.y + 12.0 * scale,
                center.x + 13.0 * scale,
                center.y - 12.0 * scale,
                3.0 * scale,
                BONE,
            );
        },
        Upgrade::FastStorm | Upgrade::ForkedStorm | Upgrade::PotentStorm => {
            draw_triangle(
                center + vec2(2.0, -20.0) * scale,
                center + vec2(-11.0, 2.0) * scale,
                center + vec2(1.0, 1.0) * scale,
                color,
            );
            draw_triangle(
                center + vec2(-1.0, -1.0) * scale,
                center + vec2(11.0, -2.0) * scale,
                center + vec2(-5.0, 20.0) * scale,
                BONE,
            );
        },
        Upgrade::Fleet => {
            for offset in [-8.0, 0.0, 8.0] {
                draw_line(
                    center.x - 15.0 * scale,
                    center.y + offset * scale,
                    center.x + 13.0 * scale,
                    center.y + offset * scale,
                    3.0 * scale,
                    color,
                );
            }
            draw_triangle(
                center + vec2(18.0, 0.0) * scale,
                center + vec2(7.0, -9.0) * scale,
                center + vec2(7.0, 9.0) * scale,
                BONE,
            );
        },
        Upgrade::Vitality => {
            draw_circle(
                center.x - 7.0 * scale,
                center.y - 5.0 * scale,
                9.0 * scale,
                color,
            );
            draw_circle(
                center.x + 7.0 * scale,
                center.y - 5.0 * scale,
                9.0 * scale,
                color,
            );
            draw_triangle(
                center + vec2(-15.0, -3.0) * scale,
                center + vec2(15.0, -3.0) * scale,
                center + vec2(0.0, 18.0) * scale,
                color,
            );
        },
        Upgrade::Magnet => {
            draw_circle_lines(center.x, center.y, 17.0 * scale, 5.0 * scale, color);
            draw_circle(center.x, center.y, 7.0 * scale, PANEL);
            draw_circle(center.x, center.y, 3.0 * scale, BONE);
        },
    }
}

fn draw_scrim(viewport: Vec2) {
    draw_rectangle(
        0.0,
        0.0,
        viewport.x,
        viewport.y,
        Color::new(0.025, 0.02, 0.05, 0.82),
    );
}

fn draw_panel(rect: Rect) {
    draw_rectangle(
        rect.x + 4.0,
        rect.y + 5.0,
        rect.w,
        rect.h,
        Color::new(0.0, 0.0, 0.0, 0.24),
    );
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color { a: 0.22, ..BONE },
    );
}

fn draw_bar(x: f32, y: f32, width: f32, height: f32, ratio: f32, fill: Color, back: Color) {
    draw_rectangle(x, y, width, height, back);
    draw_rectangle(
        x + 2.0,
        y + 2.0,
        (width - 4.0) * ratio.clamp(0.0, 1.0),
        height - 4.0,
        fill,
    );
    draw_rectangle_lines(x, y, width, height, 1.5, Color::new(0.9, 0.84, 1.0, 0.35));
}

fn draw_weapon_badge(
    x: f32,
    y: f32,
    width: f32,
    name: &str,
    level: u32,
    color: Color,
    charge: f32,
) {
    draw_panel(Rect::new(x, y, width, 57.0));
    draw_rectangle(x, y, 4.0, 57.0, color);
    draw_text(name, x + 11.0, y + 20.0, 13.0, BONE);
    draw_text(format!("LV {level}"), x + 11.0, y + 37.0, 12.0, color);
    draw_bar(
        x + 9.0,
        y + 44.0,
        width - 18.0,
        7.0,
        charge,
        color,
        Color::new(0.0, 0.0, 0.0, 0.45),
    );
}

#[cfg(test)]
mod tests {
    use super::{UpgradeLayout, title_layout, upgrade_at};
    use macroquad::prelude::vec2;

    #[test]
    fn upgrade_menu_fits_portrait_and_short_landscape_viewports() {
        for viewport in [
            vec2(320.0, 568.0),
            vec2(360.0, 400.0),
            vec2(320.0, 240.0),
            vec2(720.0, 360.0),
            vec2(1280.0, 720.0),
            vec2(1920.0, 1080.0),
        ] {
            let layout = UpgradeLayout::new(viewport);
            let title_size = if layout.compact { 29.0 } else { 38.0 } * layout.scale;
            assert!(layout.title_y >= title_size);
            assert!(layout.title_y < layout.subtitle_y);
            assert!(layout.subtitle_y < layout.cards[0].y);

            for card in layout.cards {
                assert!(card.w > 0.0 && card.h > 0.0);
                assert!(card.x >= 0.0 && card.y >= 0.0);
                assert!(card.x + card.w <= viewport.x);
                assert!(card.y + card.h <= viewport.y);
            }
            assert!(
                layout
                    .cards
                    .windows(2)
                    .all(|pair| !pair[0].overlaps(&pair[1]))
            );
        }
    }

    #[test]
    fn upgrade_hit_testing_uses_card_bounds_and_ignores_gaps() {
        for viewport in [vec2(360.0, 400.0), vec2(1280.0, 720.0)] {
            let layout = UpgradeLayout::new(viewport);
            for (index, card) in layout.cards.iter().enumerate() {
                assert_eq!(
                    upgrade_at(vec2(card.x + 1.0, card.y + 1.0), viewport),
                    Some(index)
                );
                assert_eq!(
                    upgrade_at(vec2(card.x + card.w - 1.0, card.y + card.h - 1.0), viewport),
                    Some(index),
                );
            }
            let first = layout.cards[0];
            let second = layout.cards[1];
            let gap = if layout.compact {
                vec2(
                    first.x + first.w * 0.5,
                    (first.y + first.h + second.y) * 0.5,
                )
            } else {
                vec2(
                    (first.x + first.w + second.x) * 0.5,
                    first.y + first.h * 0.5,
                )
            };
            assert_eq!(upgrade_at(gap, viewport), None);
            assert_eq!(upgrade_at(vec2(-1.0, -1.0), viewport), None);
        }
    }

    #[test]
    fn title_artwork_leaves_room_for_the_controls_footer() {
        for viewport in [
            vec2(320.0, 200.0),
            vec2(320.0, 240.0),
            vec2(320.0, 568.0),
            vec2(390.0, 844.0),
            vec2(1280.0, 720.0),
        ] {
            let (center, scale) = title_layout(viewport);
            let (footer_y, text_size) = if viewport.x < 720.0 {
                (viewport.y - 62.0, 15.0)
            } else {
                (viewport.y - 42.0, 17.0)
            };
            assert!(center.y - 224.0 * scale >= 0.0);
            assert!(center.y + 145.0 * scale < footer_y - text_size);
        }
    }
}
