//! Screen-space HUD, menus and upgrade presentation.
//!
//! Drawn after the composite so type and panel edges stay at native resolution,
//! never passing through bloom, grain or the tone curve. Everything here works
//! in window pixels rather than world units.

use macroquad::prelude::*;

use super::{
    ARCANE_VIOLET, BONE, DAMAGE_NUMBER_LIFE, DAMAGE_RED, GEM_GREEN, Game, MOON_GOLD, Phase,
    STORM_CYAN, Upgrade, format_time,
};

/// Panel fill: a cool near-black glass that lets the graded world show through.
const PANEL: Color = Color::new(0.055, 0.045, 0.105, 0.90);
const MUTED: Color = Color::new(0.62, 0.58, 0.74, 1.0);
const ACCENT: Color = Color::new(0.80, 0.74, 0.96, 1.0);

/// Width in pixels below which the interface switches to its stacked layout.
const COMPACT_WIDTH: f32 = 720.0;

/// Animation state that has to survive between frames.
///
/// The health bar's trailing "ghost" is the one piece of interface state that
/// cannot be derived from the simulation, since it encodes how quickly damage
/// arrived rather than how much of it is left.
#[derive(Default)]
pub(super) struct UiState {
    health_ghost: f32,
    reveal: f32,
}

/// Eases the trailing health bar toward the real value and times card reveals.
impl UiState {
    pub(super) fn advance(&mut self, game: &Game) {
        let ratio = game.health_ratio();
        // The ghost falls slowly, so a big hit leaves a readable red scar.
        let rate = if ratio < self.health_ghost { 0.35 } else { 3.2 };
        self.health_ghost += (ratio - self.health_ghost).min(rate * get_frame_time());
        if game.phase == Phase::LevelUp {
            self.reveal = (self.reveal + get_frame_time() * 2.4).min(1.0);
        } else {
            self.reveal = 0.0;
        }
    }
}

impl Upgrade {
    fn name(self) -> &'static str {
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

    fn description(self) -> &'static str {
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

    fn color(self) -> Color {
        match self {
            Self::ExtraKnife | Self::SharpenedMoon | Self::WiderOrbit => MOON_GOLD,
            Self::FastStorm | Self::ForkedStorm | Self::PotentStorm => STORM_CYAN,
            Self::Fleet | Self::Vitality | Self::Magnet => GEM_GREEN,
        }
    }

    fn school(self) -> &'static str {
        match self {
            Self::ExtraKnife | Self::SharpenedMoon | Self::WiderOrbit => "MOON KNIVES",
            Self::FastStorm | Self::ForkedStorm | Self::PotentStorm => "STORM LANTERN",
            Self::Fleet | Self::Vitality | Self::Magnet => "WITCHCRAFT",
        }
    }
}

/// Draws the in-run interface: HUD plus whichever overlay the phase calls for.
pub(super) fn draw_frame(game: &Game, ui: &UiState, view: &Camera2D) {
    draw_run_header(game);
    draw_loadout(game, ui);
    draw_damage_numbers(game, view);

    match game.phase {
        Phase::Paused => draw_pause(),
        Phase::LevelUp => draw_level_up(game, ui),
        Phase::GameOver => draw_game_over(game),
        Phase::Title | Phase::Running => {},
    }
}

/// Top bar: experience, elapsed time and kill count.
fn draw_run_header(game: &Game) {
    let width = screen_width();
    let compact = is_compact();
    let size = if compact { 16.0 } else { 20.0 };

    draw_panel(Rect::new(14.0, 14.0, width - 28.0, 47.0));

    let ratio = (game.player.experience as f32
        / f32::from(u16::try_from(game.player.next_level).unwrap_or(1)))
    .clamp(0.0, 1.0);
    draw_bar(
        Vec2::new(18.0, 18.0),
        width - 36.0,
        8.0,
        ratio,
        ARCANE_VIOLET,
    );
    if game.pickup_flash > 0.0 {
        let flash = game.pickup_flash / 0.24;
        draw_rectangle_lines(
            18.0,
            18.0,
            width - 36.0,
            8.0,
            2.0,
            GEM_GREEN.with_alpha(flash),
        );
    }

    draw_text(
        format!("LEVEL {:02}", game.player.level),
        22.0,
        50.0,
        size,
        BONE,
    );
    draw_text_center(
        &format_time(game.elapsed),
        width * 0.5,
        51.0,
        if compact { 20.0 } else { 25.0 },
        BONE,
    );
    let kills = if compact {
        format!("{:03} KOs", game.kills)
    } else {
        format!("{:03} BANISHED", game.kills)
    };
    draw_text_right(&kills, width - 22.0, 50.0, size, BONE);
}

/// Floating damage numbers.
///
/// These are anchored in world space but drawn at native resolution, so they stay
/// legible over a bright spell flash instead of being washed out by the composite.
fn draw_damage_numbers(game: &Game, view: &Camera2D) {
    for number in &game.damage_numbers {
        let fade = (number.life / DAMAGE_NUMBER_LIFE).clamp(0.0, 1.0);
        let alpha = (fade * 1.6).min(1.0);
        // Lift each number as it fades so the cluster separates over time.
        let rise = (1.0 - fade) * 26.0;
        let at = view.world_to_screen(number.position + vec2(0.0, -rise));
        // Skip anything the camera has pushed off screen.
        if at.x < -60.0
            || at.y < -40.0
            || at.x > screen_width() + 60.0
            || at.y > screen_height() + 40.0
        {
            continue;
        }
        let size = 15.0 + fade * 5.0;
        let text = number.value.to_string();
        draw_text_center(
            &text,
            at.x,
            at.y + size * 0.4,
            size,
            Color::new(0.0, 0.0, 0.0, alpha * 0.7),
        );
        draw_text_center(
            &text,
            at.x - 1.5,
            at.y + size * 0.4 - 1.5,
            size,
            number.color.with_alpha(alpha),
        );
    }
}

/// Bottom row: vitality and the two weapon badges.
fn draw_loadout(game: &Game, ui: &UiState) {
    let width = screen_width();
    let height = screen_height();
    let compact = is_compact();

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
    draw_health_bar(
        Vec2::new(health.x + 8.0, health.y + 30.0),
        health.w - 16.0,
        17.0,
        game.health_ratio(),
        ui.health_ghost,
    );
    draw_text(
        format!("{:.0} / {:.0}", game.player.health, game.player.max_health),
        health.x + 15.0,
        health.y + 44.0,
        14.0,
        BONE,
    );

    draw_weapon_badge(
        moon_x,
        weapon_y,
        weapon_width,
        "MOON KNIVES",
        game.moon.level,
        MOON_GOLD,
        1.0,
    );
    draw_weapon_badge(
        storm_x,
        weapon_y,
        weapon_width,
        "STORM LANTERN",
        game.storm.level,
        STORM_CYAN,
        game.storm_charge(),
    );
}

/// Health bar with a trailing scar showing recently lost health.
fn draw_health_bar(at: Vec2, width: f32, height: f32, ratio: f32, ghost: f32) {
    draw_rectangle(
        at.x,
        at.y,
        width,
        height,
        Color::new(0.02, 0.015, 0.035, 0.75),
    );
    let scarred = ghost.max(ratio).clamp(0.0, 1.0);
    draw_rectangle(
        at.x + 2.0,
        at.y + 2.0,
        (width - 4.0) * scarred,
        height - 4.0,
        Color::new(0.55, 0.16, 0.20, 0.85),
    );
    draw_rectangle(
        at.x + 2.0,
        at.y + 2.0,
        (width - 4.0) * ratio,
        height - 4.0,
        DAMAGE_RED,
    );
    // Leading highlight reads as glass rather than a flat block of colour.
    draw_rectangle(
        at.x + 2.0,
        at.y + 2.0,
        (width - 4.0) * ratio,
        (height - 4.0) * 0.45,
        Color::new(1.0, 1.0, 1.0, 0.16),
    );
    draw_rectangle_lines(at.x, at.y, width, height, 1.5, ACCENT.with_alpha(0.35));
}

/// The title screen: a starfield, a waxing moon and the entry prompt.
pub(super) fn draw_title(game: &Game) {
    let center = vec2(screen_width() * 0.5, screen_height() * 0.5);
    draw_starfield(game);

    // Moon: a lit disc with a bite taken out by the background colour.
    draw_circle(center.x, center.y - 126.0, 82.0, MOON_GOLD.with_alpha(0.07));
    draw_circle_lines(
        center.x,
        center.y - 126.0,
        95.0 + (game.visual_time * 1.4).sin() * 3.0,
        1.0,
        MOON_GOLD.with_alpha(0.22),
    );
    draw_circle(center.x, center.y - 126.0, 59.0, BONE);
    draw_circle(center.x + 24.0, center.y - 145.0, 59.0, super::BACKGROUND);
    draw_circle(
        center.x - 14.0,
        center.y - 108.0,
        52.0,
        BONE.with_alpha(0.10),
    );

    draw_text_center("NIGHTFALL", center.x, center.y + 15.0, 72.0, BONE);
    draw_text_center(
        "A tiny survival spellbook",
        center.x,
        center.y + 50.0,
        23.0,
        MUTED,
    );

    let pulse = 0.75 + (game.visual_time * 3.0).sin() * 0.15;
    let prompt_width = (screen_width() - 32.0).min(420.0);
    draw_panel(Rect::new(
        center.x - prompt_width * 0.5,
        center.y + 91.0,
        prompt_width,
        54.0,
    ));
    draw_text_center(
        "PRESS ENTER OR CLICK TO BEGIN",
        center.x,
        center.y + 127.0,
        25.0,
        MOON_GOLD.with_alpha(pulse),
    );

    if is_compact() {
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

/// Deterministic starfield, hashed so it never consumes gameplay randomness.
fn draw_starfield(game: &Game) {
    for index in 0..90 {
        let hash = super::star_hash(index, index * 19);
        let position = vec2(
            (hash & 0xFFFF) as f32 / 65_535.0 * screen_width(),
            ((hash >> 16) & 0xFFFF) as f32 / 65_535.0 * screen_height(),
        );
        let twinkle = 0.25 + (game.visual_time * 2.0 + index as f32).sin().abs() * 0.45;
        draw_circle(
            position.x,
            position.y,
            1.4,
            Color::new(0.75, 0.68, 0.95, twinkle),
        );
    }
}

fn draw_pause() {
    draw_scrim();
    let panel_width = (screen_width() - 32.0).min(490.0);
    draw_panel(Rect::new(
        screen_width() * 0.5 - panel_width * 0.5,
        screen_height() * 0.5 - 90.0,
        panel_width,
        160.0,
    ));
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
        if is_compact() { 15.0 } else { 20.0 },
        MOON_GOLD,
    );
}

fn draw_level_up(game: &Game, ui: &UiState) {
    draw_scrim();
    let compact = is_compact();
    // Cards rise and fade in as the reveal completes.
    let eased = 1.0 - (1.0 - ui.reveal) * (1.0 - ui.reveal);

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

    for (index, upgrade) in game.offers.iter().enumerate() {
        draw_upgrade_card(index, *upgrade, compact, eased);
    }
}

fn draw_game_over(game: &Game) {
    draw_scrim();
    let center_x = screen_width() * 0.5;
    let center_y = screen_height() * 0.5;
    let compact = is_compact();
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
        &format!("You endured {}", format_time(game.elapsed)),
        center_x,
        center_y - 48.0,
        23.0,
        MOON_GOLD,
    );
    let result = if compact {
        format!("{} banished  ·  level {}", game.kills, game.player.level)
    } else {
        format!(
            "{} enemies banished  ·  level {} reached",
            game.kills, game.player.level
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

/// Which upgrade card, if any, sits under a screen point.
pub(crate) fn upgrade_at(point: Vec2) -> Option<usize> {
    (0..3).find(|index| upgrade_rect(*index).contains(point))
}

/// One upgrade card, lifted and outlined while the pointer is over it.
fn draw_upgrade_card(index: usize, upgrade: Upgrade, compact: bool, reveal: f32) {
    let rect = upgrade_rect(index);
    let hovered = rect.contains(Vec2::from(mouse_position()));
    let lift = if hovered {
        if compact { -2.0 } else { -5.0 }
    } else {
        0.0
    };
    // Cards settle upward into place as the reveal completes.
    let slide = (1.0 - reveal) * (index as f32 + 1.0) * 34.0;
    let card = Rect::new(rect.x, rect.y + lift + slide, rect.w, rect.h);
    let color = upgrade.color();
    let alpha = reveal.clamp(0.0, 1.0);

    draw_rectangle(
        card.x + 7.0,
        card.y + 9.0,
        card.w,
        card.h,
        Color::new(0.0, 0.0, 0.0, 0.34 * alpha),
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
        }
        .with_alpha(0.98 * alpha),
    );
    draw_rectangle(card.x, card.y, card.w, 5.0, color.with_alpha(alpha));
    draw_rectangle_lines(
        card.x,
        card.y,
        card.w,
        card.h,
        if hovered { 3.0 } else { 1.5 },
        color.with_alpha(if hovered { 1.0 } else { 0.6 } * alpha),
    );
    // Number badge: the keyboard shortcut, echoed visually.
    draw_rectangle(
        card.x + card.w - 40.0,
        card.y + 15.0,
        25.0,
        25.0,
        color.with_alpha(alpha),
    );
    draw_text_center(
        &(index + 1).to_string(),
        card.x + card.w - 27.5,
        card.y + 35.0,
        17.0,
        super::INK.with_alpha(alpha),
    );

    if compact {
        draw_compact_upgrade(upgrade, card, alpha);
    } else {
        draw_wide_upgrade(upgrade, card, alpha);
    }
}

fn draw_compact_upgrade(upgrade: Upgrade, card: Rect, alpha: f32) {
    draw_upgrade_icon(upgrade, vec2(card.x + 48.0, card.y + card.h * 0.5), alpha);
    draw_text(
        upgrade.school(),
        card.x + 86.0,
        card.y + 32.0,
        12.0,
        upgrade.color().with_alpha(alpha),
    );
    draw_text(
        upgrade.name(),
        card.x + 86.0,
        card.y + 61.0,
        19.0,
        BONE.with_alpha(alpha),
    );
    draw_text(
        upgrade.description(),
        card.x + 86.0,
        card.y + 88.0,
        14.0,
        MUTED.with_alpha(alpha),
    );
}

fn draw_wide_upgrade(upgrade: Upgrade, card: Rect, alpha: f32) {
    let center_x = card.x + card.w * 0.5;
    draw_upgrade_icon(upgrade, vec2(center_x, card.y + 57.0), alpha);
    draw_text_center(
        upgrade.school(),
        center_x,
        card.y + 96.0,
        13.0,
        upgrade.color().with_alpha(alpha),
    );
    draw_text_center(
        upgrade.name(),
        center_x,
        card.y + 127.0,
        21.0,
        BONE.with_alpha(alpha),
    );
    draw_text_center(
        upgrade.description(),
        center_x,
        card.y + 163.0,
        15.0,
        MUTED.with_alpha(alpha),
    );
}

/// Layout for one upgrade card: stacked when narrow, side by side otherwise.
fn upgrade_rect(index: usize) -> Rect {
    if is_compact() {
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

/// Icon for an upgrade, drawn from the same primitives as the creatures.
fn draw_upgrade_icon(upgrade: Upgrade, center: Vec2, alpha: f32) {
    let color = upgrade.color();
    draw_circle(center.x, center.y, 27.0, color.with_alpha(0.10 * alpha));
    let color = color.with_alpha(alpha);

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
                BONE.with_alpha(alpha),
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
                BONE.with_alpha(alpha),
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
                BONE.with_alpha(alpha),
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
            draw_circle(center.x, center.y, 3.0, BONE.with_alpha(alpha));
        },
    }
}

/// True when the window is too narrow for the wide interface layout.
fn is_compact() -> bool {
    screen_width() < COMPACT_WIDTH
}

/// Darkens the frame behind a modal screen.
fn draw_scrim() {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.020, 0.016, 0.042, 0.80),
    );
}

/// A lifted glass panel: drop shadow, fill, hairline border, accent underline.
fn draw_panel(rect: Rect) {
    draw_rectangle(
        rect.x + 4.0,
        rect.y + 6.0,
        rect.w,
        rect.h,
        Color::new(0.0, 0.0, 0.0, 0.38),
    );
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::new(BONE.r, BONE.g, BONE.b, 0.20),
    );
    draw_rectangle(
        rect.x + 1.0,
        rect.y + 1.0,
        rect.w - 2.0,
        1.0,
        ACCENT.with_alpha(0.10),
    );
}

/// Progress bar with track, fill and a bright leading cap.
fn draw_bar(at: Vec2, width: f32, height: f32, ratio: f32, fill: Color) {
    draw_rectangle(
        at.x,
        at.y,
        width,
        height,
        Color::new(0.02, 0.015, 0.035, 0.80),
    );
    let filled = (width - 4.0) * ratio.clamp(0.0, 1.0);
    draw_rectangle(at.x + 2.0, at.y + 2.0, filled, height - 4.0, fill);
    draw_rectangle(
        at.x + 2.0,
        at.y + 2.0,
        filled,
        (height - 4.0) * 0.45,
        WHITE_GLINT,
    );
    if filled > 2.0 {
        draw_rectangle(
            at.x + filled,
            at.y + 1.0,
            2.0,
            height - 2.0,
            Color::new(1.0, 1.0, 1.0, 0.55),
        );
    }
}

const WHITE_GLINT: Color = Color::new(1.0, 1.0, 1.0, 0.18);

/// Weapon badge: level, name and a charge meter.
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
        Vec2::new(x + 9.0, y + 44.0),
        width - 18.0,
        7.0,
        charge,
        color,
    );
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

fn draw_text_right(text: &str, right: f32, baseline: f32, size: f32, color: Color) {
    let dimensions = measure_text(text, None, size as u16, 1.0);
    draw_text(text, right - dimensions.width, baseline, size, color);
}
