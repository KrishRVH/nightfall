//! Screen-space HUD, menus, and upgrade presentation.

use macroquad::prelude::*;

use super::{draw_text_center, hash_cell};
use crate::game::{
    ARCANE_VIOLET, BACKGROUND, BONE, DAMAGE_RED, GEM_GREEN, Game, INK, MOON_GOLD, STORM_CYAN,
    Upgrade, format_time,
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
    pub(super) fn draw_hud(&self) {
        self.draw_run_header();
        self.draw_loadout();
    }

    fn draw_run_header(&self) {
        let width = screen_width();
        let compact = width < 720.0;
        let hud_text_size = if compact { 16.0 } else { 20.0 };
        let experience_ratio = self.player.experience as f32 / self.player.next_level.max(1) as f32;
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
                Color::new(
                    GEM_GREEN.r,
                    GEM_GREEN.g,
                    GEM_GREEN.b,
                    self.pickup_flash / 0.24,
                ),
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

    fn draw_loadout(&self) {
        let width = screen_width();
        let height = screen_height();
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

        let storm_charge = (1.0 - self.storm_timer.max(0.0) / self.storm.cooldown).clamp(0.0, 1.0);
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

    pub(super) fn draw_title(&self) {
        let center = vec2(screen_width() * 0.5, screen_height() * 0.5);
        for index in 0..90 {
            let hash = hash_cell(index, index * 19);
            let position = vec2(
                (hash & 0xFFFF) as f32 / 65_535.0 * screen_width(),
                ((hash >> 16) & 0xFFFF) as f32 / 65_535.0 * screen_height(),
            );
            let twinkle = 0.25 + (self.visual_time * 2.0 + index as f32).sin().abs() * 0.45;
            draw_circle(
                position.x,
                position.y,
                1.4,
                Color::new(0.75, 0.68, 0.95, twinkle),
            );
        }

        draw_circle(
            center.x,
            center.y - 126.0,
            82.0,
            Color::new(0.96, 0.84, 0.55, 0.10),
        );
        draw_circle_lines(
            center.x,
            center.y - 126.0,
            95.0 + (self.visual_time * 1.4).sin() * 3.0,
            1.0,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, 0.18),
        );
        draw_circle(center.x, center.y - 126.0, 59.0, BONE);
        draw_circle(center.x + 24.0, center.y - 145.0, 59.0, BACKGROUND);
        draw_text_center("NIGHTFALL", center.x, center.y + 15.0, 72.0, BONE);
        draw_text_center(
            "A tiny survival spellbook",
            center.x,
            center.y + 50.0,
            23.0,
            Color::new(0.68, 0.61, 0.80, 1.0),
        );

        let pulse = 0.75 + (self.visual_time * 3.0).sin() * 0.15;
        let prompt_width = (screen_width() - 32.0).min(420.0);
        draw_rectangle(
            center.x - prompt_width * 0.5,
            center.y + 91.0,
            prompt_width,
            54.0,
            Color::new(0.09, 0.07, 0.15, 0.72),
        );
        draw_rectangle_lines(
            center.x - prompt_width * 0.5,
            center.y + 91.0,
            prompt_width,
            54.0,
            1.5,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, pulse * 0.65),
        );
        draw_text_center(
            "PRESS ENTER OR CLICK TO BEGIN",
            center.x,
            center.y + 127.0,
            25.0,
            Color::new(MOON_GOLD.r, MOON_GOLD.g, MOON_GOLD.b, pulse),
        );
        if screen_width() < 720.0 {
            draw_text_center(
                "WASD / ARROWS TO MOVE",
                center.x,
                screen_height() - 62.0,
                15.0,
                MUTED,
            );
            draw_text_center(
                "WEAPONS AUTO-FIRE  ·  ESC PAUSES",
                center.x,
                screen_height() - 36.0,
                15.0,
                MUTED,
            );
        } else {
            draw_text_center(
                "WASD / ARROWS TO MOVE  ·  WEAPONS FIRE AUTOMATICALLY  ·  ESC TO PAUSE",
                center.x,
                screen_height() - 42.0,
                17.0,
                MUTED,
            );
        }
    }

    pub(super) fn draw_pause() {
        draw_scrim();
        let panel_width = (screen_width() - 32.0).min(490.0);
        let panel = Rect::new(
            screen_width() * 0.5 - panel_width * 0.5,
            screen_height() * 0.5 - 90.0,
            panel_width,
            160.0,
        );
        draw_panel(panel);
        draw_text_center(
            "PAUSED",
            screen_width() * 0.5,
            screen_height() * 0.5 - 20.0,
            54.0,
            BONE,
        );
        draw_text_center(
            "Press Escape, P, Enter, or Space to continue",
            screen_width() * 0.5,
            screen_height() * 0.5 + 28.0,
            if screen_width() < 720.0 { 15.0 } else { 20.0 },
            MOON_GOLD,
        );
    }

    pub(super) fn draw_level_up(&self) {
        draw_scrim();
        let compact = screen_width() < 720.0;
        draw_text_center(
            "THE NIGHT ANSWERS",
            screen_width() * 0.5,
            if compact {
                94.0
            } else {
                screen_height() * 0.5 - 188.0
            },
            if compact { 29.0 } else { 38.0 },
            BONE,
        );
        draw_text_center(
            "Choose one power to continue",
            screen_width() * 0.5,
            if compact {
                122.0
            } else {
                screen_height() * 0.5 - 154.0
            },
            if compact { 14.0 } else { 17.0 },
            MUTED,
        );

        for (index, upgrade) in self.offers.iter().enumerate() {
            draw_upgrade_card(index, *upgrade, compact);
        }
    }

    pub(super) fn draw_game_over(&self) {
        draw_scrim();
        let center_x = screen_width() * 0.5;
        let center_y = screen_height() * 0.5;
        let compact = screen_width() < 720.0;
        let panel_width = (screen_width() - 32.0).min(580.0);
        draw_panel(Rect::new(
            center_x - panel_width * 0.5,
            center_y - 155.0,
            panel_width,
            270.0,
        ));
        draw_text_center(
            "DAWN FOUND YOU",
            center_x,
            center_y - 105.0,
            if compact { 34.0 } else { 47.0 },
            BONE,
        );
        draw_text_center(
            &format!("You endured {}", format_time(self.elapsed)),
            center_x,
            center_y - 48.0,
            23.0,
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
        draw_text_center(&result, center_x, center_y - 12.0, 20.0, MUTED);
        draw_text_center(
            "PRESS R, ENTER, OR CLICK TO TRY AGAIN",
            center_x,
            center_y + 72.0,
            if compact { 16.0 } else { 23.0 },
            STORM_CYAN,
        );
    }
}

pub(in crate::game) fn upgrade_at(point: Vec2) -> Option<usize> {
    (0..3).find(|index| upgrade_rect(*index).contains(point))
}

fn draw_upgrade_card(index: usize, upgrade: Upgrade, compact: bool) {
    let rect = upgrade_rect(index);
    let hovered = rect.contains(Vec2::from(mouse_position()));
    let lift = if hovered {
        if compact { -2.0 } else { -5.0 }
    } else {
        0.0
    };
    let card = Rect::new(rect.x, rect.y + lift, rect.w, rect.h);
    let color = upgrade.color();

    draw_rectangle(
        card.x + 7.0,
        card.y + 9.0,
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
    draw_rectangle(card.x, card.y, card.w, 5.0, color);
    draw_rectangle_lines(
        card.x,
        card.y,
        card.w,
        card.h,
        if hovered { 3.0 } else { 1.5 },
        Color::new(color.r, color.g, color.b, if hovered { 1.0 } else { 0.65 }),
    );
    draw_rectangle(card.x + card.w - 40.0, card.y + 15.0, 25.0, 25.0, color);
    draw_text_center(
        &(index + 1).to_string(),
        card.x + card.w - 27.5,
        card.y + 35.0,
        17.0,
        INK,
    );

    if compact {
        draw_compact_upgrade(upgrade, card);
    } else {
        draw_wide_upgrade(upgrade, card);
    }
}

fn draw_compact_upgrade(upgrade: Upgrade, card: Rect) {
    draw_upgrade_icon(upgrade, vec2(card.x + 48.0, card.y + card.h * 0.5));
    draw_text(
        upgrade.school(),
        card.x + 86.0,
        card.y + 32.0,
        12.0,
        upgrade.color(),
    );
    draw_text(upgrade.name(), card.x + 86.0, card.y + 61.0, 19.0, BONE);
    draw_text(
        upgrade.description(),
        card.x + 86.0,
        card.y + 88.0,
        14.0,
        MUTED,
    );
}

fn draw_wide_upgrade(upgrade: Upgrade, card: Rect) {
    let center_x = card.x + card.w * 0.5;
    draw_upgrade_icon(upgrade, vec2(center_x, card.y + 57.0));
    draw_text_center(
        upgrade.school(),
        center_x,
        card.y + 96.0,
        13.0,
        upgrade.color(),
    );
    draw_text_center(upgrade.name(), center_x, card.y + 127.0, 21.0, BONE);
    draw_text_center(upgrade.description(), center_x, card.y + 163.0, 15.0, MUTED);
}

fn upgrade_rect(index: usize) -> Rect {
    if screen_width() < 720.0 {
        let gap = 10.0;
        let width = (screen_width() - 28.0).min(520.0);
        let height = 108.0;
        let total_height = height * 3.0 + gap * 2.0;
        return Rect::new(
            (screen_width() - width) * 0.5,
            (screen_height() - total_height) * 0.5 + index as f32 * (height + gap) + 18.0,
            width,
            height,
        );
    }

    let gap = 18.0;
    let total_width = screen_width().min(930.0);
    let width = (total_width - gap * 4.0) / 3.0;
    let start_x = (screen_width() - total_width) * 0.5 + gap;
    Rect::new(
        start_x + index as f32 * (width + gap),
        screen_height() * 0.5 - 105.0,
        width,
        190.0,
    )
}

fn draw_upgrade_icon(upgrade: Upgrade, center: Vec2) {
    let color = upgrade.color();
    draw_circle(
        center.x,
        center.y,
        27.0,
        Color::new(color.r, color.g, color.b, 0.10),
    );

    match upgrade {
        Upgrade::ExtraKnife | Upgrade::SharpenedMoon | Upgrade::WiderOrbit => {
            draw_circle(center.x, center.y, 16.0, color);
            draw_circle(center.x + 7.0, center.y - 5.0, 16.0, PANEL);
            draw_line(
                center.x - 11.0,
                center.y + 12.0,
                center.x + 13.0,
                center.y - 12.0,
                3.0,
                BONE,
            );
        },
        Upgrade::FastStorm | Upgrade::ForkedStorm | Upgrade::PotentStorm => {
            draw_triangle(
                center + vec2(2.0, -20.0),
                center + vec2(-11.0, 2.0),
                center + vec2(1.0, 1.0),
                color,
            );
            draw_triangle(
                center + vec2(-1.0, -1.0),
                center + vec2(11.0, -2.0),
                center + vec2(-5.0, 20.0),
                BONE,
            );
        },
        Upgrade::Fleet => {
            for offset in [-8.0, 0.0, 8.0] {
                draw_line(
                    center.x - 15.0,
                    center.y + offset,
                    center.x + 13.0,
                    center.y + offset,
                    3.0,
                    color,
                );
            }
            draw_triangle(
                center + vec2(18.0, 0.0),
                center + vec2(7.0, -9.0),
                center + vec2(7.0, 9.0),
                BONE,
            );
        },
        Upgrade::Vitality => {
            draw_circle(center.x - 7.0, center.y - 5.0, 9.0, color);
            draw_circle(center.x + 7.0, center.y - 5.0, 9.0, color);
            draw_triangle(
                center + vec2(-15.0, -3.0),
                center + vec2(15.0, -3.0),
                center + vec2(0.0, 18.0),
                color,
            );
        },
        Upgrade::Magnet => {
            draw_circle_lines(center.x, center.y, 17.0, 5.0, color);
            draw_circle(center.x, center.y, 7.0, PANEL);
            draw_circle(center.x, center.y, 3.0, BONE);
        },
    }
}

fn draw_scrim() {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
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
        Color::new(BONE.r, BONE.g, BONE.b, 0.22),
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

fn draw_text_right(text: &str, right: f32, baseline: f32, size: f32, color: Color) {
    let dimensions = measure_text(text, None, size as u16, 1.0);
    draw_text(text, right - dimensions.width, baseline, size, color);
}
