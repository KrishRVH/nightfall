//! Screen-space HUD, menus, and upgrade presentation.

use std::f32::consts::PI;

use macroquad::prelude::*;

use super::{draw_text_center, hash_cell};
use crate::game::{
    ABYSSAL_MINE, AETHER_MAUVE, ARCANE_VIOLET, BACKGROUND, BONE, CINDER_ORANGE, CRESCENT_AUREATE,
    CombatPulse, DAMAGE_RED, GEM_GREEN, GLOOM_PURPLE, GRAVE_MARROW, Game, INK, LANCE_AZURE,
    MOON_GOLD, PHANTOM_CERULEAN, PHANTOM_EMBER, PHANTOM_NET_GREEN, RIFT_CYAN, RIFT_VIOLET,
    SCYTHE_IRON, SOLAR_NOVA, STORM_CYAN, Upgrade, VOID_BLOOM, WRAITH_AMETHYST, format_time,
};

const PANEL: Color = Color::new(0.075, 0.060, 0.125, 0.94);
const MUTED: Color = Color::new(0.60, 0.55, 0.70, 1.0);
const COMBAT_WEAPON_BAR_COUNT: usize = crate::game::COMBAT_WEAPON_BAR_COUNT;

impl Upgrade {
    const fn name(self) -> &'static str {
        match self {
            Self::ExtraKnife => "Another Moon",
            Self::SharpenedMoon => "Honed Crescent",
            Self::WiderOrbit => "Long Eclipse",
            Self::FastStorm => "Quickened Spark",
            Self::ForkedStorm => "Forked Omen",
            Self::PotentStorm => "Thunderheart",
            Self::GraveMine => "Gravefield",
            Self::GraveMineReach => "Cinder Reach",
            Self::GraveMineRage => "Mourning Burst",
            Self::AstralFlare => "Astral Halo",
            Self::AstralFlareBloom => "Comet Bloom",
            Self::AstralFlarePulse => "Solar Pulse",
            Self::EchoCannon => "Echo Cannon",
            Self::EchoCannonResonance => "Resonant Cannon",
            Self::EchoCannonCataclysm => "Cataclysm Cannon",
            Self::WraithLash => "Wraith Lash",
            Self::WraithLashReach => "Crescent Lash",
            Self::WraithLashRend => "Rending Lash",
            Self::HarrowVolley => "Harrow Volley",
            Self::HarrowVolleyAim => "Seeking Volleys",
            Self::HarrowVolleyPierce => "Piercing Volleys",
            Self::AetherSpear | Self::AetherSpearSplit | Self::AetherSpearRage => "Aether Spear",
            Self::RiftPulse | Self::RiftPulseAnchor | Self::RiftPulseCascade => "Rift Pulse",
            Self::SolarNova | Self::SolarNovaBloom | Self::SolarNovaCataclysm => "Solar Nova",
            Self::CrescentHalo | Self::CrescentHaloSpiral | Self::CrescentHaloCataclysm => {
                "Crescent Halo"
            },
            Self::VoidBloom | Self::VoidBloomReach | Self::VoidBloomCascade => "Void Bloom",
            Self::AbyssalMine | Self::AbyssalMineReach | Self::AbyssalMineRage => "Abyssal Mines",
            Self::Starfall | Self::StarfallCascade | Self::StarfallCataclysm => "Starfall",
            Self::PhantomNet | Self::PhantomNetReach | Self::PhantomNetRage => "Phantom Net",
            Self::LuminousLance | Self::LuminousLanceFork | Self::LuminousLanceRend => {
                "Luminous Lance"
            },
            Self::TemporalRift | Self::TemporalRiftAnchor | Self::TemporalRiftSurge => {
                "Temporal Rift"
            },
            Self::ScytheCyclone | Self::ScytheCycloneSpiral | Self::ScytheCycloneRavage => {
                "Scythe Cyclone"
            },
            Self::GloomVolley | Self::GloomVolleyCage | Self::GloomVolleyEcho => "Gloom Volley",
            Self::PulseLance | Self::PulseLanceSurge | Self::PulseLanceCataclysm => "Pulse Lance",
            Self::ShardStorm | Self::ShardStormCascade | Self::ShardStormCataclysm => "Shard Storm",
            Self::PrismBolts | Self::PrismBoltsTwin | Self::PrismBoltsRift => "Prism Bolts",
            Self::InfernoBomb | Self::InfernoBombScatter | Self::InfernoBombCataclysm => {
                "Inferno Bomb"
            },
            Self::GravityWeave | Self::GravityWeaveAnchor | Self::GravityWeaveCollapse => {
                "Gravity Weave"
            },
            Self::SigilNet | Self::SigilNetReach | Self::SigilNetRuin => "Sigil Net",
            Self::Fleet => "Fleetfoot",
            Self::Vitality => "Blood Ward",
            Self::Magnet => "Grave Pull",
            Self::RuneWard | Self::RuneWardEcho | Self::RuneWardCataclysm => "Rune Ward",
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
            Self::GraveMine => "Unlock homing graveshard mines",
            Self::GraveMineReach => "+8 blast radius and +9 homing",
            Self::GraveMineRage => "Mine damage up, cooldown down",
            Self::AstralFlare => "Unlock radial pulse around player",
            Self::AstralFlareBloom => "+12 flare radius and damage",
            Self::AstralFlarePulse => "Quicker pulse cooldown",
            Self::SolarNova => "Unlock expanding solar nova rings",
            Self::SolarNovaBloom => "+1 ring, +14 radius, +11% damage",
            Self::SolarNovaCataclysm | Self::InfernoBombCataclysm => "+22% damage, faster cadence",
            Self::EchoCannon => "Unlock spectral cannon pulses",
            Self::EchoCannonResonance => "+1 shard, +16% damage, +18 speed",
            Self::EchoCannonCataclysm => "+1 bounce, +22% damage, -18% cooldown",
            Self::WraithLash => "Unlock chain whip strike",
            Self::WraithLashReach => "+32 range and +5% damage",
            Self::WraithLashRend => "+2.6 lash width, +32% damage",
            Self::HarrowVolley => "Unlock piercing dart volleys",
            Self::HarrowVolleyAim => "+1 dart and dart speed",
            Self::HarrowVolleyPierce => "+24% dart damage, faster cool-down",
            Self::AetherSpear => "Unlock homing astral spears",
            Self::AetherSpearSplit => "+1 spear, +7% damage, wider spread",
            Self::AetherSpearRage => "+1 pierce, +16% spear damage, faster",
            Self::RiftPulse => "Unlock growing pulses from the rift",
            Self::RiftPulseAnchor => "+20 radius and +12% duration",
            Self::RiftPulseCascade => "+20% damage, faster pulse cadence",
            Self::CrescentHalo => "Unlock crescent orbit blades",
            Self::CrescentHaloSpiral => "+2 blades and wider arc",
            Self::CrescentHaloCataclysm => "+18% cadence and +6% reach",
            Self::VoidBloom => "Unlock homing void seeds",
            Self::VoidBloomReach => "+1 seed and +15% homing",
            Self::VoidBloomCascade => "+1 seed, +10% duration, faster cooldown",
            Self::AbyssalMine => "Unlock tracking abyssal minelets",
            Self::AbyssalMineReach => "+1 mine, +10 blast radius, +14 homing",
            Self::AbyssalMineRage => "+24% damage, faster mine cadence",
            Self::Starfall => "Unlock meteor storms from above",
            Self::StarfallCascade => "+1 meteor, +10% damage, bigger blast",
            Self::StarfallCataclysm => "+18% damage, faster cadence",
            Self::PhantomNet => "Unlock orbiting phantom tethers",
            Self::PhantomNetReach => "+1 thread, wider orbit",
            Self::PhantomNetRage => "+20% damage, better seeking",
            Self::LuminousLance => "Unlock homing spears",
            Self::LuminousLanceFork => "+1 spear, +7.5% damage, +26 range",
            Self::LuminousLanceRend => "+1 pierce, +16% damage, faster",
            Self::TemporalRift => "Unlock pulsing temporal wells",
            Self::TemporalRiftAnchor => "+1 gate, wider and longer wells",
            Self::TemporalRiftSurge => "+20% damage, faster pulses",
            Self::ScytheCyclone => "Unlock orbiting scythe blades",
            Self::ScytheCycloneSpiral => "+1 blade, wider sweep",
            Self::ScytheCycloneRavage => "+6% cadence, +26% damage",
            Self::GloomVolley => "Unlock void-touched shards",
            Self::GloomVolleyCage => "+1 shard, +24 range, +1 pierce",
            Self::GloomVolleyEcho => "+17% damage, more pierce, faster shards",
            Self::PulseLance => "Unlock piercing lance strikes",
            Self::PulseLanceSurge => "+1 chain and wider strike arc",
            Self::ShardStorm => "Unlock shard storm projectiles",
            Self::ShardStormCascade => "+1 shard, +40 range, +1 pierce",
            Self::PulseLanceCataclysm | Self::ShardStormCataclysm => "+24% damage, faster cadence",
            Self::PrismBolts => "Unlock prism bolts",
            Self::PrismBoltsTwin => "+1 bolt, +10% homing",
            Self::PrismBoltsRift => "+20% range and +12% speed",
            Self::InfernoBomb => "Unlock inferno bombs",
            Self::InfernoBombScatter => "+1 bomb and broader blast",
            Self::GravityWeave => "Unlock gravity wells",
            Self::GravityWeaveAnchor => "+1 well and longer cycle",
            Self::GravityWeaveCollapse => "+22% pull, +16% radius",
            Self::SigilNet => "Unlock orbiting sigil threads",
            Self::SigilNetReach => "+1 thread and wider orbit",
            Self::SigilNetRuin => "+18% damage, stronger pull",
            Self::Fleet => "+18% movement speed",
            Self::Vitality => "+35 max health and healing",
            Self::Magnet => "+45% pickup reach",
            Self::RuneWard => "Conjure pulsing ward circles",
            Self::RuneWardEcho => "+10 radius, +11% damage, +8 pull",
            Self::RuneWardCataclysm => "+22% cadence, +0.12s pulse duration",
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::ExtraKnife | Self::SharpenedMoon | Self::WiderOrbit => MOON_GOLD,
            Self::FastStorm | Self::ForkedStorm | Self::PotentStorm => STORM_CYAN,
            Self::GraveMine
            | Self::GraveMineReach
            | Self::GraveMineRage
            | Self::AstralFlare
            | Self::AstralFlareBloom
            | Self::AstralFlarePulse
            | Self::Starfall
            | Self::StarfallCascade
            | Self::StarfallCataclysm => CINDER_ORANGE,
            Self::SolarNova | Self::SolarNovaBloom | Self::SolarNovaCataclysm => SOLAR_NOVA,
            Self::WraithLash | Self::WraithLashReach | Self::WraithLashRend => WRAITH_AMETHYST,
            Self::HarrowVolley | Self::HarrowVolleyAim | Self::HarrowVolleyPierce => ARCANE_VIOLET,
            Self::AetherSpear
            | Self::AetherSpearSplit
            | Self::AetherSpearRage
            | Self::LuminousLance
            | Self::LuminousLanceFork
            | Self::LuminousLanceRend => AETHER_MAUVE,
            Self::RiftPulse | Self::RiftPulseAnchor | Self::RiftPulseCascade => RIFT_VIOLET,
            Self::CrescentHalo | Self::CrescentHaloSpiral | Self::CrescentHaloCataclysm => {
                CRESCENT_AUREATE
            },
            Self::VoidBloom | Self::VoidBloomReach | Self::VoidBloomCascade => VOID_BLOOM,
            Self::AbyssalMine | Self::AbyssalMineReach | Self::AbyssalMineRage => ABYSSAL_MINE,
            Self::PhantomNet
            | Self::PhantomNetReach
            | Self::PhantomNetRage
            | Self::SigilNet
            | Self::SigilNetReach
            | Self::SigilNetRuin => PHANTOM_NET_GREEN,
            Self::TemporalRift
            | Self::TemporalRiftAnchor
            | Self::TemporalRiftSurge
            | Self::GravityWeave
            | Self::GravityWeaveAnchor
            | Self::GravityWeaveCollapse => RIFT_CYAN,
            Self::ScytheCyclone | Self::ScytheCycloneSpiral | Self::ScytheCycloneRavage => {
                SCYTHE_IRON
            },
            Self::GloomVolley
            | Self::GloomVolleyCage
            | Self::GloomVolleyEcho
            | Self::ShardStorm
            | Self::ShardStormCascade
            | Self::ShardStormCataclysm => GLOOM_PURPLE,
            Self::PulseLance | Self::PulseLanceSurge | Self::PulseLanceCataclysm => LANCE_AZURE,
            Self::PrismBolts | Self::PrismBoltsTwin | Self::PrismBoltsRift => PHANTOM_CERULEAN,
            Self::InfernoBomb | Self::InfernoBombScatter | Self::InfernoBombCataclysm => {
                CINDER_ORANGE
            },
            Self::EchoCannon | Self::EchoCannonResonance | Self::EchoCannonCataclysm => {
                PHANTOM_CERULEAN
            },
            Self::RuneWard | Self::RuneWardEcho | Self::RuneWardCataclysm => PHANTOM_EMBER,
            Self::Fleet | Self::Vitality | Self::Magnet => GEM_GREEN,
        }
    }

    const fn school(self) -> &'static str {
        match self {
            Self::ExtraKnife | Self::SharpenedMoon | Self::WiderOrbit => "MOON KNIVES",
            Self::FastStorm | Self::ForkedStorm | Self::PotentStorm => "STORM LANTERN",
            Self::GraveMine | Self::GraveMineReach | Self::GraveMineRage => "GRAVE MINEFIELD",
            Self::AstralFlare | Self::AstralFlareBloom | Self::AstralFlarePulse => "ASTRAL FLARE",
            Self::SolarNova | Self::SolarNovaBloom | Self::SolarNovaCataclysm => "SOLAR NOVA",
            Self::EchoCannon | Self::EchoCannonResonance | Self::EchoCannonCataclysm => {
                "ECHO CANNON"
            },
            Self::WraithLash | Self::WraithLashReach | Self::WraithLashRend => "WRAITH LASH",
            Self::HarrowVolley | Self::HarrowVolleyAim | Self::HarrowVolleyPierce => {
                "HARROW VOLLEY"
            },
            Self::AetherSpear | Self::AetherSpearSplit | Self::AetherSpearRage => "AETHER SPEAR",
            Self::RiftPulse | Self::RiftPulseAnchor | Self::RiftPulseCascade => "RIFT PULSE",
            Self::CrescentHalo | Self::CrescentHaloSpiral | Self::CrescentHaloCataclysm => {
                "CRESCENT HALO"
            },
            Self::VoidBloom | Self::VoidBloomReach | Self::VoidBloomCascade => "VOID BLOOM",
            Self::AbyssalMine | Self::AbyssalMineReach | Self::AbyssalMineRage => "ABYSSAL MINES",
            Self::Starfall | Self::StarfallCascade | Self::StarfallCataclysm => "STARFALL",
            Self::PhantomNet | Self::PhantomNetReach | Self::PhantomNetRage => "PHANTOM NET",
            Self::LuminousLance | Self::LuminousLanceFork | Self::LuminousLanceRend => {
                "LUMINOUS LANCE"
            },
            Self::TemporalRift | Self::TemporalRiftAnchor | Self::TemporalRiftSurge => {
                "TEMPORAL RIFT"
            },
            Self::ScytheCyclone | Self::ScytheCycloneSpiral | Self::ScytheCycloneRavage => {
                "SCYTHE CYCLONE"
            },
            Self::GloomVolley | Self::GloomVolleyCage | Self::GloomVolleyEcho => "GLOOM VOLLEY",
            Self::PulseLance | Self::PulseLanceSurge | Self::PulseLanceCataclysm => "PULSE LANCE",
            Self::ShardStorm | Self::ShardStormCascade | Self::ShardStormCataclysm => "SHARD STORM",
            Self::PrismBolts | Self::PrismBoltsTwin | Self::PrismBoltsRift => "PRISM BOLTS",
            Self::InfernoBomb | Self::InfernoBombScatter | Self::InfernoBombCataclysm => {
                "INFERNO BOMB"
            },
            Self::GravityWeave | Self::GravityWeaveAnchor | Self::GravityWeaveCollapse => {
                "GRAVITY WEAVE"
            },
            Self::SigilNet | Self::SigilNetReach | Self::SigilNetRuin => "SIGIL NET",
            Self::RuneWard | Self::RuneWardEcho | Self::RuneWardCataclysm => "RUNE WARD",
            Self::Fleet | Self::Vitality | Self::Magnet => "WITCHCRAFT",
        }
    }
}

#[derive(Copy, Clone)]
struct WeaponBadgeInfo {
    name: &'static str,
    level: u32,
    color: Color,
    charge: f32,
    unlocked: bool,
}

const WEAPON_BADGE_TARGET_COLUMNS_COMPACT: usize = 3;
const WEAPON_BADGE_TARGET_COLUMNS_DESKTOP: usize = 4;
const WEAPON_BADGE_MIN_WIDTH: f32 = 116.0;
const WEAPON_BADGE_MIN_HEIGHT: f32 = 18.0;
const WEAPON_BADGE_MAX_HEIGHT: f32 = 38.0;
const WEAPON_BADGE_MAX_ROWS: usize = 3;
const WEAPON_BADGE_VISIBLE_LIMIT: usize = 12;

struct RunHeaderState {
    chapter: &'static str,
    omen: &'static str,
    mood: Color,
    menace: f32,
    pressures: CombatPulse,
    vector: &'static str,
}

impl Game {
    pub(super) fn draw_hud(&self) {
        self.draw_run_header();
        self.draw_loadout();
    }

    fn draw_run_header(&self) {
        let width = screen_width();
        let compact = width < 720.0;
        let (chapter, omen, mood, menace) = self.night_state();
        let vector = self.flavor_vector();
        let pressures = self.combat_pressures();
        let ritual_warning = self.ritual_warning();
        let ritual_active = self.is_ritual_window();
        let ritual_time = if ritual_active {
            self.ritual_time_remaining()
        } else {
            self.ritual_window_countdown()
        };
        let header_state = RunHeaderState {
            chapter,
            omen,
            mood,
            menace,
            pressures,
            vector,
        };

        self.draw_run_header_background(width);
        self.draw_run_header_texts(width, compact, header_state);
        draw_run_ritual_status(width, compact, ritual_warning, ritual_active, ritual_time);
    }

    fn draw_run_header_background(&self, width: f32) {
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
    }

    fn draw_run_header_texts(&self, width: f32, compact: bool, status: RunHeaderState) {
        let chapter = status.chapter;
        let omen = status.omen;
        let mood = status.mood;
        let menace = status.menace;
        let pressures = status.pressures;
        let vector = status.vector;
        let hud_text_size = if compact { 16.0 } else { 20.0 };
        draw_text(
            format!("LEVEL {:02}", self.player.level),
            22.0,
            50.0,
            hud_text_size,
            BONE,
        );
        draw_text(chapter, 22.0, 69.0, if compact { 12.0 } else { 15.0 }, mood);
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
        draw_text(
            omen,
            width - 24.0,
            if compact { 70.0 } else { 73.0 },
            if compact { 10.0 } else { 13.0 },
            MUTED,
        );
        draw_text(
            format!("PHASE {:.0}", (menace * 100.0).round()),
            width * 0.5 - 34.0,
            34.0,
            if compact { 12.0 } else { 14.0 },
            Color::new(mood.r, mood.g, mood.b, 0.95),
        );
        draw_text_center(
            vector,
            width * 0.5,
            67.0,
            if compact { 10.0 } else { 12.0 },
            Color::new(mood.r * 0.9, mood.g * 0.9, mood.b * 0.9, 0.86),
        );
        draw_combat_pressure_ribbon(width, compact, pressures);
    }

    fn draw_loadout(&self) {
        let width = screen_width();
        let compact = width < 720.0;
        let health = self.draw_health_panel(width, compact);
        let weapon_bars = self.weapon_bars();
        // Prefer unlocked weapons, then fill remaining slots with locked previews.
        let mut visible = Vec::with_capacity(WEAPON_BADGE_VISIBLE_LIMIT);
        for unlocked_only in [true, false] {
            for &badge in &weapon_bars {
                if badge.unlocked == unlocked_only && visible.len() < WEAPON_BADGE_VISIBLE_LIMIT {
                    visible.push(badge);
                }
            }
        }
        draw_weapon_loadout(width, compact, health, &visible);
    }

    fn draw_health_panel(&self, width: f32, compact: bool) -> Rect {
        let health = if compact {
            Rect::new(12.0, screen_height() - 72.0, width - 24.0, 58.0)
        } else {
            Rect::new(16.0, screen_height() - 73.0, 252.0, 57.0)
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
        health
    }

    #[allow(
        clippy::too_many_lines,
        reason = "loadout table is one badge per combat family"
    )]
    fn weapon_bars(&self) -> [WeaponBadgeInfo; COMBAT_WEAPON_BAR_COUNT] {
        let charge = |unlocked: bool, timer: f32, cooldown: f32| -> f32 {
            if !unlocked || cooldown <= 0.0 {
                0.0
            } else {
                (1.0 - (timer.max(0.0) / cooldown)).clamp(0.0, 1.0)
            }
        };
        let badge = |name: &'static str,
                     level: u32,
                     color: Color,
                     unlocked: bool,
                     timer: f32,
                     cooldown: f32| {
            WeaponBadgeInfo {
                name,
                level,
                color,
                charge: charge(unlocked, timer, cooldown),
                unlocked,
            }
        };

        [
            WeaponBadgeInfo {
                name: "MOON KNIVES",
                level: self.moon.level,
                color: MOON_GOLD,
                charge: 1.0,
                unlocked: true,
            },
            badge(
                "STORM LANTERN",
                self.storm.level,
                STORM_CYAN,
                true,
                self.storm_timer,
                self.storm.cooldown,
            ),
            badge(
                "GRAVE MINEFIELD",
                self.grave_mines.level,
                GRAVE_MARROW,
                self.grave_mines.unlocked(),
                self.grave_mines.timer,
                self.grave_mines.cooldown,
            ),
            badge(
                "ASTRAL FLARE",
                self.astral_flare.level,
                CINDER_ORANGE,
                self.astral_flare.unlocked(),
                self.astral_flare.timer,
                self.astral_flare.cooldown,
            ),
            badge(
                "WRAITH LASH",
                self.wraith_lash.level,
                WRAITH_AMETHYST,
                self.wraith_lash.unlocked(),
                self.wraith_lash.timer,
                self.wraith_lash.cooldown,
            ),
            badge(
                "HARROW VOLLEY",
                self.harrow_volley.level,
                ARCANE_VIOLET,
                self.harrow_volley.unlocked(),
                self.harrow_volley.timer,
                self.harrow_volley.cooldown,
            ),
            badge(
                "AETHER SPEAR",
                self.aether_spears.level,
                AETHER_MAUVE,
                self.aether_spears.unlocked(),
                self.aether_spears.timer,
                self.aether_spears.cooldown,
            ),
            badge(
                "RIFT PULSE",
                self.rift_pulse.level,
                RIFT_VIOLET,
                self.rift_pulse.unlocked(),
                self.rift_pulse.timer,
                self.rift_pulse.cooldown,
            ),
            badge(
                "ECHO CANNON",
                self.echo_cannon.level,
                PHANTOM_CERULEAN,
                self.echo_cannon.unlocked(),
                self.echo_cannon.timer,
                self.echo_cannon.cooldown,
            ),
            badge(
                "RUNE WARD",
                self.rune_wards.level,
                PHANTOM_EMBER,
                self.rune_wards.unlocked(),
                self.rune_wards.timer,
                self.rune_wards.cooldown,
            ),
            badge(
                "CRESCENT HALO",
                self.crescent_halo.level,
                CRESCENT_AUREATE,
                self.crescent_halo.unlocked(),
                self.crescent_halo.timer,
                self.crescent_halo.cooldown,
            ),
            badge(
                "VOID BLOOM",
                self.void_bloom.level,
                VOID_BLOOM,
                self.void_bloom.unlocked(),
                self.void_bloom.timer,
                self.void_bloom.cooldown,
            ),
            badge(
                "SOLAR NOVA",
                self.solar_nova.level,
                SOLAR_NOVA,
                self.solar_nova.unlocked(),
                self.solar_nova.timer,
                self.solar_nova.cooldown,
            ),
            badge(
                "ABYSSAL MINES",
                self.abyssal_mines.level,
                ABYSSAL_MINE,
                self.abyssal_mines.unlocked(),
                self.abyssal_mines.timer,
                self.abyssal_mines.cooldown,
            ),
            badge(
                "STARFALL",
                self.starfall.level,
                CINDER_ORANGE,
                self.starfall.unlocked(),
                self.starfall.timer,
                self.starfall.cooldown,
            ),
            badge(
                "PHANTOM NET",
                self.phantom_net.level,
                PHANTOM_NET_GREEN,
                self.phantom_net.unlocked(),
                self.phantom_net.timer,
                self.phantom_net.cooldown,
            ),
            badge(
                "LUMINOUS LANCE",
                self.luminous_lance.level,
                AETHER_MAUVE,
                self.luminous_lance.unlocked(),
                self.luminous_lance.timer,
                self.luminous_lance.cooldown,
            ),
            badge(
                "TEMPORAL RIFT",
                self.temporal_rift.level,
                RIFT_CYAN,
                self.temporal_rift.unlocked(),
                self.temporal_rift.timer,
                self.temporal_rift.cooldown,
            ),
            badge(
                "SCYTHE CYCLONE",
                self.scythe_cyclone.level,
                SCYTHE_IRON,
                self.scythe_cyclone.unlocked(),
                self.scythe_cyclone.timer,
                self.scythe_cyclone.cooldown,
            ),
            badge(
                "GLOOM VOLLEY",
                self.gloom_volley.level,
                GLOOM_PURPLE,
                self.gloom_volley.unlocked(),
                self.gloom_volley.timer,
                self.gloom_volley.cooldown,
            ),
            badge(
                "PULSE LANCE",
                self.pulse_lance.level,
                LANCE_AZURE,
                self.pulse_lance.unlocked(),
                self.pulse_lance.timer,
                self.pulse_lance.cooldown,
            ),
            badge(
                "SHARD STORM",
                self.shard_storm.level,
                GLOOM_PURPLE,
                self.shard_storm.unlocked(),
                self.shard_storm.timer,
                self.shard_storm.cooldown,
            ),
            badge(
                "PRISM BOLTS",
                self.prism_bolts.level,
                PHANTOM_CERULEAN,
                self.prism_bolts.unlocked(),
                self.prism_bolts.timer,
                self.prism_bolts.cooldown,
            ),
            badge(
                "INFERNO BOMB",
                self.inferno_bombs.level,
                CINDER_ORANGE,
                self.inferno_bombs.unlocked(),
                self.inferno_bombs.timer,
                self.inferno_bombs.cooldown,
            ),
            badge(
                "GRAVITY WEAVE",
                self.gravity_weave.level,
                RIFT_CYAN,
                self.gravity_weave.unlocked(),
                self.gravity_weave.timer,
                self.gravity_weave.cooldown,
            ),
            badge(
                "SIGIL NET",
                self.sigil_net.level,
                PHANTOM_NET_GREEN,
                self.sigil_net.unlocked(),
                self.sigil_net.timer,
                self.sigil_net.cooldown,
            ),
        ]
    }

    pub(super) fn draw_title(&self) {
        let center = vec2(screen_width() * 0.5, screen_height() * 0.5);
        let (chapter, ..) = self.night_state();
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
            &format!("{} — {}", chapter, self.flavor_vector()),
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
        draw_text_center(
            self.flavor_vector(),
            center_x,
            center_y - 24.0,
            17.0,
            Color::new(0.85, 0.83, 0.96, 0.95),
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

#[allow(
    clippy::too_many_lines,
    reason = "icon branch map is compact and readable as a switch map"
)]
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
            icon_bolt(center, color, BONE);
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
        Upgrade::WraithLash | Upgrade::WraithLashReach | Upgrade::WraithLashRend => {
            icon_lash(center, color);
        },
        Upgrade::HarrowVolley | Upgrade::HarrowVolleyAim | Upgrade::HarrowVolleyPierce => {
            draw_poly(center.x, center.y + 1.0, 3, 9.0, PI * 0.32, MOON_GOLD);
            draw_poly(center.x, center.y - 1.0, 3, 6.0, PI * -0.22, color);
            draw_circle_lines(center.x - 10.0, center.y, 4.0, 1.5, color);
            draw_circle_lines(center.x + 10.0, center.y, 4.0, 1.5, color);
        },
        // Mine-like weapons
        Upgrade::GraveMine
        | Upgrade::GraveMineReach
        | Upgrade::GraveMineRage
        | Upgrade::AbyssalMine
        | Upgrade::AbyssalMineReach
        | Upgrade::AbyssalMineRage => {
            icon_mine(center, color);
        },
        // Orbiting nets / threads
        Upgrade::PhantomNet
        | Upgrade::PhantomNetReach
        | Upgrade::PhantomNetRage
        | Upgrade::SigilNet
        | Upgrade::SigilNetReach
        | Upgrade::SigilNetRuin => {
            icon_orbit_net(center, color);
        },
        // Expanding rings / wells
        Upgrade::AstralFlare
        | Upgrade::AstralFlareBloom
        | Upgrade::AstralFlarePulse
        | Upgrade::SolarNova
        | Upgrade::SolarNovaBloom
        | Upgrade::SolarNovaCataclysm
        | Upgrade::EchoCannon
        | Upgrade::EchoCannonResonance
        | Upgrade::EchoCannonCataclysm
        | Upgrade::RiftPulse
        | Upgrade::RiftPulseAnchor
        | Upgrade::RiftPulseCascade
        | Upgrade::RuneWard
        | Upgrade::RuneWardEcho
        | Upgrade::RuneWardCataclysm
        | Upgrade::TemporalRift
        | Upgrade::TemporalRiftAnchor
        | Upgrade::TemporalRiftSurge
        | Upgrade::GravityWeave
        | Upgrade::GravityWeaveAnchor
        | Upgrade::GravityWeaveCollapse
        | Upgrade::Starfall
        | Upgrade::StarfallCascade
        | Upgrade::StarfallCataclysm
        | Upgrade::InfernoBomb
        | Upgrade::InfernoBombScatter
        | Upgrade::InfernoBombCataclysm
        | Upgrade::VoidBloom
        | Upgrade::VoidBloomReach
        | Upgrade::VoidBloomCascade => {
            icon_pulse(center, color);
        },
        // Spears / lances / bolts
        Upgrade::AetherSpear
        | Upgrade::AetherSpearSplit
        | Upgrade::AetherSpearRage
        | Upgrade::LuminousLance
        | Upgrade::LuminousLanceFork
        | Upgrade::LuminousLanceRend
        | Upgrade::PulseLance
        | Upgrade::PulseLanceSurge
        | Upgrade::PulseLanceCataclysm
        | Upgrade::PrismBolts
        | Upgrade::PrismBoltsTwin
        | Upgrade::PrismBoltsRift => {
            icon_spear(center, color);
        },
        // Spinning blades
        Upgrade::CrescentHalo
        | Upgrade::CrescentHaloSpiral
        | Upgrade::CrescentHaloCataclysm
        | Upgrade::ScytheCyclone
        | Upgrade::ScytheCycloneSpiral
        | Upgrade::ScytheCycloneRavage => {
            icon_blades(center, color);
        },
        // Shard volleys
        Upgrade::GloomVolley
        | Upgrade::GloomVolleyCage
        | Upgrade::GloomVolleyEcho
        | Upgrade::ShardStorm
        | Upgrade::ShardStormCascade
        | Upgrade::ShardStormCataclysm => {
            icon_shards(center, color);
        },
    }
}

fn icon_bolt(center: Vec2, color: Color, accent: Color) {
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
        accent,
    );
}

fn icon_lash(center: Vec2, color: Color) {
    draw_line(
        center.x - 17.0,
        center.y + 10.0,
        center.x - 2.0,
        center.y - 12.0,
        2.5,
        color,
    );
    draw_line(
        center.x - 2.0,
        center.y - 12.0,
        center.x + 11.0,
        center.y + 12.0,
        2.5,
        color,
    );
    draw_circle(center.x - 7.0, center.y - 2.0, 4.0, color);
    draw_line(
        center.x - 2.0,
        center.y + 12.0,
        center.x + 17.0,
        center.y,
        2.5,
        color,
    );
    draw_circle(center.x + 6.0, center.y + 1.0, 3.0, PANEL);
}

fn icon_mine(center: Vec2, color: Color) {
    draw_circle_lines(center.x, center.y, 16.0, 3.0, color);
    draw_circle(
        center.x,
        center.y + 3.0,
        10.0,
        Color::new(color.r * 0.25, color.g * 0.25, color.b * 0.25, 0.9),
    );
    for offset in [-8.0, -2.0, 4.0] {
        draw_circle(center.x + offset, center.y - 1.0, 3.0, color);
    }
}

fn icon_orbit_net(center: Vec2, color: Color) {
    draw_circle_lines(
        center.x,
        center.y,
        16.0,
        1.8,
        Color::new(color.r, color.g, color.b, 0.85),
    );
    for index in 0..5u8 {
        let angle = index as f32 * PI * 0.4;
        draw_circle(
            center.x + angle.cos() * 8.0,
            center.y + angle.sin() * 8.0,
            2.3,
            color,
        );
    }
    draw_line(
        center.x - 8.0,
        center.y,
        center.x + 8.0,
        center.y,
        1.4,
        color,
    );
}

fn icon_pulse(center: Vec2, color: Color) {
    draw_circle(
        center.x,
        center.y,
        15.0,
        Color::new(color.r, color.g, color.b, 0.22),
    );
    draw_circle_lines(
        center.x,
        center.y,
        16.0,
        2.0,
        Color::new(color.r, color.g, color.b, 0.9),
    );
    for ring in 0..3u8 {
        let radius = 6.0 + ring as f32 * 4.0;
        draw_circle_lines(
            center.x,
            center.y,
            radius,
            1.1,
            Color::new(1.0, 1.0, 1.0, (0.28 - ring as f32 * 0.07).max(0.05)),
        );
    }
}

fn icon_spear(center: Vec2, color: Color) {
    draw_line(
        center.x - 14.0,
        center.y,
        center.x + 14.0,
        center.y,
        2.4,
        Color::new(color.r, color.g, color.b, 0.2),
    );
    draw_line(
        center.x - 12.0,
        center.y - 3.0,
        center.x + 12.0,
        center.y - 3.0,
        2.0,
        color,
    );
    draw_circle(
        center.x + 6.0,
        center.y - 1.0,
        3.0,
        Color::new(1.0, 1.0, 1.0, 0.55),
    );
    draw_circle_lines(
        center.x,
        center.y,
        15.0,
        1.4,
        Color::new(color.r, color.g, color.b, 0.55),
    );
}

fn icon_blades(center: Vec2, color: Color) {
    for index in 0..4u8 {
        let angle = index as f32 * PI * 0.5;
        draw_line(
            center.x,
            center.y + 2.0,
            center.x + angle.cos() * 14.0,
            center.y + angle.sin() * 11.0,
            1.5,
            color,
        );
    }
    draw_circle(
        center.x,
        center.y + 2.0,
        7.0,
        Color::new(color.r, color.g, color.b, 0.35),
    );
}

fn icon_shards(center: Vec2, color: Color) {
    for shard in 0..4u8 {
        let angle = shard as f32 * PI * 0.5;
        draw_line(
            center.x,
            center.y,
            center.x + angle.cos() * 12.0,
            center.y + angle.sin() * 12.0,
            1.5,
            color,
        );
    }
    draw_circle_lines(
        center.x,
        center.y,
        13.0,
        1.6,
        Color::new(color.r, color.g, color.b, 0.82),
    );
}

fn draw_run_ritual_status(
    width: f32,
    compact: bool,
    ritual_warning: f32,
    ritual_active: bool,
    ritual_time: f32,
) {
    if ritual_warning > 0.0 || ritual_active {
        let status = if ritual_active {
            format!("RITUAL · {:>3.0}s", ritual_time.ceil())
        } else {
            format!("RITUAL IN {:>3.0}s", ritual_time.ceil())
        };
        let color = if ritual_active {
            Color::new(1.0, 0.35, 0.35, 0.95)
        } else {
            Color::new(DAMAGE_RED.r, DAMAGE_RED.g, DAMAGE_RED.b, 0.7)
        };
        draw_text(
            &status,
            width * 0.5 + 72.0,
            34.0,
            if compact { 12.0 } else { 14.0 },
            color,
        );
    }
}

fn draw_combat_pressure_ribbon(width: f32, compact: bool, pressures: CombatPulse) {
    let pressure_row_y = if compact { 78.0 } else { 81.0 };
    let bar_height = if compact { 4.0 } else { 5.0 };
    let gutter = if compact { 8.0 } else { 10.0 };
    let metrics = [
        ("W", pressures.wave, MOON_GOLD),
        ("B", pressures.beat, STORM_CYAN),
        ("T", pressures.tempo, WRAITH_AMETHYST),
        ("C", pressures.cluster, CINDER_ORANGE),
        ("E", pressures.enrage, DAMAGE_RED),
        ("P", pressures.time_pulse, ARCANE_VIOLET),
    ];
    let track_width = width - 44.0;
    let lane_width = track_width / metrics.len() as f32;
    for (index, (label, value, color)) in metrics.iter().enumerate() {
        let x = 22.0 + index as f32 * lane_width;
        draw_text(
            label,
            x,
            pressure_row_y,
            if compact { 8.0 } else { 10.0 },
            Color::new(color.r, color.g, color.b, 0.74),
        );
        draw_bar(
            x,
            pressure_row_y + 7.0,
            lane_width - gutter,
            bar_height,
            *value,
            *color,
            Color::new(0.0, 0.0, 0.0, 0.42),
        );
    }
}

fn draw_weapon_loadout(width: f32, compact: bool, health: Rect, weapon_bars: &[WeaponBadgeInfo]) {
    if weapon_bars.is_empty() {
        return;
    }

    let visible_limit = weapon_bars.len().min(WEAPON_BADGE_VISIBLE_LIMIT);
    let target_columns = if compact {
        WEAPON_BADGE_TARGET_COLUMNS_COMPACT
    } else {
        WEAPON_BADGE_TARGET_COLUMNS_DESKTOP
    };

    let gap = if compact { 8.0 } else { 10.0 };
    let loadout_left = if compact {
        12.0
    } else {
        health.x + health.w + 10.0
    };
    let loadout_width = if compact {
        width - 24.0
    } else {
        (width - loadout_left - 14.0).max(250.0)
    };
    let max_columns_by_width = ((loadout_width + gap) / (WEAPON_BADGE_MIN_WIDTH + gap))
        .floor()
        .max(1.0) as usize;
    let mut columns = max_columns_by_width.min(target_columns).min(visible_limit);
    if columns == 0 {
        columns = 1;
    }
    let rows_needed_for_max_height = visible_limit.div_ceil(columns);
    if rows_needed_for_max_height > WEAPON_BADGE_MAX_ROWS {
        let required_columns = visible_limit.div_ceil(WEAPON_BADGE_MAX_ROWS);
        columns = required_columns
            .min(max_columns_by_width)
            .min(visible_limit)
            .max(1);
    }
    let visible_count = visible_limit.min(columns * WEAPON_BADGE_MAX_ROWS);
    let rows = visible_count.div_ceil(columns);
    let weapon_width = (loadout_width - gap * (columns as f32 - 1.0)) / columns as f32;
    let max_total_height = (health.y - 10.0).max(0.0);
    let weapon_height = ((max_total_height - gap * (rows as f32 - 1.0)) / rows as f32)
        .clamp(WEAPON_BADGE_MIN_HEIGHT, WEAPON_BADGE_MAX_HEIGHT);
    let total_height = rows as f32 * weapon_height + gap * (rows as f32 - 1.0);
    let weapon_y = (health.y - total_height - 6.0).max(6.0);

    for (index, &badge) in weapon_bars[..visible_count].iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = loadout_left + (column as f32 * (weapon_width + gap));
        let y = weapon_y + (row as f32 * (weapon_height + gap));
        draw_weapon_badge(Rect::new(x, y, weapon_width, weapon_height), badge);
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

fn draw_weapon_badge(rect: Rect, badge: WeaponBadgeInfo) {
    let WeaponBadgeInfo {
        name,
        level,
        color,
        charge,
        unlocked,
    } = badge;
    draw_panel(rect);
    draw_rectangle(rect.x, rect.y, 4.0, rect.h, color);
    let title_color = if unlocked { BONE } else { MUTED };
    let status_color = if unlocked {
        color
    } else {
        Color::new(color.r, color.g, color.b, 0.42)
    };
    let bar_color = if unlocked {
        color
    } else {
        Color::new(color.r, color.g, color.b, 0.33)
    };
    let title_size = if rect.h < 30.0 { 10.0 } else { 12.0 };
    let status_size = if rect.h < 30.0 { 8.0 } else { 10.0 };
    let bar_height = rect.h.clamp(4.0, 8.0) * 0.88;
    let bar_top = rect.y + rect.h - bar_height - 6.0;
    let text_mid = (rect.h * 0.33).clamp(16.0, 22.0);
    let text_bottom = (rect.h * 0.57).clamp(28.0, 36.0);
    draw_text(name, rect.x + 11.0, text_mid, title_size, title_color);
    let status = if unlocked {
        format!("LV {level}")
    } else {
        "LOCKED".to_string()
    };
    draw_text(
        &status,
        rect.x + 11.0,
        rect.y + (text_bottom - 1.0),
        status_size,
        status_color,
    );
    draw_bar(
        rect.x + 9.0,
        bar_top,
        rect.w - 18.0,
        bar_height,
        charge,
        bar_color,
        Color::new(0.0, 0.0, 0.0, 0.45),
    );
}

fn draw_text_right(text: &str, right: f32, baseline: f32, size: f32, color: Color) {
    let dimensions = measure_text(text, None, size as u16, 1.0);
    draw_text(text, right - dimensions.width, baseline, size, color);
}
