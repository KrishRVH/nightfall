//! Game state and frame-by-frame simulation.
//!
//! [`Game::update`] is the central teaching path: it applies input, enemy AI,
//! weapon damage, deaths, pickups, and effects in an explicit order. Drawing
//! lives in the private `render` child module and cannot mutate this state.

mod render;

use std::f32::consts::TAU;

use macroquad::{
    prelude::{Color, Vec2, screen_height, screen_width, vec2},
    rand,
};

const VIEW_HEIGHT: f32 = 720.0;
const PLAYER_RADIUS: f32 = 18.0;
const MAX_ENEMIES: usize = 420;
const DAMAGE_NUMBER_LIFE: f32 = 0.62;
const ENEMY_SEPARATION_RATE: f32 = 27.0;
const IMPACT_RING_LIFE: f32 = 0.24;
const FLAVOR_SHIFT_SECS: f32 = 3.0;
const ENEMY_BEAT_PERIOD_SECS: f32 = 10.0;
const ENEMY_BEAT_WINDOW_SECS: f32 = 1.5;
const ENEMY_BEAT_PEAK: f32 = 1.25;
const PHASE_FLASH_TIME: f32 = 0.55;
const RITUAL_PERIOD_SECS: f32 = 30.0;
const RITUAL_WARNING_SECS: f32 = 2.6;
const RITUAL_ACTIVE_SECS: f32 = 4.2;
const ENRAGE_WAVE_SECS: f32 = 18.0;
const ENRAGE_WINDOW_SECS: f32 = 3.6;
const ENRAGE_SPAWN_MULTIPLIER: f32 = 1.55;
const ENEMY_WAVE_PERIOD_SECS: f32 = 11.5;
const ENEMY_WAVE_WINDOW_SECS: f32 = 2.7;
const ENEMY_WAVE_SPAWN_BURST: f32 = 3.8;
const ENEMY_PULSE_PERIOD_SECS: f32 = 8.8;
const ENEMY_PULSE_WINDOW_SECS: f32 = 2.4;
const ENEMY_PULSE_SPAWN_BURST: f32 = 0.58;
const ENEMY_CLUSTER_PERIOD_SECS: f32 = 18.0;
const ENEMY_CLUSTER_WINDOW_SECS: f32 = 3.2;
const ENEMY_CLUSTER_SPAWN_BURST: f32 = 1.65;
const ENEMY_TEMPO_PERIOD_SECS: f32 = 19.2;
const ENEMY_TEMPO_PEAK_SHIFT: f32 = 1.05;
const ENEMY_TIMING_PERIOD_SECS: f32 = 31.0;
const ENEMY_TIMING_STRENGTH: f32 = 0.45;
const ENEMY_TIMING_SPAWN_SWAY: f32 = 0.22;
const ENEMY_TIMING_SPAWN_BURST: f32 = 1.8;
const ENEMY_KIND_TIME_SWAY: f32 = 0.24;
const ENEMY_KIND_MOVEMENT_SWAY: f32 = 0.16;
const ENEMY_ORIGIN_SPIN_SECS: f32 = 46.0;
const ENEMY_LANE_COUNT: usize = 6;
const ENEMY_LANE_CYCLE_SECS: f32 = 9.0;
const ENEMY_LANE_SWAY_SECS: f32 = 4.2;
const ENEMY_LANE_WIDTH_SWAY: f32 = 0.24;
const ENEMY_SPEED_TEMPO_SWAY: f32 = 0.22;
const ENEMY_KIND_COUNT: usize = 5;
const ENEMY_TIMING_PHASE_OFFSETS: [f32; ENEMY_KIND_COUNT] =
    [0.0, TAU * 0.2, TAU * 0.4, TAU * 0.6, TAU * 0.8];
const RITUAL_WARNING_START_SECS: f32 =
    RITUAL_PERIOD_SECS - (RITUAL_WARNING_SECS + RITUAL_ACTIVE_SECS);
const RITUAL_BURST_MULTIPLIER: f32 = 1.7;

const INK: Color = Color::new(0.07, 0.055, 0.12, 1.0);
const BACKGROUND: Color = Color::new(0.055, 0.045, 0.10, 1.0);
const BONE: Color = Color::new(0.96, 0.91, 0.79, 1.0);
const MOON_GOLD: Color = Color::new(1.0, 0.74, 0.25, 1.0);
const ARCANE_VIOLET: Color = Color::new(0.58, 0.38, 0.98, 1.0);
const STORM_CYAN: Color = Color::new(0.31, 0.88, 0.96, 1.0);
const DAMAGE_RED: Color = Color::new(0.94, 0.25, 0.32, 1.0);
const GEM_GREEN: Color = Color::new(0.35, 0.87, 0.52, 1.0);
const WRAITH_AMETHYST: Color = Color::new(0.58, 0.32, 0.95, 1.0);
const CINDER_ORANGE: Color = Color::new(0.97, 0.48, 0.17, 1.0);
const GRAVE_MARROW: Color = Color::new(0.54, 0.38, 0.20, 1.0);
const AETHER_MAUVE: Color = Color::new(0.84, 0.46, 0.93, 1.0);
const RIFT_VIOLET: Color = Color::new(0.40, 0.22, 0.78, 1.0);
const PHANTOM_CERULEAN: Color = Color::new(0.42, 0.86, 0.95, 1.0);
const PHANTOM_EMBER: Color = Color::new(0.90, 0.52, 0.74, 1.0);
const CRESCENT_AUREATE: Color = Color::new(0.80, 0.64, 0.98, 1.0);
const VOID_BLOOM: Color = Color::new(0.42, 0.22, 0.57, 1.0);
const SOLAR_NOVA: Color = Color::new(1.0, 0.52, 0.18, 1.0);
const ABYSSAL_MINE: Color = Color::new(0.16, 0.25, 0.48, 1.0);
const STARFALL_GOLD: Color = Color::new(1.0, 0.77, 0.34, 1.0);
const PHANTOM_NET_GREEN: Color = Color::new(0.37, 0.90, 0.55, 1.0);
const LANCE_AZURE: Color = Color::new(0.38, 0.78, 1.0, 1.0);
const RIFT_CYAN: Color = Color::new(0.32, 0.87, 0.95, 1.0);
const SCYTHE_IRON: Color = Color::new(0.74, 0.74, 0.82, 1.0);
const GLOOM_PURPLE: Color = Color::new(0.58, 0.24, 0.76, 1.0);

const FLAVOR_RITUAL: &[&str] = &[
    "The dead draw breath as one.",
    "A brass hum rolls out of every grave.",
    "Lanterns flare toward an unseen choir.",
    "Ritual seams glow in the fog.",
    "A blood-colored bell rings at the edge of hearing.",
    "The sky tightens. Something under the clouds moves.",
    "Shadow vessels open and remember your heartbeat.",
    "The earth leans into the ritual line.",
];

const FLAVOR_TENSION: &[&str] = &[
    "The ground keeps score.",
    "Instinct says turn. Shadow says stay.",
    "Enemy rhythm climbs through your ribs.",
    "Death times itself to your breath.",
    "Ash moves with intent.",
    "Light sharpens, then fails by a thread.",
    "Your shadow is no longer yours.",
    "The night has lost patience.",
];

const FLAVOR_TEMPO: &[&str] = &[
    "The hunt outruns thought.",
    "Enemies arrive two heartbeats early.",
    "Time thins to a metal taste.",
    "Footsteps land before decisions finish.",
    "The safe lane closes like a fist.",
    "You move once; two answers must follow.",
    "Every moment has teeth.",
    "No room left for a slower answer.",
];

#[derive(Clone, Copy, Debug)]
struct NightProfile {
    title: &'static str,
    omen: &'static str,
    vector: [&'static str; 8],
    accent: Color,
    menace: f32,
    spawn_pressure: f32,
    speed_scale: f32,
    health_scale: f32,
}

#[derive(Clone, Copy)]
struct CombatPulse {
    wave: f32,
    beat: f32,
    tempo: f32,
    cluster: f32,
    enrage: f32,
    time_pulse: f32,
}

const NIGHT_PROFILES: [NightProfile; 6] = [
    NightProfile {
        title: "Ember Dusk",
        omen: "the first veil",
        vector: [
            "The crypt keeps time with your breath.",
            "Moonlight arrives in shards.",
            "Your pulse is the loudest lantern.",
            "Stone remembers the shape of your shadow.",
            "A soft bell tolls inside your chest.",
            "Ash settles differently around your boots.",
            "The horizon blinks, then stops pretending.",
            "A new shade moves in the smoke.",
        ],
        accent: ARCANE_VIOLET,
        menace: 0.04,
        spawn_pressure: 1.00,
        speed_scale: 0.86,
        health_scale: 0.0,
    },
    NightProfile {
        title: "Witching Gloom",
        omen: "hollows murmur",
        vector: [
            "Moonlight forgets the path home.",
            "The wind has learned your name.",
            "Old iron sits on the tongue.",
            "Dry leaves follow like witnesses.",
            "Every alley has one extra shadow.",
            "Witchfire glows where no moon reaches.",
            "Ritual chalk on the cobbles points inward.",
            "Each breath leaves a language the fog can read.",
        ],
        accent: STORM_CYAN,
        menace: 0.11,
        spawn_pressure: 0.82,
        speed_scale: 0.95,
        health_scale: 0.03,
    },
    NightProfile {
        title: "Howler's Weave",
        omen: "the choir is awake",
        vector: [
            "The wind carries teeth instead of mercy.",
            "Ash and static trade places in your ribs.",
            "Nothing is still long enough to trust.",
            "Silence is only the pause before hunger.",
            "Your shadow reaches toward the wrong things.",
            "The moon thins whenever you blink.",
            "Howls leave frost where no wind blows.",
            "One lantern dies; five answer.",
        ],
        accent: WRAITH_AMETHYST,
        menace: 0.16,
        spawn_pressure: 0.72,
        speed_scale: 1.02,
        health_scale: 0.08,
    },
    NightProfile {
        title: "Moonless Rime",
        omen: "vigil shudders",
        vector: [
            "Rifts open where grief can hear you.",
            "The horizon swallows stars one blink at a time.",
            "Every silence is a hand closing.",
            "The cold has a voice, and it knows your name.",
            "Shadows hurry where no wind blows.",
            "The fog opens a mouth full of teeth.",
            "Snow peels from the dark like skin.",
            "Your heartbeat keeps score against a stopped clock.",
        ],
        accent: CINDER_ORANGE,
        menace: 0.20,
        spawn_pressure: 0.66,
        speed_scale: 1.10,
        health_scale: 0.13,
    },
    NightProfile {
        title: "Cathedral of Ash",
        omen: "the dead answer",
        vector: [
            "All prayers are echoes now.",
            "Runes ring like teeth in fog.",
            "Grief hangs like glass over the valley.",
            "A cathedral of bone rises underfoot.",
            "You carry two shadows: one yours, one borrowed.",
            "A choir without mouths hums beneath the soil.",
            "The graveyard is listening.",
            "Not even mercy has directions here.",
        ],
        accent: CINDER_ORANGE,
        menace: 0.24,
        spawn_pressure: 0.58,
        speed_scale: 1.18,
        health_scale: 0.18,
    },
    NightProfile {
        title: "Crown of Briar",
        omen: "the night breaks law",
        vector: [
            "One light remains, one breath, then none.",
            "You hear the world inhale and hold it.",
            "The night remembers it owns this hour.",
            "A final bell rings under the ribs of the sky.",
            "Fear has a thousand names; none are yours.",
            "You do not outrun the night; you bargain.",
            "The dark no longer ends at the horizon.",
            "The world is a throat deciding to speak.",
        ],
        accent: DAMAGE_RED,
        menace: 0.30,
        spawn_pressure: 0.50,
        speed_scale: 1.26,
        health_scale: 0.24,
    },
];

/// A frame of device input translated into game concepts.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Input {
    pub(crate) movement: Vec2,
    pub(crate) choice: Option<usize>,
    pub(crate) pointer: Option<Vec2>,
    pub(crate) accept: bool,
    pub(crate) pause: bool,
    pub(crate) retry: bool,
}

/// A request for the outer application loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Control {
    Continue,
    Restart,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Title,
    Running,
    LevelUp,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, Debug)]
struct Player {
    position: Vec2,
    velocity: Vec2,
    facing: f32,
    health: f32,
    max_health: f32,
    speed: f32,
    pickup_radius: f32,
    invulnerability: f32,
    level: u32,
    experience: u32,
    next_level: u32,
}

impl Player {
    const fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            facing: 1.0,
            health: 100.0,
            max_health: 100.0,
            speed: 260.0,
            pickup_radius: 105.0,
            invulnerability: 0.0,
            level: 1,
            experience: 0,
            next_level: 9,
        }
    }

    fn lantern_position(self) -> Vec2 {
        self.position + vec2(-self.facing * 20.0, 2.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct MoonKnives {
    level: u32,
    count: usize,
    damage: f32,
    radius: f32,
    speed: f32,
}

impl MoonKnives {
    const fn new() -> Self {
        Self {
            level: 1,
            count: 2,
            damage: 18.0,
            radius: 88.0,
            speed: 2.8,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct StormLantern {
    level: u32,
    damage: f32,
    cooldown: f32,
    range: f32,
    jumps: usize,
}

impl StormLantern {
    const fn new() -> Self {
        Self {
            level: 1,
            damage: 24.0,
            cooldown: 1.45,
            range: 340.0,
            jumps: 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GraveMinefield {
    level: u32,
    shard_count: usize,
    cooldown: f32,
    timer: f32,
    damage: f32,
    blast_radius: f32,
    homing: f32,
}

impl GraveMinefield {
    const fn new() -> Self {
        Self {
            level: 0,
            shard_count: 0,
            cooldown: 2.35,
            timer: 0.0,
            damage: 26.0,
            blast_radius: 76.0,
            homing: 205.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            shard_count: 3,
            cooldown: 2.35,
            timer: 0.0,
            damage: 26.0,
            blast_radius: 76.0,
            homing: 205.0,
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct AstralFlare {
    level: u32,
    cooldown: f32,
    timer: f32,
    radius: f32,
    damage: f32,
}

impl AstralFlare {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.3,
            timer: 0.0,
            radius: 84.0,
            damage: 30.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct WraithLash {
    level: u32,
    cooldown: f32,
    timer: f32,
    range: f32,
    width: f32,
    damage: f32,
}

impl WraithLash {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 0.8,
            timer: 0.0,
            range: 255.0,
            width: 12.0,
            damage: 18.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct HarrowVolley {
    level: u32,
    cooldown: f32,
    timer: f32,
    speed: f32,
    dart_count: usize,
    damage: f32,
    range: f32,
}

impl HarrowVolley {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 1.2,
            timer: 0.0,
            speed: 290.0,
            dart_count: 0,
            damage: 20.0,
            range: 390.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            dart_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct AetherSpear {
    level: u32,
    cooldown: f32,
    timer: f32,
    spear_count: usize,
    speed: f32,
    damage: f32,
    pierce: usize,
    range: f32,
    homing: f32,
}

impl AetherSpear {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 1.25,
            timer: 0.0,
            spear_count: 0,
            speed: 350.0,
            damage: 15.0,
            pierce: 1,
            range: 470.0,
            homing: 0.22,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            spear_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct AetherSpearProjectile {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    damage: f32,
    radius: f32,
    pierce: usize,
    range: f32,
}

#[derive(Clone, Copy, Debug)]
struct RiftPulse {
    level: u32,
    cooldown: f32,
    timer: f32,
    radius: f32,
    duration: f32,
    tick_interval: f32,
    damage: f32,
    range: f32,
}

impl RiftPulse {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.4,
            timer: 0.0,
            radius: 96.0,
            duration: 0.86,
            tick_interval: 0.2,
            damage: 18.0,
            range: 520.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct EchoCannon {
    level: u32,
    cooldown: f32,
    timer: f32,
    shard_count: usize,
    speed: f32,
    damage: f32,
    range: f32,
    bounces: usize,
}

impl EchoCannon {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 1.45,
            timer: 0.0,
            shard_count: 0,
            speed: 345.0,
            damage: 20.0,
            range: 500.0,
            bounces: 0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            shard_count: 3,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct EchoShard {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    damage: f32,
    radius: f32,
    bounces_left: usize,
}

#[derive(Clone, Copy, Debug)]
struct RuneWard {
    level: u32,
    cooldown: f32,
    timer: f32,
    ward_count: usize,
    radius: f32,
    duration: f32,
    pulse_interval: f32,
    damage: f32,
    pull: f32,
}

impl RuneWard {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.2,
            timer: 0.0,
            ward_count: 0,
            radius: 84.0,
            duration: 0.78,
            pulse_interval: 0.18,
            damage: 18.0,
            pull: 42.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            ward_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct CrescentHalo {
    level: u32,
    cooldown: f32,
    timer: f32,
    blades: usize,
    radius: f32,
    damage: f32,
    width: f32,
    rotation: f32,
}

impl CrescentHalo {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 1.15,
            timer: 0.0,
            blades: 0,
            radius: 132.0,
            damage: 15.0,
            width: 5.0,
            rotation: 1.55,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            blades: 4,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct VoidBloom {
    level: u32,
    cooldown: f32,
    timer: f32,
    seed_count: usize,
    speed: f32,
    seed_life: f32,
    damage: f32,
    blast_radius: f32,
    homing: f32,
}

impl VoidBloom {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.2,
            timer: 0.0,
            seed_count: 0,
            speed: 66.0,
            seed_life: 1.58,
            damage: 19.0,
            blast_radius: 86.0,
            homing: 165.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            seed_count: 3,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct SolarNova {
    level: u32,
    cooldown: f32,
    timer: f32,
    rings: usize,
    radius: f32,
    damage: f32,
}

impl SolarNova {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.5,
            timer: 0.0,
            rings: 2,
            radius: 84.0,
            damage: 26.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct AbyssalMine {
    level: u32,
    cooldown: f32,
    timer: f32,
    mine_count: usize,
    damage: f32,
    blast_radius: f32,
    homing: f32,
}

impl AbyssalMine {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.8,
            timer: 0.0,
            mine_count: 0,
            damage: 29.0,
            blast_radius: 85.0,
            homing: 175.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            mine_count: 3,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct Starfall {
    level: u32,
    cooldown: f32,
    timer: f32,
    meteor_count: usize,
    damage: f32,
    blast_radius: f32,
    speed: f32,
    gravity: f32,
}

impl Starfall {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.9,
            timer: 0.0,
            meteor_count: 0,
            damage: 28.0,
            blast_radius: 92.0,
            speed: 195.0,
            gravity: 54.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            meteor_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct PhantomNet {
    level: u32,
    cooldown: f32,
    timer: f32,
    thread_count: usize,
    damage: f32,
    blast_radius: f32,
    orbit_radius: f32,
    seek: f32,
}

impl PhantomNet {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 3.2,
            timer: 0.0,
            thread_count: 0,
            damage: 22.0,
            blast_radius: 26.0,
            orbit_radius: 76.0,
            seek: 160.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            thread_count: 3,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct PhantomTether {
    angle: f32,
    orbit_radius: f32,
    angular_speed: f32,
    timer: f32,
    arming: f32,
    damage: f32,
    blast_radius: f32,
}

#[derive(Clone, Copy, Debug)]
struct LuminousLance {
    level: u32,
    cooldown: f32,
    timer: f32,
    spear_count: usize,
    damage: f32,
    range: f32,
    speed: f32,
    homing: f32,
    pierce: usize,
}

impl LuminousLance {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 1.65,
            timer: 0.0,
            spear_count: 0,
            damage: 18.0,
            range: 390.0,
            speed: 520.0,
            homing: 0.2,
            pierce: 0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            spear_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct LuminousLanceBolt {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    damage: f32,
    radius: f32,
    range: f32,
    pierce: usize,
}

#[derive(Clone, Copy, Debug)]
struct TemporalRift {
    level: u32,
    cooldown: f32,
    timer: f32,
    gate_count: usize,
    duration: f32,
    interval: f32,
    radius: f32,
    damage: f32,
    pull: f32,
}

impl TemporalRift {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 3.4,
            timer: 0.0,
            gate_count: 0,
            duration: 0.86,
            interval: 0.16,
            radius: 88.0,
            damage: 14.0,
            pull: 50.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            gate_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct TemporalRiftWell {
    center: Vec2,
    life: f32,
    max_life: f32,
    timer: f32,
    interval: f32,
    max_radius: f32,
    damage: f32,
    pull: f32,
}

#[derive(Clone, Copy, Debug)]
struct ScytheCyclone {
    level: u32,
    cooldown: f32,
    timer: f32,
    blades: usize,
    radius: f32,
    width: f32,
    damage: f32,
}

impl ScytheCyclone {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 1.45,
            timer: 0.0,
            blades: 0,
            radius: 125.0,
            width: 4.5,
            damage: 17.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            blades: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct GloomVolley {
    level: u32,
    cooldown: f32,
    timer: f32,
    shard_count: usize,
    damage: f32,
    range: f32,
    speed: f32,
    piercing: usize,
}

impl GloomVolley {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.05,
            timer: 0.0,
            shard_count: 0,
            damage: 16.0,
            range: 470.0,
            speed: 285.0,
            piercing: 0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            shard_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct GloomShard {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    damage: f32,
    radius: f32,
    pierce: usize,
    range: f32,
}

#[derive(Clone, Copy, Debug)]
struct PulseLance {
    level: u32,
    cooldown: f32,
    timer: f32,
    range: f32,
    width: f32,
    damage: f32,
    chain_count: usize,
}

impl PulseLance {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 0.95,
            timer: 0.0,
            range: 340.0,
            width: 12.0,
            damage: 18.0,
            chain_count: 0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            chain_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct ShardStorm {
    level: u32,
    cooldown: f32,
    timer: f32,
    shard_count: usize,
    speed: f32,
    damage: f32,
    range: f32,
    pierce: usize,
}

impl ShardStorm {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.15,
            timer: 0.0,
            shard_count: 0,
            speed: 286.0,
            damage: 15.0,
            range: 500.0,
            pierce: 0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            shard_count: 3,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct PrismNeedle {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    damage: f32,
    radius: f32,
    pierce: usize,
    range: f32,
    homing: f32,
    speed: f32,
}

#[derive(Clone, Copy, Debug)]
struct PrismBolts {
    level: u32,
    cooldown: f32,
    timer: f32,
    shot_count: usize,
    speed: f32,
    damage: f32,
    range: f32,
    homing: f32,
    pierce: usize,
}

impl PrismBolts {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 1.45,
            timer: 0.0,
            shot_count: 0,
            speed: 430.0,
            damage: 17.0,
            range: 500.0,
            homing: 0.21,
            pierce: 0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            shot_count: 2,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct InfernoFireball {
    position: Vec2,
    velocity: Vec2,
    timer: f32,
    arming: f32,
    damage: f32,
    blast_radius: f32,
    gravity: f32,
}

#[derive(Clone, Copy, Debug)]
struct InfernoBomb {
    level: u32,
    cooldown: f32,
    timer: f32,
    meteor_count: usize,
    speed: f32,
    damage: f32,
    blast_radius: f32,
    gravity: f32,
}

impl InfernoBomb {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 2.8,
            timer: 0.0,
            meteor_count: 0,
            speed: 220.0,
            damage: 34.0,
            blast_radius: 88.0,
            gravity: 58.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            meteor_count: 1,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct GravityWell {
    center: Vec2,
    life: f32,
    max_life: f32,
    timer: f32,
    interval: f32,
    max_radius: f32,
    damage: f32,
    pull: f32,
}

#[derive(Clone, Copy, Debug)]
struct GravityWeave {
    level: u32,
    cooldown: f32,
    timer: f32,
    well_count: usize,
    duration: f32,
    interval: f32,
    max_radius: f32,
    damage: f32,
    pull: f32,
}

impl GravityWeave {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 3.2,
            timer: 0.0,
            well_count: 0,
            duration: 0.78,
            interval: 0.14,
            max_radius: 86.0,
            damage: 13.0,
            pull: 48.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            well_count: 1,
            pull: 58.0,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct SigilThread {
    angle: f32,
    orbit_radius: f32,
    angular_speed: f32,
    timer: f32,
    arming: f32,
    damage: f32,
    blast_radius: f32,
}

#[derive(Clone, Copy, Debug)]
struct SigilNet {
    level: u32,
    cooldown: f32,
    timer: f32,
    thread_count: usize,
    orbit_radius: f32,
    speed: f32,
    damage: f32,
    blast_radius: f32,
    pull: f32,
}

impl SigilNet {
    const fn new() -> Self {
        Self {
            level: 0,
            cooldown: 3.1,
            timer: 0.0,
            thread_count: 0,
            orbit_radius: 76.0,
            speed: 0.82,
            damage: 18.0,
            blast_radius: 24.0,
            pull: 0.0,
        }
    }

    fn unlock(&mut self) {
        if self.level > 0 {
            return;
        }
        *self = Self {
            level: 1,
            thread_count: 2,
            pull: 44.0,
            ..Self::new()
        };
    }

    const fn unlocked(self) -> bool {
        self.level > 0
    }
}

#[derive(Clone, Copy, Debug)]
struct RunePulse {
    position: Vec2,
    life: f32,
    max_life: f32,
    timer: f32,
    interval: f32,
    radius: f32,
    damage: f32,
    pull: f32,
}

#[derive(Clone, Copy, Debug)]
struct RiftEcho {
    center: Vec2,
    life: f32,
    max_life: f32,
    timer: f32,
    interval: f32,
    max_radius: f32,
    damage: f32,
    range_scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnemyKind {
    Shade,
    Wisp,
    Brute,
    Howl,
    Revenant,
}

impl EnemyKind {
    const fn radius(self) -> f32 {
        match self {
            Self::Shade => 16.0,
            Self::Wisp => 12.0,
            Self::Howl => 14.0,
            Self::Brute | Self::Revenant => 25.0,
        }
    }

    const fn base_health(self) -> f32 {
        match self {
            Self::Shade => 34.0,
            Self::Wisp => 22.0,
            Self::Brute => 105.0,
            Self::Howl => 44.0,
            Self::Revenant => 122.0,
        }
    }

    const fn speed(self) -> f32 {
        match self {
            Self::Shade => 72.0,
            Self::Wisp => 112.0,
            Self::Brute => 43.0,
            Self::Howl => 132.0,
            Self::Revenant => 54.0,
        }
    }

    const fn contact_damage(self) -> f32 {
        match self {
            Self::Shade => 13.0,
            Self::Wisp => 9.0,
            Self::Brute => 22.0,
            Self::Howl => 11.0,
            Self::Revenant => 24.0,
        }
    }

    const fn experience(self) -> u32 {
        match self {
            Self::Shade | Self::Wisp => 3,
            Self::Howl => 4,
            Self::Brute | Self::Revenant => 10,
        }
    }

    const fn timing_index(self) -> usize {
        match self {
            Self::Shade => 0,
            Self::Wisp => 1,
            Self::Howl => 2,
            Self::Brute => 3,
            Self::Revenant => 4,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Enemy {
    kind: EnemyKind,
    position: Vec2,
    velocity: Vec2,
    health: f32,
    radius: f32,
    flash: f32,
    // One cooldown per enemy prevents overlapping moon knives from damaging every frame.
    moon_immunity: f32,
    phase: f32,
}

#[derive(Clone, Copy, Debug)]
struct Gem {
    position: Vec2,
    velocity: Vec2,
    value: u32,
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: Color,
}

#[derive(Clone, Debug)]
struct Lightning {
    points: Vec<Vec2>,
    life: f32,
}

#[derive(Clone, Copy, Debug)]
struct DamageNumber {
    position: Vec2,
    value: u32,
    life: f32,
    color: Color,
}

#[derive(Clone, Copy, Debug)]
struct ImpactRing {
    position: Vec2,
    life: f32,
    max_radius: f32,
    color: Color,
}

/// Homing charge that arms, seeks, then detonates (mines, seeds, meteors share this).
#[derive(Clone, Copy, Debug)]
struct ArmedCharge {
    position: Vec2,
    velocity: Vec2,
    timer: f32,
    arming: f32,
    damage: f32,
    blast_radius: f32,
}

type GraveShard = ArmedCharge;
type AbyssalMinelet = ArmedCharge;
type RuneSeed = ArmedCharge;
type StarfallMeteor = ArmedCharge;

#[derive(Clone, Copy)]
struct ChargeMotion {
    seek_range: f32,
    homing: f32,
    seek_rate: f32,
    proximity_pad: f32,
    gravity: f32,
}

impl ChargeMotion {
    const fn homing(seek_range: f32, homing: f32, seek_rate: f32) -> Self {
        Self {
            seek_range,
            homing,
            seek_rate,
            proximity_pad: 12.0,
            gravity: 0.0,
        }
    }

    const fn falling(gravity: f32, proximity_pad: f32) -> Self {
        Self {
            seek_range: 0.0,
            homing: 0.0,
            seek_rate: 0.0,
            proximity_pad,
            gravity,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FlarePulse {
    position: Vec2,
    life: f32,
    max_radius: f32,
}

#[derive(Clone, Copy, Debug)]
struct WraithTrace {
    start: Vec2,
    end: Vec2,
    life: f32,
    width: f32,
}

#[derive(Clone, Copy, Debug)]
struct HarrowDart {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    damage: f32,
    radius: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Upgrade {
    ExtraKnife,
    SharpenedMoon,
    WiderOrbit,
    FastStorm,
    ForkedStorm,
    PotentStorm,

    GraveMine,
    GraveMineReach,
    GraveMineRage,

    AstralFlare,
    AstralFlareBloom,
    AstralFlarePulse,

    WraithLash,
    WraithLashReach,
    WraithLashRend,

    HarrowVolley,
    HarrowVolleyAim,
    HarrowVolleyPierce,
    AetherSpear,
    AetherSpearSplit,
    AetherSpearRage,
    RiftPulse,
    RiftPulseAnchor,
    RiftPulseCascade,

    EchoCannon,
    EchoCannonResonance,
    EchoCannonCataclysm,
    RuneWard,
    RuneWardEcho,
    RuneWardCataclysm,
    CrescentHalo,
    CrescentHaloSpiral,
    CrescentHaloCataclysm,
    VoidBloom,
    VoidBloomReach,
    VoidBloomCascade,
    SolarNova,
    SolarNovaBloom,
    SolarNovaCataclysm,
    AbyssalMine,
    AbyssalMineReach,
    AbyssalMineRage,

    Starfall,
    StarfallCascade,
    StarfallCataclysm,
    PhantomNet,
    PhantomNetReach,
    PhantomNetRage,
    LuminousLance,
    LuminousLanceFork,
    LuminousLanceRend,
    TemporalRift,
    TemporalRiftAnchor,
    TemporalRiftSurge,
    ScytheCyclone,
    ScytheCycloneSpiral,
    ScytheCycloneRavage,
    GloomVolley,
    GloomVolleyCage,
    GloomVolleyEcho,
    PulseLance,
    PulseLanceSurge,
    PulseLanceCataclysm,
    ShardStorm,
    ShardStormCascade,
    ShardStormCataclysm,
    PrismBolts,
    PrismBoltsTwin,
    PrismBoltsRift,
    InfernoBomb,
    InfernoBombScatter,
    InfernoBombCataclysm,
    GravityWeave,
    GravityWeaveAnchor,
    GravityWeaveCollapse,
    SigilNet,
    SigilNetReach,
    SigilNetRuin,

    Fleet,
    Vitality,
    Magnet,
}

impl Upgrade {
    const FAMILY_MOON: [Self; 3] = [Self::ExtraKnife, Self::SharpenedMoon, Self::WiderOrbit];
    const FAMILY_STORM: [Self; 3] = [Self::FastStorm, Self::ForkedStorm, Self::PotentStorm];
    const FAMILY_GRAVE: [Self; 3] = [Self::GraveMine, Self::GraveMineReach, Self::GraveMineRage];
    const FAMILY_FLARE: [Self; 3] = [
        Self::AstralFlare,
        Self::AstralFlareBloom,
        Self::AstralFlarePulse,
    ];
    const FAMILY_WRAITH: [Self; 3] = [
        Self::WraithLash,
        Self::WraithLashReach,
        Self::WraithLashRend,
    ];
    const FAMILY_HARROW: [Self; 3] = [
        Self::HarrowVolley,
        Self::HarrowVolleyAim,
        Self::HarrowVolleyPierce,
    ];
    const FAMILY_AETHER: [Self; 3] = [
        Self::AetherSpear,
        Self::AetherSpearSplit,
        Self::AetherSpearRage,
    ];
    const FAMILY_RIFT: [Self; 3] = [
        Self::RiftPulse,
        Self::RiftPulseAnchor,
        Self::RiftPulseCascade,
    ];
    const FAMILY_ECHO: [Self; 3] = [
        Self::EchoCannon,
        Self::EchoCannonResonance,
        Self::EchoCannonCataclysm,
    ];
    const FAMILY_WARD: [Self; 3] = [Self::RuneWard, Self::RuneWardEcho, Self::RuneWardCataclysm];
    const FAMILY_HALO: [Self; 3] = [
        Self::CrescentHalo,
        Self::CrescentHaloSpiral,
        Self::CrescentHaloCataclysm,
    ];
    const FAMILY_VOID: [Self; 3] = [
        Self::VoidBloom,
        Self::VoidBloomReach,
        Self::VoidBloomCascade,
    ];
    const FAMILY_SOLAR: [Self; 3] = [
        Self::SolarNova,
        Self::SolarNovaBloom,
        Self::SolarNovaCataclysm,
    ];
    const FAMILY_ABYSSAL: [Self; 3] = [
        Self::AbyssalMine,
        Self::AbyssalMineReach,
        Self::AbyssalMineRage,
    ];
    const FAMILY_STARFALL: [Self; 3] = [
        Self::Starfall,
        Self::StarfallCascade,
        Self::StarfallCataclysm,
    ];
    const FAMILY_PHANTOM_NET: [Self; 3] = [
        Self::PhantomNet,
        Self::PhantomNetReach,
        Self::PhantomNetRage,
    ];
    const FAMILY_LUMINOUS_LANCE: [Self; 3] = [
        Self::LuminousLance,
        Self::LuminousLanceFork,
        Self::LuminousLanceRend,
    ];
    const FAMILY_TEMPORAL_RIFT: [Self; 3] = [
        Self::TemporalRift,
        Self::TemporalRiftAnchor,
        Self::TemporalRiftSurge,
    ];
    const FAMILY_SCYTHE: [Self; 3] = [
        Self::ScytheCyclone,
        Self::ScytheCycloneSpiral,
        Self::ScytheCycloneRavage,
    ];
    const FAMILY_GLOOM: [Self; 3] = [
        Self::GloomVolley,
        Self::GloomVolleyCage,
        Self::GloomVolleyEcho,
    ];
    const FAMILY_PULSE_LANCE: [Self; 3] = [
        Self::PulseLance,
        Self::PulseLanceSurge,
        Self::PulseLanceCataclysm,
    ];
    const FAMILY_SHARD_STORM: [Self; 3] = [
        Self::ShardStorm,
        Self::ShardStormCascade,
        Self::ShardStormCataclysm,
    ];
    const FAMILY_PRISM_BOLTS: [Self; 3] =
        [Self::PrismBolts, Self::PrismBoltsTwin, Self::PrismBoltsRift];
    const FAMILY_INFERNO_BOMBS: [Self; 3] = [
        Self::InfernoBomb,
        Self::InfernoBombScatter,
        Self::InfernoBombCataclysm,
    ];
    const FAMILY_GRAVITY_WEAVE: [Self; 3] = [
        Self::GravityWeave,
        Self::GravityWeaveAnchor,
        Self::GravityWeaveCollapse,
    ];
    const FAMILY_SIGIL_NET: [Self; 3] = [Self::SigilNet, Self::SigilNetReach, Self::SigilNetRuin];
    const FAMILY_WITCH: [Self; 3] = [Self::Fleet, Self::Vitality, Self::Magnet];

    const fn family(self) -> UpgradeFamily {
        match self {
            Self::ExtraKnife | Self::SharpenedMoon | Self::WiderOrbit => UpgradeFamily::Moon,
            Self::FastStorm | Self::ForkedStorm | Self::PotentStorm => UpgradeFamily::Storm,
            Self::GraveMine | Self::GraveMineReach | Self::GraveMineRage => UpgradeFamily::Grave,
            Self::AstralFlare | Self::AstralFlareBloom | Self::AstralFlarePulse => {
                UpgradeFamily::Flare
            },
            Self::WraithLash | Self::WraithLashReach | Self::WraithLashRend => {
                UpgradeFamily::Wraith
            },
            Self::HarrowVolley | Self::HarrowVolleyAim | Self::HarrowVolleyPierce => {
                UpgradeFamily::Harrow
            },
            Self::AetherSpear | Self::AetherSpearSplit | Self::AetherSpearRage => {
                UpgradeFamily::Aether
            },
            Self::RiftPulse | Self::RiftPulseAnchor | Self::RiftPulseCascade => UpgradeFamily::Rift,
            Self::EchoCannon | Self::EchoCannonResonance | Self::EchoCannonCataclysm => {
                UpgradeFamily::Echo
            },
            Self::RuneWard | Self::RuneWardEcho | Self::RuneWardCataclysm => UpgradeFamily::Ward,
            Self::CrescentHalo | Self::CrescentHaloSpiral | Self::CrescentHaloCataclysm => {
                UpgradeFamily::Halo
            },
            Self::VoidBloom | Self::VoidBloomReach | Self::VoidBloomCascade => UpgradeFamily::Void,
            Self::SolarNova | Self::SolarNovaBloom | Self::SolarNovaCataclysm => {
                UpgradeFamily::Solar
            },
            Self::AbyssalMine | Self::AbyssalMineReach | Self::AbyssalMineRage => {
                UpgradeFamily::Abyssal
            },
            Self::Starfall | Self::StarfallCascade | Self::StarfallCataclysm => {
                UpgradeFamily::Starfall
            },
            Self::PhantomNet | Self::PhantomNetReach | Self::PhantomNetRage => {
                UpgradeFamily::PhantomNet
            },
            Self::LuminousLance | Self::LuminousLanceFork | Self::LuminousLanceRend => {
                UpgradeFamily::LuminousLance
            },
            Self::TemporalRift | Self::TemporalRiftAnchor | Self::TemporalRiftSurge => {
                UpgradeFamily::TemporalRift
            },
            Self::ScytheCyclone | Self::ScytheCycloneSpiral | Self::ScytheCycloneRavage => {
                UpgradeFamily::Scythe
            },
            Self::GloomVolley | Self::GloomVolleyCage | Self::GloomVolleyEcho => {
                UpgradeFamily::Gloom
            },
            Self::PulseLance | Self::PulseLanceSurge | Self::PulseLanceCataclysm => {
                UpgradeFamily::PulseLance
            },
            Self::ShardStorm | Self::ShardStormCascade | Self::ShardStormCataclysm => {
                UpgradeFamily::ShardStorm
            },
            Self::PrismBolts | Self::PrismBoltsTwin | Self::PrismBoltsRift => {
                UpgradeFamily::PrismBolts
            },
            Self::InfernoBomb | Self::InfernoBombScatter | Self::InfernoBombCataclysm => {
                UpgradeFamily::Inferno
            },
            Self::GravityWeave | Self::GravityWeaveAnchor | Self::GravityWeaveCollapse => {
                UpgradeFamily::Gravity
            },
            Self::SigilNet | Self::SigilNetReach | Self::SigilNetRuin => UpgradeFamily::SigilNet,
            Self::Fleet | Self::Vitality | Self::Magnet => UpgradeFamily::Witch,
        }
    }

    const fn is_witchcraft(self) -> bool {
        matches!(self.family(), UpgradeFamily::Witch)
    }

    const fn family_pool(family: UpgradeFamily) -> [Self; 3] {
        match family {
            UpgradeFamily::Moon => Self::FAMILY_MOON,
            UpgradeFamily::Storm => Self::FAMILY_STORM,
            UpgradeFamily::Grave => Self::FAMILY_GRAVE,
            UpgradeFamily::Flare => Self::FAMILY_FLARE,
            UpgradeFamily::Wraith => Self::FAMILY_WRAITH,
            UpgradeFamily::Harrow => Self::FAMILY_HARROW,
            UpgradeFamily::Aether => Self::FAMILY_AETHER,
            UpgradeFamily::Rift => Self::FAMILY_RIFT,
            UpgradeFamily::Echo => Self::FAMILY_ECHO,
            UpgradeFamily::Ward => Self::FAMILY_WARD,
            UpgradeFamily::Witch => Self::FAMILY_WITCH,
            UpgradeFamily::Halo => Self::FAMILY_HALO,
            UpgradeFamily::Void => Self::FAMILY_VOID,
            UpgradeFamily::Solar => Self::FAMILY_SOLAR,
            UpgradeFamily::Abyssal => Self::FAMILY_ABYSSAL,
            UpgradeFamily::Starfall => Self::FAMILY_STARFALL,
            UpgradeFamily::PhantomNet => Self::FAMILY_PHANTOM_NET,
            UpgradeFamily::LuminousLance => Self::FAMILY_LUMINOUS_LANCE,
            UpgradeFamily::TemporalRift => Self::FAMILY_TEMPORAL_RIFT,
            UpgradeFamily::Scythe => Self::FAMILY_SCYTHE,
            UpgradeFamily::Gloom => Self::FAMILY_GLOOM,
            UpgradeFamily::PulseLance => Self::FAMILY_PULSE_LANCE,
            UpgradeFamily::ShardStorm => Self::FAMILY_SHARD_STORM,
            UpgradeFamily::PrismBolts => Self::FAMILY_PRISM_BOLTS,
            UpgradeFamily::Inferno => Self::FAMILY_INFERNO_BOMBS,
            UpgradeFamily::Gravity => Self::FAMILY_GRAVITY_WEAVE,
            UpgradeFamily::SigilNet => Self::FAMILY_SIGIL_NET,
        }
    }
}

const OFFER_FAMILY_COUNT: usize = 27;
const OFFER_SLOT_COUNT: usize = 3;
// Only combat families are shown in the HUD row; witchcraft upgrades are passive.
pub(crate) const COMBAT_WEAPON_BAR_COUNT: usize = OFFER_FAMILY_COUNT - 1;

/// Offer-pool families. Discriminants match historical family ids used by the HUD index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum UpgradeFamily {
    Moon = 0,
    Storm = 1,
    Grave = 2,
    Flare = 3,
    Wraith = 4,
    Harrow = 5,
    Aether = 6,
    Rift = 7,
    Echo = 8,
    Ward = 9,
    Witch = 10,
    Halo = 11,
    Void = 12,
    Solar = 13,
    Abyssal = 14,
    Starfall = 15,
    PhantomNet = 16,
    LuminousLance = 17,
    TemporalRift = 18,
    Scythe = 19,
    Gloom = 20,
    PulseLance = 21,
    ShardStorm = 22,
    PrismBolts = 23,
    Inferno = 24,
    Gravity = 25,
    SigilNet = 26,
}

impl UpgradeFamily {
    const ALL: [Self; OFFER_FAMILY_COUNT] = [
        Self::Moon,
        Self::Storm,
        Self::Grave,
        Self::Flare,
        Self::Wraith,
        Self::Harrow,
        Self::Aether,
        Self::Rift,
        Self::Echo,
        Self::Ward,
        Self::Witch,
        Self::Halo,
        Self::Void,
        Self::Solar,
        Self::Abyssal,
        Self::Starfall,
        Self::PhantomNet,
        Self::LuminousLance,
        Self::TemporalRift,
        Self::Scythe,
        Self::Gloom,
        Self::PulseLance,
        Self::ShardStorm,
        Self::PrismBolts,
        Self::Inferno,
        Self::Gravity,
        Self::SigilNet,
    ];
}

/// The complete mutable state of one run.
pub(crate) struct Game {
    phase: Phase,
    player: Player,
    moon: MoonKnives,
    storm: StormLantern,
    grave_mines: GraveMinefield,
    astral_flare: AstralFlare,
    wraith_lash: WraithLash,
    harrow_volley: HarrowVolley,
    aether_spears: AetherSpear,
    rift_pulse: RiftPulse,
    echo_cannon: EchoCannon,
    rune_wards: RuneWard,
    crescent_halo: CrescentHalo,
    void_bloom: VoidBloom,
    solar_nova: SolarNova,
    abyssal_mines: AbyssalMine,
    starfall: Starfall,
    phantom_net: PhantomNet,
    luminous_lance: LuminousLance,
    temporal_rift: TemporalRift,
    scythe_cyclone: ScytheCyclone,
    gloom_volley: GloomVolley,
    pulse_lance: PulseLance,
    shard_storm: ShardStorm,
    prism_bolts: PrismBolts,
    inferno_bombs: InfernoBomb,
    gravity_weave: GravityWeave,
    sigil_net: SigilNet,
    enemies: Vec<Enemy>,
    gems: Vec<Gem>,
    particles: Vec<Particle>,
    lightning: Vec<Lightning>,
    flare_pulses: Vec<FlarePulse>,
    wraith_traces: Vec<WraithTrace>,
    grave_shards: Vec<GraveShard>,
    harrow_darts: Vec<HarrowDart>,
    aether_projectiles: Vec<AetherSpearProjectile>,
    echo_shards: Vec<EchoShard>,
    void_seeds: Vec<RuneSeed>,
    abyssal_minelets: Vec<AbyssalMinelet>,
    starfall_meteors: Vec<StarfallMeteor>,
    phantom_tethers: Vec<PhantomTether>,
    luminous_lance_bolts: Vec<LuminousLanceBolt>,
    temporal_rift_wells: Vec<TemporalRiftWell>,
    gloom_shards: Vec<GloomShard>,
    shard_storm_shards: Vec<GloomShard>,
    prism_needles: Vec<PrismNeedle>,
    inferno_meteors: Vec<InfernoFireball>,
    gravity_wells: Vec<GravityWell>,
    sigil_threads: Vec<SigilThread>,
    rune_pulses: Vec<RunePulse>,
    rift_echoes: Vec<RiftEcho>,
    impact_rings: Vec<ImpactRing>,
    damage_numbers: Vec<DamageNumber>,
    offers: [Upgrade; 3],
    elapsed: f32,
    visual_time: f32,
    spawn_timer: f32,
    storm_timer: f32,
    moon_angle: f32,
    camera: Vec2,
    shake: f32,
    hit_stop: f32,
    pickup_flash: f32,
    kills: u32,
    phase_bucket: u8,
    phase_flash: f32,
}

impl Game {
    /// Creates a fresh run on its title screen.
    pub(crate) fn new() -> Self {
        Self::with_phase(Phase::Title)
    }

    /// Creates a fresh run that starts immediately.
    pub(crate) fn new_running() -> Self {
        Self::with_phase(Phase::Running)
    }

    fn with_phase(phase: Phase) -> Self {
        Self {
            phase,
            player: Player::new(),
            moon: MoonKnives::new(),
            storm: StormLantern::new(),
            grave_mines: GraveMinefield::new(),
            astral_flare: AstralFlare::new(),
            wraith_lash: WraithLash::new(),
            harrow_volley: HarrowVolley::new(),
            aether_spears: AetherSpear::new(),
            rift_pulse: RiftPulse::new(),
            echo_cannon: EchoCannon::new(),
            rune_wards: RuneWard::new(),
            crescent_halo: CrescentHalo::new(),
            void_bloom: VoidBloom::new(),
            solar_nova: SolarNova::new(),
            abyssal_mines: AbyssalMine::new(),
            starfall: Starfall::new(),
            phantom_net: PhantomNet::new(),
            luminous_lance: LuminousLance::new(),
            temporal_rift: TemporalRift::new(),
            scythe_cyclone: ScytheCyclone::new(),
            gloom_volley: GloomVolley::new(),
            pulse_lance: PulseLance::new(),
            shard_storm: ShardStorm::new(),
            prism_bolts: PrismBolts::new(),
            inferno_bombs: InfernoBomb::new(),
            gravity_weave: GravityWeave::new(),
            sigil_net: SigilNet::new(),
            enemies: Vec::with_capacity(256),
            gems: Vec::with_capacity(128),
            particles: Vec::with_capacity(512),
            lightning: Vec::with_capacity(8),
            flare_pulses: Vec::with_capacity(16),
            wraith_traces: Vec::with_capacity(8),
            grave_shards: Vec::with_capacity(16),
            harrow_darts: Vec::with_capacity(28),
            aether_projectiles: Vec::with_capacity(24),
            echo_shards: Vec::with_capacity(16),
            void_seeds: Vec::with_capacity(12),
            abyssal_minelets: Vec::with_capacity(16),
            starfall_meteors: Vec::with_capacity(16),
            phantom_tethers: Vec::with_capacity(12),
            luminous_lance_bolts: Vec::with_capacity(18),
            temporal_rift_wells: Vec::with_capacity(16),
            gloom_shards: Vec::with_capacity(24),
            shard_storm_shards: Vec::with_capacity(24),
            prism_needles: Vec::with_capacity(20),
            inferno_meteors: Vec::with_capacity(16),
            gravity_wells: Vec::with_capacity(14),
            sigil_threads: Vec::with_capacity(16),
            rune_pulses: Vec::with_capacity(16),
            rift_echoes: Vec::with_capacity(16),
            impact_rings: Vec::with_capacity(24),
            damage_numbers: Vec::with_capacity(32),
            offers: [Upgrade::ExtraKnife, Upgrade::FastStorm, Upgrade::Fleet],
            elapsed: 0.0,
            visual_time: 0.0,
            phase_bucket: phase_bucket(0.0),
            phase_flash: 0.0,
            spawn_timer: 0.05,
            storm_timer: 0.35,
            moon_angle: 0.0,
            camera: Vec2::ZERO,
            shake: 0.0,
            hit_stop: 0.0,
            pickup_flash: 0.0,
            kills: 0,
        }
    }

    /// Advances the simulation by one frame.
    pub(crate) fn update(&mut self, dt: f32, input: Input) -> Control {
        self.visual_time += dt;

        match self.phase {
            Phase::Title => {
                if input.accept || input.pointer.is_some() {
                    self.phase = Phase::Running;
                }
            },
            Phase::Paused => {
                if input.pause || input.accept {
                    self.phase = Phase::Running;
                }
            },
            Phase::LevelUp => {
                let choice = input
                    .choice
                    .or_else(|| input.pointer.and_then(render::upgrade_at));
                if let Some(choice) = choice.filter(|choice| *choice < self.offers.len()) {
                    self.apply_upgrade(self.offers[choice]);
                    if self.player.experience >= self.player.next_level {
                        self.begin_level_up();
                    } else {
                        self.phase = Phase::Running;
                    }
                }
            },
            Phase::GameOver => {
                if input.retry || input.accept || input.pointer.is_some() {
                    return Control::Restart;
                }
            },
            Phase::Running => {
                if input.pause {
                    self.phase = Phase::Paused;
                } else {
                    self.update_running(dt, input.movement);
                }
            },
        }

        Control::Continue
    }

    fn update_running(&mut self, dt: f32, movement: Vec2) {
        // Hit stop freezes gameplay but lets short-lived impact effects finish.
        if self.hit_stop > 0.0 {
            self.hit_stop = (self.hit_stop - dt).max(0.0);
            self.update_effects(dt);
            return;
        }

        self.elapsed += dt;
        self.update_phase_progress();
        self.shake = (self.shake - 24.0 * dt).max(0.0);
        self.pickup_flash = (self.pickup_flash - dt).max(0.0);
        self.player.invulnerability = (self.player.invulnerability - dt).max(0.0);
        self.moon_angle = (self.moon_angle + self.moon.speed * dt) % TAU;
        self.tick_weapon_timers(dt);

        self.update_player(dt, movement);
        self.spawn_enemies(dt);
        self.update_grave_shards(dt);
        self.update_harrow_darts(dt);
        self.update_aether_projectiles(dt);
        self.update_void_seeds(dt);
        self.update_abyssal_minelets(dt);
        self.update_echo_cannon(dt);
        self.update_rune_wards(dt);
        self.update_rift_echoes(dt);
        self.update_starfall_meteors(dt);
        self.update_phantom_tethers(dt);
        self.update_shard_storm_shards(dt);
        self.update_prism_needles(dt);
        self.update_inferno_meteors(dt);
        self.update_gravity_wells(dt);
        self.update_luminous_lance_bolts(dt);
        self.update_temporal_rift_wells(dt);
        self.update_sigil_threads(dt);
        self.update_gloom_shards(dt);
        self.update_enemies(dt);

        self.resolve_moon_knives();
        self.resolve_storm_lantern();
        self.resolve_astral_flare();
        self.resolve_wraith_lash();
        self.resolve_harrow_volley();
        self.resolve_grave_mines();
        self.resolve_aether_spears();
        self.resolve_rift_pulse();
        self.resolve_crescent_halo();
        self.resolve_void_bloom();
        self.resolve_solar_nova();
        self.resolve_abyssal_minelets();
        self.resolve_starfall();
        self.resolve_phantom_net();
        self.resolve_luminous_lance();
        self.resolve_temporal_rift();
        self.resolve_scythe_cyclone();
        self.resolve_pulse_lance();
        self.resolve_shard_storm();
        self.resolve_prism_bolts();
        self.resolve_inferno_bombs();
        self.resolve_gravity_weave();
        self.resolve_sigil_net();
        self.resolve_gloom_volley();
        self.resolve_echo_cannon();
        self.resolve_rune_wards();

        self.resolve_player_contact();
        self.remove_defeated_enemies();
        self.collect_gems(dt);
        self.update_effects(dt);

        let camera_weight = smoothing_weight(9.0, dt);
        self.camera = self.camera.lerp(self.player.position, camera_weight);

        if self.player.health <= 0.0 {
            self.player.health = 0.0;
            self.phase = Phase::GameOver;
            self.shake = 18.0;
        }
    }

    fn update_player(&mut self, dt: f32, movement: Vec2) {
        let target_velocity = movement * self.player.speed;
        let acceleration = smoothing_weight(18.0, dt);
        self.player.velocity = self.player.velocity.lerp(target_velocity, acceleration);
        self.player.position += self.player.velocity * dt;
        if movement.x.abs() > 0.1 {
            self.player.facing = movement.x.signum();
        }
    }

    fn tick_weapon_timers(&mut self, dt: f32) {
        self.spawn_timer -= dt;
        self.storm_timer -= dt;
        for timer in [
            &mut self.grave_mines.timer,
            &mut self.echo_cannon.timer,
            &mut self.astral_flare.timer,
            &mut self.wraith_lash.timer,
            &mut self.harrow_volley.timer,
            &mut self.aether_spears.timer,
            &mut self.rift_pulse.timer,
            &mut self.rune_wards.timer,
            &mut self.crescent_halo.timer,
            &mut self.void_bloom.timer,
            &mut self.solar_nova.timer,
            &mut self.abyssal_mines.timer,
            &mut self.starfall.timer,
            &mut self.phantom_net.timer,
            &mut self.luminous_lance.timer,
            &mut self.temporal_rift.timer,
            &mut self.scythe_cyclone.timer,
            &mut self.gloom_volley.timer,
            &mut self.pulse_lance.timer,
            &mut self.shard_storm.timer,
            &mut self.prism_bolts.timer,
            &mut self.inferno_bombs.timer,
            &mut self.gravity_weave.timer,
            &mut self.sigil_net.timer,
        ] {
            *timer = (*timer - dt).max(0.0);
        }
    }

    fn update_phase_progress(&mut self) {
        let bucket = phase_bucket(self.elapsed);
        if bucket != self.phase_bucket {
            self.phase_bucket = bucket;
            self.phase_flash = PHASE_FLASH_TIME;
        }
    }

    fn spawn_enemies(&mut self, _dt: f32) {
        let mut spawned = 0;
        let profile = night_profile(self.elapsed);
        let combat_pressures = self.combat_pressures();
        let timing_pressure = enemy_timing_pressure(self.elapsed);
        let timing_wave = enemy_timing_spawn_pressure(self.elapsed);
        let lane_pressure = enemy_spawn_direction_pressure(self.elapsed);
        let lane_center = enemy_spawn_lane_center(self.elapsed);
        let lane_span = enemy_spawn_lane_span(self.elapsed, lane_pressure);
        let ritual_window = is_ritual_window(self.elapsed);
        let ritual_multiplier = if ritual_window {
            RITUAL_BURST_MULTIPLIER
        } else {
            1.0
        };
        let enrage_multiplier = (1.0 + combat_pressures.enrage * (ENRAGE_SPAWN_MULTIPLIER - 1.0))
            .clamp(1.0, ENRAGE_SPAWN_MULTIPLIER);
        let wave_multiplier =
            1.0 + combat_pressures.wave * ENEMY_WAVE_SPAWN_BURST + combat_pressures.time_pulse;
        let tempo_multiplier = 1.0 + combat_pressures.tempo * 1.05;
        let beat_multiplier = 1.0 + combat_pressures.beat * (ENEMY_BEAT_PEAK - 1.0);
        let pulse_multiplier = 1.0 + combat_pressures.time_pulse * ENEMY_PULSE_SPAWN_BURST;
        let cluster_multiplier = 1.0 + combat_pressures.cluster * ENEMY_CLUSTER_SPAWN_BURST;
        let max_spawns = (6.0
            + combat_pressures.enrage * 5.0
            + combat_pressures.wave * 4.0
            + combat_pressures.tempo * 3.0
            + combat_pressures.time_pulse * 2.0
            + timing_wave * ENEMY_TIMING_SPAWN_BURST) as usize;
        // Catch up after a slow frame, but cap the work to prevent a spawn spiral.
        while self.spawn_timer <= 0.0 && spawned < max_spawns {
            self.spawn_timer += spawn_interval(self.elapsed)
                / (ritual_multiplier
                    * enrage_multiplier
                    * wave_multiplier
                    * tempo_multiplier
                    * beat_multiplier
                    * cluster_multiplier
                    * pulse_multiplier);
            spawned += 1;
            if self.enemies.len() >= MAX_ENEMIES {
                continue;
            }

            let roll = rand::gen_range(0.0, 1.0);
            let kind = spawn_kind(
                self.elapsed,
                roll,
                ritual_window,
                combat_pressures.enrage,
                combat_pressures.wave,
                combat_pressures.tempo,
                combat_pressures.beat,
                timing_pressure,
                combat_pressures.cluster,
                combat_pressures.time_pulse,
            );
            let spawn_funnel = enemy_spawn_funnel(self.elapsed);
            let spread = TAU / 7.5 * (0.35 + (1.0 - lane_pressure.clamp(0.0, 1.0)) * 1.8);
            let angle = if lane_pressure > 0.0 {
                lane_center + rand::gen_range(-lane_span, lane_span)
            } else if combat_pressures.cluster > 0.0 {
                spawn_funnel + rand::gen_range(-spread, spread)
            } else {
                rand::gen_range(0.0, TAU)
            };
            let spawn_radius = spawn_distance(view_width());
            let position = self.player.position + Vec2::from_angle(angle) * spawn_radius;
            let health_scale = 1.0 + self.elapsed / 140.0;
            let health_scale = health_scale * (1.0 + profile.health_scale);

            self.enemies.push(Enemy {
                kind,
                position,
                velocity: Vec2::ZERO,
                health: kind.base_health() * health_scale,
                radius: kind.radius(),
                flash: 0.0,
                moon_immunity: 0.0,
                phase: rand::gen_range(0.0, TAU),
            });
        }
    }

    fn update_enemies(&mut self, dt: f32) {
        let profile = night_profile(self.elapsed);
        let movement_timing = enemy_archetype_timing_factors(enemy_timing_pressure(self.elapsed));
        for enemy in &mut self.enemies {
            enemy.flash = (enemy.flash - dt).max(0.0);
            enemy.moon_immunity = (enemy.moon_immunity - dt).max(0.0);

            let direction = (self.player.position - enemy.position).normalize_or_zero();
            let kind_gate = enemy_kind_movement_gate(self.elapsed, enemy.kind);
            let weave = (self.elapsed * 5.0 + enemy.phase).sin() * 0.44 * kind_gate;
            let desired_direction = match enemy.kind {
                EnemyKind::Wisp => {
                    (direction + vec2(-direction.y, direction.x) * weave).normalize_or_zero()
                },
                EnemyKind::Howl => (direction + vec2(-direction.y, direction.x) * (weave * 1.35))
                    .normalize_or_zero(),
                EnemyKind::Shade | EnemyKind::Brute => direction,
                EnemyKind::Revenant => {
                    let frenzy = (self.elapsed * 1.55 + enemy.phase).sin() * 0.38 + 1.0;
                    (direction + vec2(-direction.y, direction.x) * weave * 0.35).normalize_or_zero()
                        * frenzy
                },
            };
            let mut desired_velocity = desired_direction * enemy.kind.speed() * profile.speed_scale;
            desired_velocity *=
                1.0 + (movement_timing[enemy.kind.timing_index()] - 1.0) * ENEMY_SPEED_TEMPO_SWAY;
            desired_velocity *= 1.0 + (kind_gate - 1.0) * ENEMY_KIND_MOVEMENT_SWAY;
            if matches!(enemy.kind, EnemyKind::Howl) {
                desired_velocity *= 0.68;
            }
            let steering = if matches!(enemy.kind, EnemyKind::Revenant) {
                smoothing_weight(4.5, dt)
            } else {
                smoothing_weight(8.0, dt)
            };

            enemy.velocity = enemy.velocity.lerp(desired_velocity, steering);
            enemy.position += enemy.velocity * dt;
        }

        Self::separate_enemies(&mut self.enemies, dt);
    }

    fn separate_enemies(enemies: &mut [Enemy], dt: f32) {
        // Split overlap correction equally between both enemies.
        let correction = smoothing_weight(ENEMY_SEPARATION_RATE, dt) * 0.5;
        for left_index in 0..enemies.len() {
            let (left, right) = enemies.split_at_mut(left_index + 1);
            let first = &mut left[left_index];
            for second in right {
                let offset = second.position - first.position;
                let distance_squared = offset.length_squared();
                let minimum = (first.radius + second.radius) * 0.72;
                if distance_squared > 0.01 && distance_squared < minimum * minimum {
                    let distance = distance_squared.sqrt();
                    let push = offset / distance * (minimum - distance);
                    first.position -= push * correction;
                    second.position += push * correction;
                }
            }
        }
    }

    fn resolve_moon_knives(&mut self) {
        let mut hit_positions = Vec::new();
        for enemy in &mut self.enemies {
            if enemy.moon_immunity > 0.0 {
                continue;
            }
            for index in 0..self.moon.count {
                let angle = self.moon_angle + index as f32 * TAU / self.moon.count as f32;
                let blade = self.player.position + Vec2::from_angle(angle) * self.moon.radius;
                let reach = enemy.radius + 17.0;
                if enemy.position.distance_squared(blade) < reach * reach {
                    enemy.health -= self.moon.damage;
                    enemy.flash = 0.08;
                    enemy.moon_immunity = 0.16;
                    enemy.velocity += (enemy.position - blade).normalize_or_zero() * 130.0;
                    hit_positions.push(enemy.position);
                    break;
                }
            }
        }

        if !hit_positions.is_empty() {
            self.shake = self.shake.max(4.0);
            for (index, position) in hit_positions.into_iter().enumerate() {
                self.damage_number(position, self.moon.damage, MOON_GOLD);
                self.impact_ring(position, self.moon.damage, MOON_GOLD);
                if index < 5 {
                    self.burst(position, MOON_GOLD, 5, 135.0);
                }
            }
        }
    }

    fn resolve_storm_lantern(&mut self) {
        if self.storm_timer > 0.0 {
            return;
        }

        let targets = chain_targets(
            &self.enemies,
            self.player.position,
            self.storm.range,
            self.storm.jumps,
        );
        if targets.is_empty() {
            self.storm_timer = 0.12;
            return;
        }

        let mut points = Vec::with_capacity(targets.len() + 1);
        points.push(self.player.lantern_position());
        let mut hits = Vec::with_capacity(targets.len());
        for (jump, index) in targets.into_iter().enumerate() {
            let enemy = &mut self.enemies[index];
            let falloff = 0.88_f32.powi(jump as i32);
            let damage = self.storm.damage * falloff;
            enemy.health -= damage;
            enemy.flash = 0.13;
            enemy.velocity += (enemy.position - self.player.position).normalize_or_zero() * 55.0;
            points.push(enemy.position);
            hits.push((enemy.position, damage));
        }

        self.lightning.push(Lightning { points, life: 0.18 });
        for (position, damage) in hits {
            self.damage_number(position, damage, STORM_CYAN);
            self.impact_ring(position, damage, STORM_CYAN);
            self.burst(position, STORM_CYAN, 4, 75.0);
        }
        self.storm_timer = self.storm.cooldown;
        self.shake = self.shake.max(5.5);
        self.hit_stop = 0.025;
    }

    fn resolve_grave_mines(&mut self) {
        if !self.grave_mines.unlocked() || self.grave_mines.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.grave_mines.timer = 0.2;
            return;
        }

        let angle_origin = rand::gen_range(0.0, TAU);
        for index in 0..self.grave_mines.shard_count {
            let angle = angle_origin + index as f32 * TAU / self.grave_mines.shard_count as f32;
            let velocity = Vec2::from_angle(angle) * rand::gen_range(44.0, 72.0);
            self.grave_shards.push(GraveShard {
                position: self.player.position + Vec2::from_angle(angle) * 12.0,
                velocity,
                timer: rand::gen_range(1.4, 2.45),
                arming: 0.18,
                damage: self.grave_mines.damage,
                blast_radius: self.grave_mines.blast_radius,
            });
        }
        self.grave_mines.timer = self.grave_mines.cooldown;
        self.shake = self.shake.max(4.5);
        self.burst(self.player.position, GRAVE_MARROW, 6, 95.0);
    }

    fn resolve_astral_flare(&mut self) {
        if !self.astral_flare.unlocked() || self.astral_flare.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.astral_flare.timer = 0.3;
            return;
        }

        let radius = self.astral_flare.radius + (self.astral_flare.level as f32 - 1.0) * 4.5;
        let hit_any = self.area_strike(
            self.player.position,
            radius,
            self.astral_flare.damage,
            55.0,
            CINDER_ORANGE,
        );
        self.flare_pulses.push(FlarePulse {
            position: self.player.position,
            life: 0.34,
            max_radius: radius,
        });
        self.astral_flare.timer = self.astral_flare.cooldown;

        if hit_any {
            self.shake = self.shake.max(6.0);
            self.hit_stop = 0.02;
            self.burst(self.player.position, CINDER_ORANGE, 7, 85.0);
            self.impact_ring(
                self.player.position,
                self.astral_flare.damage,
                CINDER_ORANGE,
            );
        }
    }

    fn resolve_wraith_lash(&mut self) {
        if !self.wraith_lash.unlocked() || self.wraith_lash.timer > 0.0 {
            return;
        }

        let Some(target_index) =
            self.nearest_enemy_index(self.player.position, Some(self.wraith_lash.range))
        else {
            self.wraith_lash.timer = 0.22;
            return;
        };
        let target = self.enemies[target_index].position;
        let mut direction = target - self.player.position;
        if direction.length_squared() < 0.001 {
            self.wraith_lash.timer = 0.22;
            return;
        }

        direction = direction.normalize_or_zero();
        let end = self.player.position + direction * self.wraith_lash.range;
        let mut strike_count = 0;
        let max_strikes = 1 + ((self.wraith_lash.level as f32 - 1.0) * 0.38) as usize;
        let trace_width =
            (self.wraith_lash.width + (self.wraith_lash.level as f32 - 1.0) * 2.8).max(4.5);
        let mut struck = Vec::new();

        for index in 0..self.enemies.len() {
            let enemy = &mut self.enemies[index];
            if enemy.health <= 0.0 {
                continue;
            }

            let hit = if enemy_to_line_distance(enemy.position, self.player.position, end)
                <= trace_width
            {
                enemy.health -= self.wraith_lash.damage;
                enemy.flash = 0.15;
                enemy.velocity +=
                    (enemy.position - self.player.position).normalize_or_zero() * 125.0;
                true
            } else {
                false
            };
            if hit {
                strike_count += 1;
                struck.push((enemy.position, self.wraith_lash.damage, index));
                if strike_count >= max_strikes {
                    break;
                }
            }
        }

        for (position, damage, _) in &struck {
            self.burst(*position, WRAITH_AMETHYST, 3, 95.0);
            self.impact_ring(*position, *damage, WRAITH_AMETHYST);
            self.damage_number(*position, *damage, WRAITH_AMETHYST);
        }
        if strike_count > 0 {
            self.shake = self.shake.max(5.0);
            self.hit_stop = 0.028;
            self.wraith_traces.push(WraithTrace {
                start: self.player.position,
                end,
                life: 0.12,
                width: trace_width,
            });
        }

        self.wraith_lash.timer = self.wraith_lash.cooldown;
    }

    fn resolve_harrow_volley(&mut self) {
        if !self.harrow_volley.unlocked() || self.harrow_volley.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.harrow_volley.timer = 0.2;
            return;
        }

        let count =
            (2 + self.harrow_volley.dart_count + self.harrow_volley.level as usize / 2).min(9);
        let base = (self.harrow_volley.damage + 5.0) + (self.harrow_volley.level as f32 * 1.7);
        for shot in 0..count {
            let angle = if let Some(target_position) = self
                .nearest_enemy_index(self.player.position, Some(self.harrow_volley.range))
                .and_then(|index| self.enemies.get(index).map(|enemy| enemy.position))
            {
                let vector = target_position - self.player.position;
                vector.y.atan2(vector.x) + (shot as f32) * 0.17
            } else {
                rand::gen_range(0.0, TAU)
            };
            let launch = Vec2::new(angle.cos(), angle.sin());
            self.harrow_darts.push(HarrowDart {
                position: self.player.position + launch * 15.0,
                velocity: launch * self.harrow_volley.speed,
                life: 1.0,
                damage: base + shot as f32 * 0.75,
                radius: 3.6,
            });
        }
        self.harrow_volley.timer = self.harrow_volley.cooldown;
        self.hit_stop = 0.02;
    }

    fn resolve_aether_spears(&mut self) {
        if !self.aether_spears.unlocked() || self.aether_spears.timer > 0.0 {
            return;
        }

        let Some(target_index) =
            self.nearest_enemy_index(self.player.position, Some(self.aether_spears.range))
        else {
            self.aether_spears.timer = 0.22;
            return;
        };

        let mut direction = self.enemies[target_index].position - self.player.position;
        if direction.length_squared() < 0.001 {
            direction = vec2(1.0, 0.0);
        }
        let base = direction.normalize_or_zero();
        let base_angle = base.y.atan2(base.x);
        let spread = self.aether_spears.spear_count.saturating_sub(1) as f32 * 0.12;

        for shot in 0..self.aether_spears.spear_count {
            let shot_ratio = if self.aether_spears.spear_count > 1 {
                shot as f32 / (self.aether_spears.spear_count as f32 - 1.0)
            } else {
                0.5
            };
            let angle = base_angle + (shot_ratio - 0.5) * spread;
            let velocity = Vec2::from_angle(angle) * self.aether_spears.speed;
            self.aether_projectiles.push(AetherSpearProjectile {
                position: self.player.position,
                velocity,
                life: self.aether_spears.range / self.aether_spears.speed,
                damage: self.aether_spears.damage,
                radius: 4.4,
                pierce: self.aether_spears.pierce,
                range: self.aether_spears.range,
            });
        }

        self.aether_spears.timer = self.aether_spears.cooldown;
        self.shake = self.shake.max(3.2);
    }

    fn resolve_rift_pulse(&mut self) {
        if !self.rift_pulse.unlocked() || self.rift_pulse.timer > 0.0 {
            return;
        }

        let Some(target_index) =
            self.nearest_enemy_index(self.player.position, Some(self.rift_pulse.range))
        else {
            self.rift_pulse.timer = 0.2;
            return;
        };

        let target = self.enemies[target_index].position;
        let echo_count = (1 + self.rift_pulse.level / 2).min(3);
        let lifetime = self.rift_pulse.duration;

        for index in 0..echo_count {
            let drift = (index as f32 - (echo_count as f32 - 1.0) * 0.5) * 13.0;
            let center = target
                + Vec2::from_angle(self.visual_time * 0.9 + index as f32 * 1.6) * drift.max(1.0);
            self.rift_echoes.push(RiftEcho {
                center,
                life: lifetime,
                max_life: lifetime,
                timer: 0.0,
                interval: self.rift_pulse.tick_interval,
                max_radius: self.rift_pulse.radius,
                damage: self.rift_pulse.damage,
                range_scale: 1.0 + index as f32 * 0.08,
            });
        }

        self.rift_pulse.timer = self.rift_pulse.cooldown;
        self.hit_stop = 0.01;
    }

    fn resolve_crescent_halo(&mut self) {
        if !self.crescent_halo.unlocked() || self.crescent_halo.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.crescent_halo.timer = 0.2;
            return;
        }

        let mut hits = Vec::new();
        let mut struck: Vec<usize> = Vec::with_capacity(12);
        let blades = (self.crescent_halo.blades + 1).min(12);
        let spin = self.visual_time * self.crescent_halo.rotation;
        let arc = TAU / blades as f32;
        let base = self.player.position;
        let width = self.crescent_halo.width + 0.6;
        let damage = self.crescent_halo.damage;
        for blade in 0..blades {
            let angle = spin + blade as f32 * arc;
            let end = base + Vec2::from_angle(angle) * self.crescent_halo.radius;
            let mut struck_this_blade = None;
            for enemy_index in 0..self.enemies.len() {
                if struck.contains(&enemy_index) {
                    continue;
                }
                let enemy = &mut self.enemies[enemy_index];
                if enemy.health <= 0.0 {
                    continue;
                }
                if enemy_to_line_distance(enemy.position, base, end)
                    <= enemy.radius + width + self.crescent_halo.level as f32 * 0.6
                {
                    enemy.health -= damage;
                    enemy.flash = 0.14;
                    enemy.velocity +=
                        (enemy.position - base).normalize_or_zero() * 165.0 * (1.0 + 0.09 * damage);
                    struck.push(enemy_index);
                    struck_this_blade = Some(enemy.position);
                    break;
                }
            }

            if let Some(position) = struck_this_blade {
                hits.push((position, damage));
            }
        }

        if hits.is_empty() {
            self.crescent_halo.timer = 0.22;
            return;
        }

        for (position, strike) in hits {
            self.burst(position, CRESCENT_AUREATE, 2, 95.0);
            self.impact_ring(position, strike, CRESCENT_AUREATE);
            self.damage_number(position, strike, CRESCENT_AUREATE);
        }
        self.crescent_halo.timer = self.crescent_halo.cooldown;
        self.shake = self.shake.max(4.6);
        self.hit_stop = 0.022;
    }

    fn resolve_void_bloom(&mut self) {
        if !self.void_bloom.unlocked() || self.void_bloom.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.void_bloom.timer = 0.28;
            return;
        }

        let seeds = (self.void_bloom.seed_count + 1).min(8);
        let angle_origin = rand::gen_range(0.0, TAU);
        for index in 0..seeds {
            let angle = angle_origin + index as f32 * TAU / seeds as f32;
            let spread = rand::gen_range(-0.15, 0.15);
            let launch = Vec2::from_angle(angle + spread) * rand::gen_range(32.0, 74.0);
            self.void_seeds.push(RuneSeed {
                position: self.player.position + Vec2::from_angle(angle) * 10.0,
                velocity: launch,
                timer: self.void_bloom.seed_life,
                arming: 0.14,
                damage: self.void_bloom.damage,
                blast_radius: self.void_bloom.blast_radius,
            });
        }
        self.void_bloom.timer = self.void_bloom.cooldown;
        self.shake = self.shake.max(3.5);
        self.hit_stop = 0.009;
    }

    fn resolve_solar_nova(&mut self) {
        if !self.solar_nova.unlocked() || self.solar_nova.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.solar_nova.timer = 0.28;
            return;
        }

        let max_radius = self.solar_nova.radius + (self.solar_nova.level as f32 - 1.0) * 6.0;
        let hit_any = self.area_strike(
            self.player.position,
            max_radius,
            self.solar_nova.damage,
            64.0,
            SOLAR_NOVA,
        );
        let rings = self.solar_nova.rings.max(1);
        for ring in 0..rings {
            let progress = 1.0 + ring as f32 / rings as f32 * 0.24;
            let lifetime = if ring == 0 {
                0.34
            } else {
                0.34 + ring as f32 * 0.03
            };
            self.flare_pulses.push(FlarePulse {
                position: self.player.position,
                life: lifetime,
                max_radius: max_radius * progress,
            });
        }

        self.solar_nova.timer = self.solar_nova.cooldown;
        if hit_any {
            self.shake = self.shake.max(6.0);
            self.hit_stop = 0.018;
            self.burst(self.player.position, SOLAR_NOVA, 7, 95.0);
            self.impact_ring(self.player.position, self.solar_nova.damage, SOLAR_NOVA);
        }
    }

    fn resolve_abyssal_minelets(&mut self) {
        if !self.abyssal_mines.unlocked() || self.abyssal_mines.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.abyssal_mines.timer = 0.26;
            return;
        }

        let angle_origin = rand::gen_range(0.0, TAU);
        for index in 0..self.abyssal_mines.mine_count {
            let angle = angle_origin + index as f32 * TAU / self.abyssal_mines.mine_count as f32;
            let velocity = Vec2::from_angle(angle) * rand::gen_range(42.0, 70.0);
            self.abyssal_minelets.push(AbyssalMinelet {
                position: self.player.position + Vec2::from_angle(angle) * 12.0,
                velocity,
                timer: rand::gen_range(1.45, 2.35),
                arming: 0.18,
                damage: self.abyssal_mines.damage,
                blast_radius: self.abyssal_mines.blast_radius,
            });
        }
        self.abyssal_mines.timer = self.abyssal_mines.cooldown;
        self.shake = self.shake.max(5.0);
        self.burst(self.player.position, ABYSSAL_MINE, 7, 100.0);
    }

    fn resolve_starfall(&mut self) {
        if !self.starfall.unlocked() || self.starfall.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.starfall.timer = 0.3;
            return;
        }

        let meteroids = (self.starfall.meteor_count + self.starfall.level as usize - 1).min(10);
        let angle_origin = rand::gen_range(0.0, TAU);
        for index in 0..meteroids {
            let angle = angle_origin + index as f32 * TAU / meteroids as f32;
            let target_index = if let Some(index) = self.nearest_enemy_index(
                self.player.position + Vec2::from_angle(angle) * 180.0,
                Some(self.starfall.blast_radius * 9.0),
            ) {
                index
            } else {
                self.nearest_enemy_index(self.player.position, None)
                    .unwrap_or(0)
            };
            let target = self.enemies[target_index].position;
            let spawn = self.player.position
                + Vec2::from_angle(angle + 0.45) * 420.0
                + vec2(0.0, rand::gen_range(-60.0, -210.0));
            let mut velocity = (target - spawn).normalize_or_zero() * self.starfall.speed;
            if !velocity.is_finite() {
                velocity = Vec2::from_angle(angle) * self.starfall.speed;
            }
            self.starfall_meteors.push(StarfallMeteor {
                position: spawn,
                velocity,
                timer: self.starfall.speed / 120.0 + 1.2,
                arming: 0.18,
                damage: self.starfall.damage * (1.0 + (index as f32 * 0.08)),
                blast_radius: self.starfall.blast_radius + self.starfall.level as f32 * 2.0,
            });
        }
        self.starfall.timer = self.starfall.cooldown;
        self.shake = self.shake.max(3.8);
    }

    fn resolve_phantom_net(&mut self) {
        if !self.phantom_net.unlocked() || self.phantom_net.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.phantom_net.timer = 0.24;
            return;
        }

        let threads =
            (self.phantom_net.thread_count + (self.phantom_net.level as usize - 1)).min(16);
        let angle_origin = rand::gen_range(0.0, TAU);
        let orbit = self.phantom_net.orbit_radius + self.phantom_net.level as f32 * 2.2;
        let angle_speed = 0.72 + self.phantom_net.level as f32 * 0.12;
        for index in 0..threads {
            let angle = angle_origin + index as f32 * TAU / threads as f32;
            self.phantom_tethers.push(PhantomTether {
                angle,
                orbit_radius: orbit,
                angular_speed: angle_speed + rand::gen_range(-0.22, 0.22),
                timer: 0.96 + self.phantom_net.level as f32 * 0.08,
                arming: 0.16,
                damage: self.phantom_net.damage,
                blast_radius: self.phantom_net.blast_radius,
            });
        }
        self.phantom_net.timer = self.phantom_net.cooldown;
        self.shake = self.shake.max(2.7);
        self.hit_stop = 0.006;
    }

    fn resolve_luminous_lance(&mut self) {
        if !self.luminous_lance.unlocked() || self.luminous_lance.timer > 0.0 {
            return;
        }

        let Some(target_index) =
            self.nearest_enemy_index(self.player.position, Some(self.luminous_lance.range))
        else {
            self.luminous_lance.timer = 0.2;
            return;
        };

        let mut direction = self.enemies[target_index].position - self.player.position;
        if direction.length_squared() < 0.001 {
            direction = vec2(1.0, 0.0);
        }
        let base = direction.normalize_or_zero();
        let base_angle = base.y.atan2(base.x);
        let bolt_count =
            (self.luminous_lance.spear_count + self.luminous_lance.level as usize - 1).min(8);
        let spread = if bolt_count > 1 {
            0.24 + (self.luminous_lance.level as f32 - 1.0) * 0.04
        } else {
            0.0
        };

        for shot in 0..bolt_count {
            let ratio = if bolt_count > 1 {
                shot as f32 / (bolt_count as f32 - 1.0)
            } else {
                0.5
            };
            let angle = base_angle + (ratio - 0.5) * spread;
            let velocity = Vec2::from_angle(angle) * self.luminous_lance.speed;
            self.luminous_lance_bolts.push(LuminousLanceBolt {
                position: self.player.position,
                velocity,
                life: self.luminous_lance.range / self.luminous_lance.speed,
                damage: self.luminous_lance.damage,
                radius: 4.6,
                range: self.luminous_lance.range,
                pierce: self.luminous_lance.pierce,
            });
        }
        self.luminous_lance.timer = self.luminous_lance.cooldown;
        self.shake = self.shake.max(3.4);
    }

    fn resolve_temporal_rift(&mut self) {
        if !self.temporal_rift.unlocked() || self.temporal_rift.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.temporal_rift.timer = 0.24;
            return;
        }

        let gates = (self.temporal_rift.gate_count + self.temporal_rift.level as usize - 1).min(6);
        let angle_origin = rand::gen_range(0.0, TAU);
        for index in 0..gates {
            let angle = angle_origin + index as f32 * TAU / gates as f32;
            let radius = 90.0 + self.temporal_rift.level as f32 * 12.0;
            let center = self.player.position
                + Vec2::from_angle(angle + self.visual_time * 0.12) * radius
                + vec2(rand::gen_range(-12.0, 12.0), rand::gen_range(-12.0, 12.0));
            self.temporal_rift_wells.push(TemporalRiftWell {
                center,
                life: self.temporal_rift.duration,
                max_life: self.temporal_rift.duration,
                timer: 0.0,
                interval: (self.temporal_rift.interval
                    * (1.0 - (self.temporal_rift.level as f32 - 1.0) * 0.08))
                    .max(0.06),
                max_radius: self.temporal_rift.radius + self.temporal_rift.level as f32 * 3.0,
                damage: self.temporal_rift.damage,
                pull: self.temporal_rift.pull,
            });
        }
        self.temporal_rift.timer = self.temporal_rift.cooldown;
        self.hit_stop = 0.006;
    }

    fn resolve_scythe_cyclone(&mut self) {
        if !self.scythe_cyclone.unlocked() || self.scythe_cyclone.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.scythe_cyclone.timer = 0.24;
            return;
        }

        let mut struck: Vec<usize> = Vec::new();
        let mut hits = Vec::new();
        let blades =
            (self.scythe_cyclone.blades + (self.scythe_cyclone.level as usize - 1)).min(14);
        let spin = self.visual_time * 1.85 + self.player.experience as f32 * 0.003;
        let arc = TAU / blades as f32;
        let radius = self.scythe_cyclone.radius + self.scythe_cyclone.level as f32 * 3.8;
        let width = (self.scythe_cyclone.width + self.scythe_cyclone.level as f32 * 0.4).max(2.8);
        let damage = self.scythe_cyclone.damage + self.scythe_cyclone.level as f32 * 1.7;

        for blade in 0..blades {
            let angle = spin + blade as f32 * arc;
            let end = self.player.position + Vec2::from_angle(angle) * radius;
            for enemy_index in 0..self.enemies.len() {
                if struck.contains(&enemy_index) {
                    continue;
                }
                let enemy = &mut self.enemies[enemy_index];
                if enemy.health <= 0.0 {
                    continue;
                }
                if enemy_to_line_distance(enemy.position, self.player.position, end)
                    <= enemy.radius + width
                {
                    enemy.health -= damage;
                    enemy.flash = 0.14;
                    enemy.velocity +=
                        (enemy.position - self.player.position).normalize_or_zero() * 110.0;
                    struck.push(enemy_index);
                    hits.push((enemy.position, damage));
                    break;
                }
            }
        }

        if hits.is_empty() {
            self.scythe_cyclone.timer = 0.22;
            return;
        }

        for (position, damage) in hits {
            self.damage_number(position, damage, SCYTHE_IRON);
            self.impact_ring(position, damage, SCYTHE_IRON);
            self.burst(position, SCYTHE_IRON, 2, 85.0);
        }
        self.scythe_cyclone.timer = self.scythe_cyclone.cooldown;
        self.shake = self.shake.max(4.5);
        self.hit_stop = 0.018;
    }

    fn resolve_gloom_volley(&mut self) {
        if !self.gloom_volley.unlocked() || self.gloom_volley.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.gloom_volley.timer = 0.2;
            return;
        }

        let shards =
            (self.gloom_volley.shard_count + (self.gloom_volley.level as usize - 1)).min(12);
        let base = if let Some(target) = self.nearest_enemy_index(self.player.position, None) {
            (self.enemies[target].position - self.player.position).normalize_or_zero()
        } else {
            vec2(1.0, 0.0)
        };
        let base_angle = base.y.atan2(base.x);
        let spread = if shards > 1 {
            0.5 + self.gloom_volley.level as f32 * 0.07
        } else {
            0.0
        };
        for shot in 0..shards {
            let ratio = if shards > 1 {
                shot as f32 / (shards as f32 - 1.0)
            } else {
                0.5
            };
            let angle = base_angle + (ratio - 0.5) * spread;
            let velocity = Vec2::from_angle(angle)
                * self.gloom_volley.speed
                * (1.0 + rand::gen_range(-0.08, 0.12));
            self.gloom_shards.push(GloomShard {
                position: self.player.position,
                velocity,
                life: 1.0 + self.gloom_volley.level as f32 * 0.17,
                damage: self.gloom_volley.damage * (1.0 + self.gloom_volley.level as f32 * 0.04),
                radius: 3.8 + self.gloom_volley.level as f32 * 0.05,
                pierce: self.gloom_volley.piercing,
                range: self.gloom_volley.range,
            });
        }
        self.gloom_volley.timer = self.gloom_volley.cooldown;
        self.shake = self.shake.max(3.2);
    }

    fn resolve_pulse_lance(&mut self) {
        if !self.pulse_lance.unlocked() || self.pulse_lance.timer > 0.0 {
            return;
        }
        if self.enemies.is_empty() {
            self.pulse_lance.timer = 0.2;
            return;
        }

        let max_chain = (self.pulse_lance.chain_count + self.pulse_lance.level as usize).min(12);
        let targets = chain_targets(
            &self.enemies,
            self.player.position,
            self.pulse_lance.range,
            max_chain + 1,
        );
        if targets.is_empty() {
            self.pulse_lance.timer = 0.2;
            return;
        }

        let mut total_hits = Vec::new();
        let mut origin = self.player.position;
        for (index, enemy_index) in targets.iter().copied().enumerate() {
            if enemy_index >= self.enemies.len() || self.enemies[enemy_index].health <= 0.0 {
                continue;
            }
            let multiplier = if index == 0 {
                1.0
            } else {
                (0.86_f32).powi(index as i32)
            };
            let enemy = &mut self.enemies[enemy_index];
            let base_damage = self.pulse_lance.damage * multiplier;
            enemy.health -= base_damage;
            enemy.flash = 0.14;
            enemy.velocity += (enemy.position - origin).normalize_or_zero() * 160.0;
            total_hits.push((enemy.position, base_damage));
            origin = enemy.position;
        }

        if total_hits.is_empty() {
            self.pulse_lance.timer = 0.22;
            return;
        }
        for (position, damage) in total_hits {
            self.damage_number(position, damage, ARCANE_VIOLET);
            self.impact_ring(position, damage, ARCANE_VIOLET);
            self.burst(position, ARCANE_VIOLET, 2, 95.0);
        }
        self.pulse_lance.timer = self.pulse_lance.cooldown;
        self.shake = self.shake.max(4.3);
    }

    fn resolve_shard_storm(&mut self) {
        if !self.shard_storm.unlocked() || self.shard_storm.timer > 0.0 {
            return;
        }
        if self.enemies.is_empty() {
            self.shard_storm.timer = 0.2;
            return;
        }

        let shards = (self.shard_storm.shard_count + (self.shard_storm.level as usize - 1)).min(12);
        let base = if let Some(target) = self.nearest_enemy_index(self.player.position, None) {
            (self.enemies[target].position - self.player.position).normalize_or_zero()
        } else {
            vec2(1.0, 0.0)
        };
        let base_angle = base.y.atan2(base.x);
        let spread = if shards > 1 {
            0.55 + self.shard_storm.level as f32 * 0.08
        } else {
            0.0
        };
        for shot in 0..shards {
            let ratio = if shards > 1 {
                shot as f32 / (shards as f32 - 1.0)
            } else {
                0.5
            };
            let angle = base_angle + (ratio - 0.5) * spread;
            let velocity = Vec2::from_angle(angle)
                * self.shard_storm.speed
                * (1.0 + rand::gen_range(-0.08, 0.11));
            self.shard_storm_shards.push(GloomShard {
                position: self.player.position,
                velocity,
                life: 1.0 + self.shard_storm.level as f32 * 0.13,
                damage: self.shard_storm.damage * (1.0 + self.shard_storm.level as f32 * 0.04),
                radius: 3.6 + self.shard_storm.level as f32 * 0.05,
                pierce: self.shard_storm.pierce,
                range: self.shard_storm.range,
            });
        }
        self.shard_storm.timer = self.shard_storm.cooldown;
        self.shake = self.shake.max(3.1);
    }

    fn resolve_prism_bolts(&mut self) {
        if !self.prism_bolts.unlocked() || self.prism_bolts.timer > 0.0 {
            return;
        }
        if self.enemies.is_empty() {
            self.prism_bolts.timer = 0.22;
            return;
        }

        let Some(target_index) =
            self.nearest_enemy_index(self.player.position, Some(self.prism_bolts.range))
        else {
            self.prism_bolts.timer = 0.22;
            return;
        };

        let mut direction = self.enemies[target_index].position - self.player.position;
        if direction.length_squared() < 0.001 {
            direction = vec2(1.0, 0.0);
        }
        let base = direction.normalize_or_zero();
        let base_angle = base.y.atan2(base.x);
        let bolts = (self.prism_bolts.shot_count + (self.prism_bolts.level as usize - 1)).min(12);
        let spread = if bolts > 1 {
            0.2 + self.prism_bolts.level as f32 * 0.06
        } else {
            0.0
        };

        for shot in 0..bolts {
            let ratio = if bolts > 1 {
                shot as f32 / (bolts as f32 - 1.0)
            } else {
                0.5
            };
            let angle = base_angle + (ratio - 0.5) * spread;
            let velocity = Vec2::from_angle(angle) * self.prism_bolts.speed;
            self.prism_needles.push(PrismNeedle {
                position: self.player.position,
                velocity,
                life: self.prism_bolts.range / self.prism_bolts.speed,
                damage: self.prism_bolts.damage,
                radius: 4.4,
                pierce: self.prism_bolts.pierce,
                range: self.prism_bolts.range,
                homing: self.prism_bolts.homing,
                speed: self.prism_bolts.speed,
            });
        }
        self.prism_bolts.timer = self.prism_bolts.cooldown;
        self.shake = self.shake.max(3.3);
    }

    fn resolve_inferno_bombs(&mut self) {
        if !self.inferno_bombs.unlocked() || self.inferno_bombs.timer > 0.0 {
            return;
        }
        if self.enemies.is_empty() {
            self.inferno_bombs.timer = 0.3;
            return;
        }

        let meteor_count =
            (self.inferno_bombs.meteor_count + (self.inferno_bombs.level as usize - 1)).min(12);
        let angle_origin = rand::gen_range(0.0, TAU);
        for index in 0..meteor_count {
            let angle = angle_origin + index as f32 * TAU / meteor_count as f32;
            let target_index = if let Some(target) = self.nearest_enemy_index(
                self.player.position + Vec2::from_angle(angle) * 180.0,
                Some(self.inferno_bombs.blast_radius * 9.0),
            ) {
                target
            } else {
                self.nearest_enemy_index(self.player.position, None)
                    .unwrap_or(0)
            };
            let target = self.enemies[target_index].position;
            let spawn = self.player.position
                + Vec2::from_angle(angle) * 420.0
                + vec2(0.0, rand::gen_range(-70.0, -250.0));
            let mut velocity = (target - spawn).normalize_or_zero() * self.inferno_bombs.speed;
            if !velocity.is_finite() {
                velocity = Vec2::from_angle(angle) * self.inferno_bombs.speed;
            }
            self.inferno_meteors.push(InfernoFireball {
                position: spawn,
                velocity,
                timer: self.inferno_bombs.speed / 130.0 + 1.2,
                arming: 0.16,
                damage: self.inferno_bombs.damage
                    * (1.0 + (index as f32 * 0.07) + self.inferno_bombs.level as f32 * 0.02),
                blast_radius: self.inferno_bombs.blast_radius
                    + self.inferno_bombs.level as f32 * 2.0,
                gravity: self.inferno_bombs.gravity,
            });
        }
        self.inferno_bombs.timer = self.inferno_bombs.cooldown;
        self.shake = self.shake.max(3.8);
    }

    fn resolve_gravity_weave(&mut self) {
        if !self.gravity_weave.unlocked() || self.gravity_weave.timer > 0.0 {
            return;
        }
        if self.enemies.is_empty() {
            self.gravity_weave.timer = 0.24;
            return;
        }

        let wells =
            (self.gravity_weave.well_count + (self.gravity_weave.level as usize - 1)).min(12);
        let angle_origin = rand::gen_range(0.0, TAU);
        for index in 0..wells {
            let angle = angle_origin + index as f32 * TAU / wells as f32;
            let orbit = 90.0 + self.gravity_weave.level as f32 * 12.0;
            let center = self.player.position
                + Vec2::from_angle(angle + self.visual_time * 0.2) * orbit
                + vec2(rand::gen_range(-12.0, 12.0), rand::gen_range(-12.0, 12.0));
            self.gravity_wells.push(GravityWell {
                center,
                life: self.gravity_weave.duration,
                max_life: self.gravity_weave.duration,
                timer: 0.0,
                interval: (self.gravity_weave.interval
                    * (1.0 - (self.gravity_weave.level as f32 - 1.0) * 0.08))
                    .max(0.06),
                max_radius: self.gravity_weave.max_radius + self.gravity_weave.level as f32 * 3.0,
                damage: self.gravity_weave.damage,
                pull: self.gravity_weave.pull,
            });
        }
        self.gravity_weave.timer = self.gravity_weave.cooldown;
        self.hit_stop = 0.006;
    }

    fn resolve_sigil_net(&mut self) {
        if !self.sigil_net.unlocked() || self.sigil_net.timer > 0.0 {
            return;
        }
        if self.enemies.is_empty() {
            self.sigil_net.timer = 0.24;
            return;
        }

        let threads = (self.sigil_net.thread_count + (self.sigil_net.level as usize - 1)).min(16);
        let angle_origin = rand::gen_range(0.0, TAU);
        let orbit = self.sigil_net.orbit_radius + self.sigil_net.level as f32 * 2.2;
        let angle_speed = self.sigil_net.speed + rand::gen_range(-0.25, 0.25);
        for index in 0..threads {
            let angle = angle_origin + index as f32 * TAU / threads as f32;
            self.sigil_threads.push(SigilThread {
                angle,
                orbit_radius: orbit,
                angular_speed: angle_speed,
                timer: 1.0 + self.sigil_net.level as f32 * 0.15,
                arming: 0.18,
                damage: self.sigil_net.damage,
                blast_radius: self.sigil_net.blast_radius,
            });
        }
        self.sigil_net.timer = self.sigil_net.cooldown;
        self.shake = self.shake.max(2.9);
        self.hit_stop = 0.006;
    }

    fn update_void_seeds(&mut self, dt: f32) {
        let seek_range = self.void_bloom.blast_radius * 5.0;
        let homing = self.void_bloom.homing;
        let mut charges = std::mem::take(&mut self.void_seeds);
        let detonations = self.advance_armed_charges(
            &mut charges,
            dt,
            ChargeMotion::homing(seek_range, homing, 12.0),
        );
        self.void_seeds = charges;
        self.detonate_charges(&detonations, 150.0, VOID_BLOOM, 5, 75.0, 0.0);
    }

    fn update_abyssal_minelets(&mut self, dt: f32) {
        let seek_range = self.abyssal_mines.blast_radius * 4.0;
        let homing = self.abyssal_mines.homing;
        let mut charges = std::mem::take(&mut self.abyssal_minelets);
        let detonations = self.advance_armed_charges(
            &mut charges,
            dt,
            ChargeMotion::homing(seek_range, homing, 10.0),
        );
        self.abyssal_minelets = charges;
        self.detonate_charges(&detonations, 130.0, ABYSSAL_MINE, 5, 90.0, 0.0);
    }

    fn update_starfall_meteors(&mut self, dt: f32) {
        let gravity = self.starfall.gravity;
        let mut charges = std::mem::take(&mut self.starfall_meteors);
        let detonations =
            self.advance_armed_charges(&mut charges, dt, ChargeMotion::falling(gravity, 10.0));
        self.starfall_meteors = charges;
        self.detonate_charges(&detonations, 120.0, STARFALL_GOLD, 4, 84.0, 4.2);
    }

    fn update_phantom_tethers(&mut self, dt: f32) {
        if self.phantom_tethers.is_empty() {
            return;
        }

        let mut detonations = Vec::new();
        let mut index = self.phantom_tethers.len();
        while index > 0 {
            index -= 1;
            let mut tether = self.phantom_tethers[index];

            tether.timer -= dt;
            tether.arming = (tether.arming - dt).max(0.0);
            tether.angle = (tether.angle + tether.angular_speed * dt) % TAU;

            let position =
                self.player.position + Vec2::from_angle(tether.angle) * tether.orbit_radius;
            let should_detonate = tether.timer <= 0.0
                || (tether.arming <= 0.0
                    && self
                        .nearest_enemy_index(position, Some(tether.blast_radius + 8.0))
                        .is_some());
            if should_detonate {
                detonations.push((position, tether.damage, tether.blast_radius));
                self.phantom_tethers.swap_remove(index);
                continue;
            }

            self.phantom_tethers[index] = tether;
        }

        for (position, damage, blast_radius) in detonations {
            let hit_any = self.area_strike(position, blast_radius, damage, 86.0, PHANTOM_NET_GREEN);
            if hit_any {
                self.burst(position, PHANTOM_NET_GREEN, 3, 74.0);
                self.impact_ring(position, blast_radius, PHANTOM_NET_GREEN);
                self.shake = self.shake.max(2.8);
            }
        }
    }

    fn update_luminous_lance_bolts(&mut self, dt: f32) {
        if self.luminous_lance_bolts.is_empty() {
            return;
        }

        let mut impact_events = Vec::new();
        let mut index = self.luminous_lance_bolts.len();
        while index > 0 {
            index -= 1;
            let mut bolt = self.luminous_lance_bolts[index];

            bolt.life -= dt;
            bolt.range -= bolt.velocity.length() * dt;
            if bolt.life <= 0.0 || bolt.range <= 0.0 {
                self.luminous_lance_bolts.swap_remove(index);
                continue;
            }

            if let Some(target) =
                self.nearest_enemy_index(bolt.position, Some(self.luminous_lance.range * 0.95))
            {
                let seek = (self.enemies[target].position - bolt.position).normalize_or_zero();
                bolt.velocity = bolt
                    .velocity
                    .lerp(seek * self.luminous_lance.speed, self.luminous_lance.homing);
            } else {
                bolt.velocity *= 0.998;
            }
            bolt.position += bolt.velocity * dt;

            let mut impact = None;
            for enemy_index in 0..self.enemies.len() {
                if self.enemies[enemy_index].health <= 0.0 {
                    continue;
                }
                let strike_radius = self.enemies[enemy_index].radius + bolt.radius;
                if bolt
                    .position
                    .distance_squared(self.enemies[enemy_index].position)
                    < strike_radius * strike_radius
                {
                    impact = Some(enemy_index);
                    break;
                }
            }

            let Some(enemy_index) = impact else {
                self.luminous_lance_bolts[index] = bolt;
                continue;
            };

            impact_events.push((enemy_index, bolt.position, bolt.damage));
            if bolt.pierce == 0 {
                self.luminous_lance_bolts.swap_remove(index);
                continue;
            }
            bolt.pierce = bolt.pierce.saturating_sub(1);
            self.luminous_lance_bolts[index] = bolt;
        }

        for &(enemy_index, position, damage) in &impact_events {
            let mut dead = false;
            let impact_position = if let Some(enemy) = self.enemies.get_mut(enemy_index) {
                enemy.health -= damage;
                enemy.flash = 0.12;
                enemy.velocity +=
                    (enemy.position - self.player.position).normalize_or_zero() * 105.0;
                dead = enemy.health <= 0.0;
                enemy.position
            } else {
                position
            };
            self.damage_number(impact_position, damage, LANCE_AZURE);
            self.impact_ring(impact_position, damage, LANCE_AZURE);
            self.burst(position, LANCE_AZURE, 2, 72.0);
            if dead {
                self.shake = self.shake.max(1.7);
            }
        }
        if !impact_events.is_empty() {
            self.shake = self.shake.max(3.4);
        }
    }

    fn update_temporal_rift_wells(&mut self, dt: f32) {
        if self.temporal_rift_wells.is_empty() {
            return;
        }

        let mut impacts = Vec::new();
        let mut index = self.temporal_rift_wells.len();
        while index > 0 {
            index -= 1;
            let mut well = self.temporal_rift_wells[index];
            well.life -= dt;
            well.timer = (well.timer - dt).max(0.0);

            let radius = well.max_radius
                * (0.18 + 0.82 * (1.0 - (well.life / well.max_life).clamp(0.0, 1.0)));
            for enemy in &mut self.enemies {
                if enemy.health <= 0.0 {
                    continue;
                }
                let to_center = well.center - enemy.position;
                let distance_sq = to_center.length_squared();
                if distance_sq <= radius * radius {
                    let distance = distance_sq.sqrt();
                    let pull = (1.0 - (distance / radius).min(1.0)) * well.pull;
                    enemy.velocity += to_center.normalize_or_zero() * pull * 1.35;
                }
            }

            if well.timer <= 0.0 {
                let radius_sq = radius * radius;
                for enemy in self.enemies.iter_mut().filter(|enemy| {
                    enemy.health > 0.0 && enemy.position.distance_squared(well.center) <= radius_sq
                }) {
                    let enemy_position = enemy.position;
                    enemy.health -= well.damage;
                    enemy.flash = 0.12;
                    enemy.velocity +=
                        (well.center - enemy.position).normalize_or_zero() * well.pull;
                    impacts.push((
                        enemy_position,
                        well.damage,
                        self.player.position.distance(enemy_position),
                    ));
                }
                well.timer = well.interval;
                self.temporal_rift_wells[index] = well;
            }

            if well.life <= 0.0 {
                self.temporal_rift_wells.swap_remove(index);
            } else {
                self.temporal_rift_wells[index] = well;
            }
        }

        for &(position, damage, distance) in &impacts {
            self.damage_number(position, damage, RIFT_CYAN);
            if distance <= 5.0 {
                self.impact_ring(position, damage * 0.5, RIFT_CYAN);
            } else {
                self.impact_ring(position, damage, RIFT_CYAN);
            }
        }
        if !impacts.is_empty() {
            self.shake = self.shake.max(4.8);
            self.burst(self.player.position, RIFT_CYAN, 3, 60.0);
        }
    }

    fn update_gloom_shards(&mut self, dt: f32) {
        if self.gloom_shards.is_empty() {
            return;
        }

        let mut impact_events = Vec::new();
        let mut index = self.gloom_shards.len();
        while index > 0 {
            index -= 1;
            let mut shard = self.gloom_shards[index];

            shard.life -= dt;
            shard.range -= shard.velocity.length() * dt;
            if shard.life <= 0.0 || shard.range <= 0.0 {
                self.gloom_shards.swap_remove(index);
                continue;
            }

            if let Some(target) =
                self.nearest_enemy_index(shard.position, Some(self.gloom_volley.range * 0.95))
            {
                let seek = (self.enemies[target].position - shard.position).normalize_or_zero();
                shard.velocity = shard.velocity.lerp(seek * self.gloom_volley.speed, 0.11);
            }
            shard.position += shard.velocity * dt;

            let mut impact = None;
            for enemy_index in 0..self.enemies.len() {
                if self.enemies[enemy_index].health <= 0.0 {
                    continue;
                }
                let strike_radius = self.enemies[enemy_index].radius + shard.radius;
                if shard
                    .position
                    .distance_squared(self.enemies[enemy_index].position)
                    < strike_radius * strike_radius
                {
                    impact = Some(enemy_index);
                    break;
                }
            }

            let Some(enemy_index) = impact else {
                self.gloom_shards[index] = shard;
                continue;
            };

            impact_events.push((enemy_index, shard.position, shard.damage));
            if shard.pierce == 0 {
                self.gloom_shards.swap_remove(index);
                continue;
            }
            shard.pierce = shard.pierce.saturating_sub(1);
            self.gloom_shards[index] = shard;
        }

        for &(enemy_index, position, damage) in &impact_events {
            let Some(enemy) = self.enemies.get_mut(enemy_index) else {
                continue;
            };
            let enemy_position = enemy.position;
            enemy.health -= damage;
            enemy.flash = 0.12;
            enemy.velocity += (enemy_position - self.player.position).normalize_or_zero() * 88.0;
            self.damage_number(enemy_position, damage, GLOOM_PURPLE);
            self.impact_ring(enemy_position, damage, GLOOM_PURPLE);
            self.burst(position, GLOOM_PURPLE, 2, 66.0);
            self.shake = self.shake.max(2.8);
        }
    }

    fn update_shard_storm_shards(&mut self, dt: f32) {
        if self.shard_storm_shards.is_empty() {
            return;
        }

        let mut impact_events = Vec::new();
        let mut index = self.shard_storm_shards.len();
        while index > 0 {
            index -= 1;
            let mut shard = self.shard_storm_shards[index];

            shard.life -= dt;
            shard.range -= shard.velocity.length() * dt;
            if shard.life <= 0.0 || shard.range <= 0.0 {
                self.shard_storm_shards.swap_remove(index);
                continue;
            }

            if let Some(target) =
                self.nearest_enemy_index(shard.position, Some(self.shard_storm.range * 0.95))
            {
                let seek = (self.enemies[target].position - shard.position).normalize_or_zero();
                shard.velocity = shard.velocity.lerp(
                    seek * self.shard_storm.speed,
                    0.11 + self.shard_storm.level as f32 * 0.02,
                );
            }
            shard.position += shard.velocity * dt;

            let mut impact = None;
            for enemy_index in 0..self.enemies.len() {
                if self.enemies[enemy_index].health <= 0.0 {
                    continue;
                }
                let strike_radius = self.enemies[enemy_index].radius
                    + shard.radius
                    + self.shard_storm.level as f32 * 0.1;
                if shard
                    .position
                    .distance_squared(self.enemies[enemy_index].position)
                    < strike_radius * strike_radius
                {
                    impact = Some(enemy_index);
                    break;
                }
            }

            let Some(enemy_index) = impact else {
                self.shard_storm_shards[index] = shard;
                continue;
            };

            impact_events.push((enemy_index, shard.position, shard.damage));
            if shard.pierce == 0 {
                self.shard_storm_shards.swap_remove(index);
                continue;
            }
            shard.pierce = shard.pierce.saturating_sub(1);
            self.shard_storm_shards[index] = shard;
        }

        for &(enemy_index, position, damage) in &impact_events {
            let Some(enemy) = self.enemies.get_mut(enemy_index) else {
                continue;
            };
            let enemy_position = enemy.position;
            enemy.health -= damage;
            enemy.flash = 0.12;
            enemy.velocity += (enemy_position - self.player.position).normalize_or_zero() * 88.0;
            self.damage_number(enemy_position, damage, GLOOM_PURPLE);
            self.impact_ring(enemy_position, damage, GLOOM_PURPLE);
            self.burst(position, GLOOM_PURPLE, 2, 66.0);
            self.shake = self.shake.max(2.8);
        }
    }

    fn update_prism_needles(&mut self, dt: f32) {
        if self.prism_needles.is_empty() {
            return;
        }

        let mut impact_events = Vec::new();
        let mut index = self.prism_needles.len();
        while index > 0 {
            index -= 1;
            let mut needle = self.prism_needles[index];

            needle.life -= dt;
            needle.range -= needle.velocity.length() * dt;
            if needle.life <= 0.0 || needle.range <= 0.0 {
                self.prism_needles.swap_remove(index);
                continue;
            }

            if let Some(target) =
                self.nearest_enemy_index(needle.position, Some(self.prism_bolts.range * 0.95))
            {
                let seek = (self.enemies[target].position - needle.position).normalize_or_zero();
                needle.velocity = needle.velocity.lerp(seek * needle.speed, needle.homing);
            } else {
                needle.velocity *= 0.995;
            }
            needle.position += needle.velocity * dt;

            let mut impact = None;
            for enemy_index in 0..self.enemies.len() {
                if self.enemies[enemy_index].health <= 0.0 {
                    continue;
                }
                let strike_radius = self.enemies[enemy_index].radius + needle.radius;
                if needle
                    .position
                    .distance_squared(self.enemies[enemy_index].position)
                    < strike_radius * strike_radius
                {
                    impact = Some(enemy_index);
                    break;
                }
            }

            let Some(enemy_index) = impact else {
                self.prism_needles[index] = needle;
                continue;
            };

            impact_events.push((enemy_index, needle.position, needle.damage));
            if needle.pierce == 0 {
                self.prism_needles.swap_remove(index);
                continue;
            }
            needle.pierce = needle.pierce.saturating_sub(1);
            self.prism_needles[index] = needle;
        }

        for &(enemy_index, position, damage) in &impact_events {
            let Some(enemy) = self.enemies.get_mut(enemy_index) else {
                continue;
            };
            let enemy_position = enemy.position;
            enemy.health -= damage;
            enemy.flash = 0.12;
            enemy.velocity += (enemy_position - self.player.position).normalize_or_zero() * 96.0;
            self.damage_number(enemy_position, damage, PHANTOM_CERULEAN);
            self.impact_ring(enemy_position, damage, PHANTOM_CERULEAN);
            self.burst(position, PHANTOM_CERULEAN, 2, 65.0);
            self.shake = self.shake.max(2.8);
        }
    }

    fn update_inferno_meteors(&mut self, dt: f32) {
        if self.inferno_meteors.is_empty() {
            return;
        }

        let mut detonations = Vec::new();
        let mut index = self.inferno_meteors.len();
        while index > 0 {
            index -= 1;
            let mut meteor = self.inferno_meteors[index];

            meteor.timer -= dt;
            meteor.arming = (meteor.arming - dt).max(0.0);
            meteor.velocity.y += meteor.gravity * dt;
            meteor.position += meteor.velocity * dt;

            let should_detonate = meteor.timer <= 0.0
                || (meteor.arming <= 0.0
                    && self
                        .nearest_enemy_index(meteor.position, Some(meteor.blast_radius + 10.0))
                        .is_some());
            if should_detonate {
                detonations.push((meteor.position, meteor.damage, meteor.blast_radius));
                self.inferno_meteors.swap_remove(index);
                continue;
            }

            self.inferno_meteors[index] = meteor;
        }

        for (position, damage, blast_radius) in detonations {
            let hit_any = self.area_strike(position, blast_radius, damage, 130.0, CINDER_ORANGE);
            if hit_any {
                self.burst(position, CINDER_ORANGE, 4, 84.0);
                self.impact_ring(position, blast_radius, CINDER_ORANGE);
                self.shake = self.shake.max(4.2);
            }
        }
    }

    fn update_gravity_wells(&mut self, dt: f32) {
        if self.gravity_wells.is_empty() {
            return;
        }

        let mut impacts = Vec::new();
        let mut index = self.gravity_wells.len();
        while index > 0 {
            index -= 1;
            let mut well = self.gravity_wells[index];
            well.life -= dt;
            well.timer = (well.timer - dt).max(0.0);

            let radius = well.max_radius
                * (0.18 + 0.82 * (1.0 - (well.life / well.max_life).clamp(0.0, 1.0)));
            for enemy in &mut self.enemies {
                if enemy.health <= 0.0 {
                    continue;
                }
                let to_center = well.center - enemy.position;
                let distance_sq = to_center.length_squared();
                if distance_sq <= radius * radius {
                    let distance = distance_sq.sqrt();
                    let pull = (1.0 - (distance / radius).min(1.0)) * well.pull;
                    enemy.velocity += to_center.normalize_or_zero() * pull * 1.3;
                }
            }

            if well.timer <= 0.0 {
                let radius_sq = radius * radius;
                for enemy in self.enemies.iter_mut().filter(|enemy| {
                    enemy.health > 0.0 && enemy.position.distance_squared(well.center) <= radius_sq
                }) {
                    let enemy_position = enemy.position;
                    enemy.health -= well.damage;
                    enemy.flash = 0.12;
                    enemy.velocity +=
                        (well.center - enemy.position).normalize_or_zero() * well.pull;
                    impacts.push((
                        enemy_position,
                        well.damage,
                        self.player.position.distance(enemy_position),
                    ));
                }
                well.timer = well.interval;
                self.gravity_wells[index] = well;
            }

            if well.life <= 0.0 {
                self.gravity_wells.swap_remove(index);
            } else {
                self.gravity_wells[index] = well;
            }
        }

        for &(position, damage, distance) in &impacts {
            self.damage_number(position, damage, RIFT_CYAN);
            if distance <= 5.0 {
                self.impact_ring(position, damage * 0.5, RIFT_CYAN);
            } else {
                self.impact_ring(position, damage, RIFT_CYAN);
            }
        }
        if !impacts.is_empty() {
            self.shake = self.shake.max(4.8);
            self.burst(self.player.position, RIFT_CYAN, 3, 60.0);
        }
    }

    fn update_sigil_threads(&mut self, dt: f32) {
        if self.sigil_threads.is_empty() {
            return;
        }

        let mut detonations = Vec::new();
        let mut index = self.sigil_threads.len();
        while index > 0 {
            index -= 1;
            let mut thread = self.sigil_threads[index];
            thread.timer = (thread.timer - dt).max(0.0);
            thread.arming = (thread.arming - dt).max(0.0);
            thread.angle = (thread.angle + thread.angular_speed * dt) % TAU;

            let position =
                self.player.position + Vec2::from_angle(thread.angle) * thread.orbit_radius;
            let should_detonate = thread.timer <= 0.0
                || (thread.arming <= 0.0
                    && self
                        .nearest_enemy_index(position, Some(thread.blast_radius + 8.0))
                        .is_some());
            if should_detonate {
                detonations.push((position, thread.damage, thread.blast_radius));
                self.sigil_threads.swap_remove(index);
                continue;
            }

            self.sigil_threads[index] = thread;
        }

        for (position, damage, blast_radius) in detonations {
            let mut hit_any = false;
            let mut impact = Vec::new();
            let radius_sq = blast_radius * blast_radius;
            for enemy in &mut self.enemies {
                if enemy.health <= 0.0 {
                    continue;
                }
                if enemy.position.distance_squared(position) <= radius_sq {
                    enemy.health -= damage;
                    enemy.flash = 0.12;
                    enemy.velocity +=
                        (enemy.position - position).normalize_or_zero() * self.sigil_net.pull;
                    impact.push((enemy.position, damage));
                    hit_any = true;
                }
            }
            if hit_any {
                for (position, damage) in impact {
                    self.damage_number(position, damage, PHANTOM_NET_GREEN);
                    self.impact_ring(position, damage, PHANTOM_NET_GREEN);
                }
                self.burst(position, PHANTOM_NET_GREEN, 3, 74.0);
                self.impact_ring(position, blast_radius, PHANTOM_NET_GREEN);
                self.shake = self.shake.max(2.9);
            }
        }
    }

    fn update_echo_cannon(&mut self, dt: f32) {
        if self.echo_shards.is_empty() {
            return;
        }

        let mut index = self.echo_shards.len();
        while index > 0 {
            index -= 1;
            let mut shard = self.echo_shards[index];
            let speed = shard.velocity.length();
            if speed <= 0.001 {
                self.echo_shards.swap_remove(index);
                continue;
            }

            shard.life -= speed * dt;
            shard.position += shard.velocity * dt;
            if shard.life <= 0.0 {
                self.echo_shards.swap_remove(index);
                continue;
            }

            let mut impact: Option<usize> = None;
            for enemy_index in 0..self.enemies.len() {
                if self.enemies[enemy_index].health <= 0.0 {
                    continue;
                }
                let strike_radius = self.enemies[enemy_index].radius
                    + shard.radius
                    + self.echo_cannon.level as f32 * 0.3;
                if shard
                    .position
                    .distance_squared(self.enemies[enemy_index].position)
                    < strike_radius * strike_radius
                {
                    impact = Some(enemy_index);
                    break;
                }
            }

            let Some(enemy_index) = impact else {
                self.echo_shards[index] = shard;
                continue;
            };

            let Some(impact_position) = self.enemies.get_mut(enemy_index).map(|enemy| {
                let to_shard = (enemy.position - shard.position).normalize_or_zero();
                enemy.health -= shard.damage;
                enemy.flash = 0.12;
                enemy.velocity += to_shard * 165.0;
                enemy.position
            }) else {
                self.echo_shards[index] = shard;
                continue;
            };
            self.damage_number(impact_position, shard.damage, PHANTOM_CERULEAN);
            self.impact_ring(impact_position, shard.damage, PHANTOM_CERULEAN);
            self.burst(shard.position, PHANTOM_CERULEAN, 2, 90.0);
            self.shake = self.shake.max(3.0);

            if shard.bounces_left > 0 {
                shard.bounces_left = shard.bounces_left.saturating_sub(1);
                let fallback = vec2(-shard.velocity.y, shard.velocity.x).normalize_or_zero();
                let direction = if speed > 0.001 {
                    (self.player.position - shard.position).normalize_or_zero()
                } else {
                    fallback
                };
                shard.velocity = (shard.velocity
                    - direction * (2.0 * shard.velocity.dot(direction)) * 1.1)
                    .normalize_or_zero()
                    * speed;
                shard.life *= 0.75;
                self.echo_shards[index] = shard;
            } else {
                self.echo_shards.swap_remove(index);
            }
        }
    }

    fn resolve_echo_cannon(&mut self) {
        if !self.echo_cannon.unlocked() || self.echo_cannon.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.echo_cannon.timer = 0.28;
            return;
        }

        let shards = (self.echo_cannon.shard_count + 1).min(10);
        let angle_origin = self.visual_time * 1.9;
        for index in 0..shards {
            let angle = angle_origin
                + index as f32 * TAU / shards as f32
                + (index as f32 * 0.43).sin() * 0.12;
            let velocity = Vec2::from_angle(angle) * self.echo_cannon.speed;
            self.echo_shards.push(EchoShard {
                position: self.player.position,
                velocity,
                life: self.echo_cannon.range,
                damage: self.echo_cannon.damage,
                radius: 4.8 + self.echo_cannon.level as f32 * 0.06,
                bounces_left: self.echo_cannon.bounces,
            });
        }

        self.echo_cannon.timer = self.echo_cannon.cooldown;
        self.shake = self.shake.max(3.0);
        self.burst(self.player.position, PHANTOM_CERULEAN, 3, 90.0);
    }

    fn resolve_rune_wards(&mut self) {
        if !self.rune_wards.unlocked() || self.rune_wards.timer > 0.0 {
            return;
        }

        if self.enemies.is_empty() {
            self.rune_wards.timer = 0.28;
            return;
        }

        let wards = (self.rune_wards.ward_count + 1).min(8);
        for index in 0..wards {
            let angle = self.visual_time * 0.9
                + index as f32 * TAU / wards as f32
                + (index as f32 + self.rune_wards.level as f32).sin() * 0.09;
            let offset = Vec2::from_angle(angle)
                * (self.rune_wards.radius + 24.0 + self.rune_wards.level as f32 * 2.0);
            self.rune_pulses.push(RunePulse {
                position: self.player.position + offset,
                life: self.rune_wards.duration,
                max_life: self.rune_wards.duration,
                timer: 0.0,
                interval: self.rune_wards.pulse_interval,
                radius: self.rune_wards.radius,
                damage: self.rune_wards.damage,
                pull: self.rune_wards.pull,
            });
        }
        self.rune_wards.timer = self.rune_wards.cooldown;
        self.shake = self.shake.max(2.0);
    }

    fn update_rune_wards(&mut self, dt: f32) {
        if self.rune_pulses.is_empty() {
            return;
        }

        let mut damage_events: Vec<(Vec2, bool, f32)> = Vec::with_capacity(12);
        let mut index = self.rune_pulses.len();
        while index > 0 {
            index -= 1;
            let mut pulse = self.rune_pulses[index];
            pulse.life -= dt;
            pulse.timer = (pulse.timer - dt).max(0.0);
            if pulse.timer > 0.0 {
                self.rune_pulses[index] = pulse;
                continue;
            }

            let radius =
                pulse.radius * (1.0 + (1.0 - (pulse.life / pulse.max_life).clamp(0.0, 1.0)) * 0.35);
            let mut pulse_hits: Vec<(Vec2, bool)> = Vec::new();
            for enemy_index in 0..self.enemies.len() {
                let enemy = &mut self.enemies[enemy_index];
                if enemy.health <= 0.0 {
                    continue;
                }

                let to_enemy = enemy.position - pulse.position;
                let distance_sq = to_enemy.length_squared();
                if distance_sq <= radius * radius {
                    let distance = distance_sq.sqrt();
                    let pull = (1.0 - (distance / radius).min(1.0)) * pulse.pull;
                    enemy.health -= pulse.damage;
                    enemy.flash = 0.12;
                    enemy.velocity +=
                        (self.player.position - enemy.position).normalize_or_zero() * pull * 1.8;
                    pulse_hits.push((enemy.position, enemy.health <= 0.0));
                }
            }

            if pulse_hits.is_empty() {
                if pulse.life <= 0.0 {
                    self.rune_pulses.swap_remove(index);
                } else {
                    pulse.timer = pulse.interval;
                    self.rune_pulses[index] = pulse;
                }
            } else {
                damage_events.extend(
                    pulse_hits
                        .into_iter()
                        .map(|(position, dead)| (position, dead, pulse.damage)),
                );
                self.shake = self.shake.max(2.2);
                self.burst(pulse.position, PHANTOM_EMBER, 2, 60.0);
                self.impact_ring(pulse.position, pulse.damage, PHANTOM_EMBER);
                pulse.timer = pulse.interval;
                self.rune_pulses[index] = pulse;
            }
        }

        for (position, dead, damage) in damage_events {
            self.damage_number(position, damage, PHANTOM_EMBER);
            if dead {
                self.shake = self.shake.max(1.5);
            }
        }
    }

    fn resolve_player_contact(&mut self) {
        if self.player.invulnerability > 0.0 {
            return;
        }

        let closest = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, enemy)| {
                let contact = PLAYER_RADIUS + enemy.radius;
                enemy.position.distance_squared(self.player.position) < contact * contact
            })
            .min_by(|(_, left), (_, right)| {
                left.position
                    .distance_squared(self.player.position)
                    .total_cmp(&right.position.distance_squared(self.player.position))
            })
            .map(|(index, _)| index);

        if let Some(index) = closest {
            let enemy = &mut self.enemies[index];
            let away = (self.player.position - enemy.position).normalize_or_zero();
            let damage = enemy.kind.contact_damage();
            self.player.health -= damage;
            self.player.velocity += away * 280.0;
            enemy.velocity -= away * 170.0;
            self.player.invulnerability = 0.72;
            self.shake = 13.0;
            self.hit_stop = 0.055;
            let impact_position = self.player.position + vec2(0.0, -28.0);
            self.damage_number(impact_position, damage, DAMAGE_RED);
            self.impact_ring(self.player.position, damage, DAMAGE_RED);
            self.burst(self.player.position, DAMAGE_RED, 12, 155.0);
        }
    }

    fn update_grave_shards(&mut self, dt: f32) {
        let seek_range = self.grave_mines.blast_radius * 4.0;
        let homing = self.grave_mines.homing;
        let mut charges = std::mem::take(&mut self.grave_shards);
        let detonations = self.advance_armed_charges(
            &mut charges,
            dt,
            ChargeMotion::homing(seek_range, homing, 10.0),
        );
        self.grave_shards = charges;
        self.detonate_charges(&detonations, 130.0, GRAVE_MARROW, 5, 90.0, 0.0);
    }

    /// Advance armed charges: optional homing, optional gravity, arming + proximity detonation.
    fn advance_armed_charges(
        &self,
        charges: &mut Vec<ArmedCharge>,
        dt: f32,
        motion: ChargeMotion,
    ) -> Vec<(Vec2, f32, f32)> {
        let mut detonations = Vec::new();
        let mut index = charges.len();
        while index > 0 {
            index -= 1;
            let mut charge = charges[index];
            charge.timer -= dt;
            charge.arming = (charge.arming - dt).max(0.0);
            charge.velocity.y += motion.gravity * dt;

            if motion.seek_range > 0.0 {
                if let Some(target) =
                    self.nearest_enemy_index(charge.position, Some(motion.seek_range))
                {
                    let seek =
                        (self.enemies[target].position - charge.position).normalize_or_zero();
                    charge.velocity = charge
                        .velocity
                        .lerp(seek * motion.homing, smoothing_weight(motion.seek_rate, dt));
                } else {
                    charge.velocity *= (-2.6 * dt).exp();
                }
            }

            charge.position += charge.velocity * dt;

            let near_enemy = charge.arming <= 0.0
                && self
                    .nearest_enemy_index(
                        charge.position,
                        Some(charge.blast_radius + motion.proximity_pad),
                    )
                    .is_some();
            if charge.timer <= 0.0 || near_enemy {
                detonations.push((charge.position, charge.damage, charge.blast_radius));
                charges.swap_remove(index);
            } else {
                charges[index] = charge;
            }
        }
        detonations
    }

    fn detonate_charges(
        &mut self,
        detonations: &[(Vec2, f32, f32)],
        knock: f32,
        color: Color,
        burst_count: usize,
        burst_speed: f32,
        shake: f32,
    ) {
        for &(position, damage, blast_radius) in detonations {
            if self.area_strike(position, blast_radius, damage, knock, color) {
                self.burst(position, color, burst_count, burst_speed);
                self.impact_ring(position, blast_radius, color);
                if shake > 0.0 {
                    self.shake = self.shake.max(shake);
                }
            }
        }
    }

    fn update_harrow_darts(&mut self, dt: f32) {
        if self.harrow_darts.is_empty() {
            return;
        }

        let mut hit_events = Vec::new();
        let mut index = self.harrow_darts.len();
        while index > 0 {
            index -= 1;
            let mut dart = self.harrow_darts[index];
            dart.life -= dt;
            if dart.life <= 0.0 {
                self.harrow_darts.swap_remove(index);
                continue;
            }

            if let Some(target) =
                self.nearest_enemy_index(dart.position, Some(self.harrow_volley.range))
            {
                let target_direction =
                    (self.enemies[target].position - dart.position).normalize_or_zero();
                dart.velocity = dart.velocity.lerp(
                    target_direction * self.harrow_volley.speed,
                    smoothing_weight(17.0, dt),
                );
            } else {
                dart.velocity *= 0.985;
            }

            dart.position += dart.velocity * dt;
            let mut impact = None;
            for enemy_index in 0..self.enemies.len() {
                if self.enemies[enemy_index].health <= 0.0 {
                    continue;
                }
                let radius = self.enemies[enemy_index].radius + dart.radius;
                if dart
                    .position
                    .distance_squared(self.enemies[enemy_index].position)
                    < radius * radius
                {
                    impact = Some((enemy_index, dart.damage));
                    break;
                }
            }

            if let Some((enemy_index, damage)) = impact {
                hit_events.push((enemy_index, dart.position, damage));
                self.harrow_darts.swap_remove(index);
            } else {
                self.harrow_darts[index] = dart;
            }
        }

        for (enemy_index, position, damage) in hit_events {
            let show = if let Some(enemy) = self.enemies.get_mut(enemy_index) {
                enemy.health -= damage;
                enemy.flash = 0.12;
                enemy.velocity +=
                    (enemy.position - self.player.position).normalize_or_zero() * 95.0;
                Some((enemy.position, damage))
            } else {
                None
            };
            if let Some((impact_position, impact_damage)) = show {
                self.damage_number(impact_position, impact_damage, MOON_GOLD);
                self.impact_ring(impact_position, impact_damage, MOON_GOLD);
                self.burst(position, MOON_GOLD, 3, 90.0);
            }
        }
    }

    fn update_aether_projectiles(&mut self, dt: f32) {
        if self.aether_projectiles.is_empty() {
            return;
        }

        let mut impact_events = Vec::new();
        let mut index = self.aether_projectiles.len();
        while index > 0 {
            index -= 1;
            let mut projectile = self.aether_projectiles[index];

            projectile.life -= dt;
            projectile.range -= projectile.velocity.length() * dt;
            if projectile.life <= 0.0 || projectile.range <= 0.0 {
                self.aether_projectiles.swap_remove(index);
                continue;
            }

            if let Some(target) =
                self.nearest_enemy_index(projectile.position, Some(self.aether_spears.range * 0.95))
            {
                let seek =
                    (self.enemies[target].position - projectile.position).normalize_or_zero();
                projectile.velocity = projectile
                    .velocity
                    .lerp(seek * self.aether_spears.speed, self.aether_spears.homing);
            }

            projectile.position += projectile.velocity * dt;

            let mut hit = None;
            for enemy_index in 0..self.enemies.len() {
                if self.enemies[enemy_index].health <= 0.0 {
                    continue;
                }
                let radius = self.enemies[enemy_index].radius + projectile.radius;
                if projectile
                    .position
                    .distance_squared(self.enemies[enemy_index].position)
                    < radius * radius
                {
                    hit = Some(enemy_index);
                    break;
                }
            }

            if let Some(enemy_index) = hit {
                impact_events.push((enemy_index, projectile.position, projectile.damage));
                if projectile.pierce > 0 {
                    projectile.pierce = projectile.pierce.saturating_sub(1);
                }
                if projectile.pierce == 0 {
                    self.aether_projectiles.swap_remove(index);
                    continue;
                }
                self.aether_projectiles[index] = projectile;
                continue;
            }

            self.aether_projectiles[index] = projectile;
        }

        let mut impacted_enemy = false;
        for (enemy_index, position, damage) in &impact_events {
            impacted_enemy = true;
            let enemy_index = *enemy_index;
            let position = *position;
            let damage = *damage;
            let struck = self
                .enemies
                .get_mut(enemy_index)
                .filter(|enemy| enemy.health > 0.0)
                .map(|enemy| {
                    enemy.health -= damage;
                    enemy.flash = 0.12;
                    enemy.velocity +=
                        (enemy.position - self.player.position).normalize_or_zero() * 115.0;
                    (enemy.position, enemy.health <= 0.0, position, damage)
                });
            if let Some((enemy_position, dead, burst_position, impact_damage)) = struck {
                self.damage_number(enemy_position, impact_damage, AETHER_MAUVE);
                self.impact_ring(enemy_position, impact_damage, AETHER_MAUVE);
                self.burst(burst_position, AETHER_MAUVE, 2, 58.0);
                if dead {
                    self.shake = self.shake.max(1.8);
                }
            }
        }
        if impacted_enemy {
            self.shake = self.shake.max(4.0);
        }
    }

    fn update_rift_echoes(&mut self, dt: f32) {
        if self.rift_echoes.is_empty() {
            return;
        }

        let mut impact_events = Vec::new();
        let mut index = self.rift_echoes.len();
        while index > 0 {
            index -= 1;
            let mut echo = self.rift_echoes[index];

            echo.life -= dt;
            echo.timer = (echo.timer - dt).max(0.0);

            if echo.timer <= 0.0 {
                let progress = (1.0 - echo.life / echo.max_life).clamp(0.0, 1.0);
                let radius = echo.max_radius * progress * echo.range_scale;
                let radius_sq = radius * radius;
                let hit_any = self.apply_rift_echo_hits(
                    echo.center,
                    radius_sq,
                    echo.damage,
                    &mut impact_events,
                );
                if hit_any {
                    self.impact_ring(echo.center, echo.damage, RIFT_VIOLET);
                }
                echo.timer += echo.interval;
            }

            if echo.life <= 0.0 {
                self.rift_echoes.swap_remove(index);
            } else {
                self.rift_echoes[index] = echo;
            }
        }
        let mut index = impact_events.len();
        while index > 0 {
            index -= 1;
            if let Some((position, damage)) = impact_events.get(index).copied() {
                self.damage_number(position, damage, RIFT_VIOLET);
            }
        }
    }

    fn apply_rift_echo_hits(
        &mut self,
        center: Vec2,
        radius_sq: f32,
        damage: f32,
        impact_events: &mut Vec<(Vec2, f32)>,
    ) -> bool {
        let mut hit_any = false;
        for enemy in &mut self.enemies {
            if enemy.health <= 0.0 {
                continue;
            }
            if enemy.position.distance_squared(center) >= radius_sq {
                continue;
            }
            enemy.health -= damage;
            enemy.flash = 0.12;
            enemy.velocity += (enemy.position - center).normalize_or_zero() * 75.0;
            impact_events.push((enemy.position, damage));
            hit_any = true;
        }
        if hit_any {
            self.shake = self.shake.max(0.9);
        }
        hit_any
    }

    fn area_strike(
        &mut self,
        center: Vec2,
        radius: f32,
        damage: f32,
        knock: f32,
        color: Color,
    ) -> bool {
        let mut any_hit = false;
        let mut hit_positions = Vec::new();
        let radius_sq = radius * radius;
        for enemy in &mut self.enemies {
            if enemy.health <= 0.0 {
                continue;
            }
            if enemy.position.distance_squared(center) < radius_sq {
                enemy.health -= damage;
                enemy.flash = 0.12;
                enemy.velocity += (enemy.position - center).normalize_or_zero() * knock;
                hit_positions.push(enemy.position);
                any_hit = true;
            }
        }
        if any_hit {
            for position in hit_positions {
                self.damage_number(position, damage, color);
                self.impact_ring(position, damage, color);
            }
        }
        any_hit
    }

    fn remove_defeated_enemies(&mut self) {
        // Walk backward because `swap_remove` changes the element at the current index.
        let mut index = self.enemies.len();
        while index > 0 {
            index -= 1;
            if self.enemies[index].health > 0.0 {
                continue;
            }
            let enemy = self.enemies.swap_remove(index);
            self.gems.push(Gem {
                position: enemy.position,
                velocity: Vec2::from_angle(rand::gen_range(0.0, TAU)) * rand::gen_range(35.0, 75.0),
                value: enemy.kind.experience(),
            });
            self.burst(enemy.position, ARCANE_VIOLET, 9, 130.0);
            self.kills += 1;
        }
    }

    fn collect_gems(&mut self, dt: f32) {
        let mut gained = 0;
        let mut index = self.gems.len();
        while index > 0 {
            index -= 1;
            let gem = &mut self.gems[index];
            let offset = self.player.position - gem.position;
            let distance = offset.length();
            if distance < self.player.pickup_radius {
                let pull = 560.0 + (self.player.pickup_radius - distance) * 7.0;
                gem.velocity += offset.normalize_or_zero() * pull * dt;
            }
            gem.velocity *= (-3.2 * dt).exp();
            gem.position += gem.velocity * dt;

            if distance < PLAYER_RADIUS + 8.0 {
                gained += gem.value;
                self.gems.swap_remove(index);
            }
        }

        if gained > 0 {
            self.pickup_flash = 0.24;
            self.burst(
                self.player.position,
                GEM_GREEN,
                gained.clamp(2, 7) as usize,
                65.0,
            );
        }

        self.player.experience += gained;
        if self.player.experience >= self.player.next_level {
            self.begin_level_up();
        }
    }

    fn begin_level_up(&mut self) {
        self.player.experience -= self.player.next_level;
        self.player.level += 1;
        self.player.next_level = 8 + self.player.level * 4 + self.player.level.pow(2) / 3;
        self.offers = random_offers();
        self.phase = Phase::LevelUp;
    }

    #[allow(
        clippy::too_many_lines,
        reason = "upgrade matrix is data-table-like and readable grouped by weapon"
    )]
    fn apply_upgrade(&mut self, upgrade: Upgrade) {
        match upgrade {
            Upgrade::ExtraKnife => {
                self.moon.count += 1;
                self.moon.level += 1;
            },
            Upgrade::SharpenedMoon => {
                self.moon.damage *= 1.50;
                self.moon.level += 1;
            },
            Upgrade::WiderOrbit => {
                self.moon.radius += 14.0;
                self.moon.speed *= 1.18;
                self.moon.level += 1;
            },
            Upgrade::FastStorm => {
                self.storm.cooldown *= 0.72;
                self.storm.level += 1;
            },
            Upgrade::ForkedStorm => {
                self.storm.jumps += 2;
                self.storm.range += 40.0;
                self.storm.level += 1;
            },
            Upgrade::PotentStorm => {
                self.storm.damage *= 1.50;
                self.storm.level += 1;
            },
            Upgrade::GraveMine => {
                let was_locked = self.grave_mines.level == 0;
                self.grave_mines.unlock();
                if !was_locked {
                    self.grave_mines.shard_count += 1;
                    self.grave_mines.damage *= 1.10;
                    self.grave_mines.homing += 6.0;
                }
                self.grave_mines.level += 1;
            },
            Upgrade::GraveMineReach => {
                self.grave_mines.unlock();
                self.grave_mines.blast_radius += 8.0;
                self.grave_mines.homing += 9.0;
                self.grave_mines.shard_count = (self.grave_mines.shard_count + 1).max(1);
                self.grave_mines.damage *= 1.08;
                self.grave_mines.level += 1;
            },
            Upgrade::GraveMineRage => {
                self.grave_mines.unlock();
                self.grave_mines.damage *= 1.24;
                self.grave_mines.cooldown *= 0.88;
                self.grave_mines.level += 1;
            },
            Upgrade::AstralFlare => {
                let was_locked = self.astral_flare.level == 0;
                self.astral_flare.unlock();
                if !was_locked {
                    self.astral_flare.damage *= 1.12;
                    self.astral_flare.radius += 8.0;
                }
                self.astral_flare.level += 1;
            },
            Upgrade::AstralFlareBloom => {
                self.astral_flare.unlock();
                self.astral_flare.radius += 12.0;
                self.astral_flare.damage *= 1.08;
                self.astral_flare.level += 1;
            },
            Upgrade::AstralFlarePulse => {
                self.astral_flare.unlock();
                self.astral_flare.damage *= 1.28;
                self.astral_flare.cooldown *= 0.86;
                self.astral_flare.level += 1;
            },
            Upgrade::WraithLash => {
                let was_locked = self.wraith_lash.level == 0;
                self.wraith_lash.unlock();
                if !was_locked {
                    self.wraith_lash.damage *= 1.10;
                    self.wraith_lash.range += 14.0;
                    self.wraith_lash.width += 0.8;
                }
                self.wraith_lash.level += 1;
            },
            Upgrade::WraithLashReach => {
                self.wraith_lash.unlock();
                self.wraith_lash.range += 32.0;
                self.wraith_lash.damage *= 1.05;
                self.wraith_lash.level += 1;
            },
            Upgrade::WraithLashRend => {
                self.wraith_lash.unlock();
                self.wraith_lash.width += 2.6;
                self.wraith_lash.damage *= 1.32;
                self.wraith_lash.level += 1;
            },
            Upgrade::HarrowVolley => {
                let was_locked = self.harrow_volley.level == 0;
                self.harrow_volley.unlock();
                if !was_locked {
                    self.harrow_volley.dart_count += 1;
                    self.harrow_volley.damage *= 1.12;
                }
                self.harrow_volley.level += 1;
            },
            Upgrade::HarrowVolleyAim => {
                self.harrow_volley.unlock();
                self.harrow_volley.speed += 22.0;
                self.harrow_volley.dart_count = (self.harrow_volley.dart_count + 1).max(1);
                self.harrow_volley.damage += 2.5;
                self.harrow_volley.level += 1;
            },
            Upgrade::HarrowVolleyPierce => {
                self.harrow_volley.unlock();
                self.harrow_volley.damage *= 1.24;
                self.harrow_volley.cooldown *= 0.89;
                self.harrow_volley.level += 1;
            },
            Upgrade::AetherSpear => {
                self.aether_spears.unlock();
                self.aether_spears.level += 1;
            },
            Upgrade::AetherSpearSplit => {
                self.aether_spears.unlock();
                self.aether_spears.spear_count = (self.aether_spears.spear_count + 1).max(2);
                self.aether_spears.damage *= 1.07;
                self.aether_spears.homing = (self.aether_spears.homing + 0.05).min(0.4);
                self.aether_spears.range += 28.0;
                self.aether_spears.level += 1;
            },
            Upgrade::AetherSpearRage => {
                self.aether_spears.unlock();
                self.aether_spears.pierce = (self.aether_spears.pierce + 1).max(2);
                self.aether_spears.cooldown *= 0.84;
                self.aether_spears.damage *= 1.16;
                self.aether_spears.speed += 36.0;
                self.aether_spears.level += 1;
            },
            Upgrade::RiftPulse => {
                self.rift_pulse.unlock();
                self.rift_pulse.level += 1;
            },
            Upgrade::RiftPulseAnchor => {
                self.rift_pulse.unlock();
                self.rift_pulse.radius += 20.0;
                self.rift_pulse.duration += 0.12;
                self.rift_pulse.damage *= 1.11;
                self.rift_pulse.range += 36.0;
                self.rift_pulse.level += 1;
            },
            Upgrade::RiftPulseCascade => {
                self.rift_pulse.unlock();
                self.rift_pulse.damage *= 1.2;
                self.rift_pulse.tick_interval *= 0.78;
                self.rift_pulse.cooldown *= 0.88;
                self.rift_pulse.duration += 0.08;
                self.rift_pulse.level += 1;
            },
            Upgrade::EchoCannon => {
                let was_locked = self.echo_cannon.level == 0;
                self.echo_cannon.unlock();
                if !was_locked {
                    self.echo_cannon.damage *= 1.09;
                    self.echo_cannon.shard_count = (self.echo_cannon.shard_count + 1).max(3);
                }
                self.echo_cannon.level += 1;
            },
            Upgrade::EchoCannonResonance => {
                self.echo_cannon.unlock();
                self.echo_cannon.shard_count += 1;
                self.echo_cannon.damage *= 1.16;
                self.echo_cannon.speed += 18.0;
                self.echo_cannon.level += 1;
            },
            Upgrade::EchoCannonCataclysm => {
                self.echo_cannon.unlock();
                self.echo_cannon.cooldown *= 0.82;
                self.echo_cannon.damage *= 1.22;
                self.echo_cannon.bounces += 1;
                self.echo_cannon.range += 56.0;
                self.echo_cannon.level += 1;
            },
            Upgrade::RuneWard => {
                let was_locked = self.rune_wards.level == 0;
                self.rune_wards.unlock();
                if !was_locked {
                    self.rune_wards.ward_count = (self.rune_wards.ward_count + 1).max(2);
                    self.rune_wards.duration += 0.05;
                }
                self.rune_wards.level += 1;
            },
            Upgrade::RuneWardEcho => {
                self.rune_wards.unlock();
                self.rune_wards.radius += 10.0;
                self.rune_wards.damage *= 1.11;
                self.rune_wards.pull += 8.0;
                self.rune_wards.ward_count += 1;
                self.rune_wards.level += 1;
            },
            Upgrade::RuneWardCataclysm => {
                self.rune_wards.unlock();
                self.rune_wards.cooldown *= 0.8;
                self.rune_wards.pulse_interval *= 0.88;
                self.rune_wards.duration += 0.12;
                self.rune_wards.level += 1;
            },
            Upgrade::CrescentHalo => {
                self.crescent_halo.unlock();
                self.crescent_halo.level += 1;
            },
            Upgrade::CrescentHaloSpiral => {
                self.crescent_halo.unlock();
                self.crescent_halo.blades = (self.crescent_halo.blades + 2).max(4);
                self.crescent_halo.radius += 12.0;
                self.crescent_halo.width += 1.3;
                self.crescent_halo.rotation *= 1.18;
                self.crescent_halo.level += 1;
            },
            Upgrade::CrescentHaloCataclysm => {
                self.crescent_halo.unlock();
                self.crescent_halo.damage *= 1.2;
                self.crescent_halo.cooldown *= 0.84;
                self.crescent_halo.radius += 8.0;
                self.crescent_halo.blades = (self.crescent_halo.blades + 1).min(10);
                self.crescent_halo.level += 1;
            },
            Upgrade::VoidBloom => {
                self.void_bloom.unlock();
                self.void_bloom.level += 1;
            },
            Upgrade::VoidBloomReach => {
                self.void_bloom.unlock();
                self.void_bloom.seed_count = (self.void_bloom.seed_count + 1).max(3);
                self.void_bloom.damage *= 1.09;
                self.void_bloom.blast_radius += 10.0;
                self.void_bloom.speed += 12.0;
                self.void_bloom.homing = (self.void_bloom.homing + 12.0).min(260.0);
                self.void_bloom.level += 1;
            },
            Upgrade::VoidBloomCascade => {
                self.void_bloom.unlock();
                self.void_bloom.seed_life *= 1.1;
                self.void_bloom.seed_count = (self.void_bloom.seed_count + 1).min(8);
                self.void_bloom.damage *= 1.18;
                self.void_bloom.cooldown *= 0.88;
                self.void_bloom.blast_radius += 6.0;
                self.void_bloom.level += 1;
            },
            Upgrade::SolarNova => {
                self.solar_nova.unlock();
                self.solar_nova.level += 1;
            },
            Upgrade::SolarNovaBloom => {
                self.solar_nova.unlock();
                self.solar_nova.rings = (self.solar_nova.rings + 1).max(3);
                self.solar_nova.radius += 14.0;
                self.solar_nova.damage *= 1.11;
                self.solar_nova.level += 1;
            },
            Upgrade::SolarNovaCataclysm => {
                self.solar_nova.unlock();
                self.solar_nova.rings = (self.solar_nova.rings + 1).max(4);
                self.solar_nova.damage *= 1.22;
                self.solar_nova.cooldown *= 0.84;
                self.solar_nova.level += 1;
            },
            Upgrade::AbyssalMine => {
                self.abyssal_mines.unlock();
                self.abyssal_mines.level += 1;
            },
            Upgrade::AbyssalMineReach => {
                self.abyssal_mines.unlock();
                self.abyssal_mines.mine_count = (self.abyssal_mines.mine_count + 1).max(3);
                self.abyssal_mines.blast_radius += 10.0;
                self.abyssal_mines.homing += 14.0;
                self.abyssal_mines.damage *= 1.1;
                self.abyssal_mines.level += 1;
            },
            Upgrade::AbyssalMineRage => {
                self.abyssal_mines.unlock();
                self.abyssal_mines.damage *= 1.24;
                self.abyssal_mines.cooldown *= 0.87;
                self.abyssal_mines.mine_count = self.abyssal_mines.mine_count.max(4);
                self.abyssal_mines.level += 1;
            },
            Upgrade::Fleet => self.player.speed *= 1.18,
            Upgrade::Vitality => {
                self.player.max_health += 35.0;
                self.player.health = (self.player.health + 35.0).min(self.player.max_health);
            },
            Upgrade::Magnet => self.player.pickup_radius *= 1.45,
            Upgrade::Starfall => {
                self.starfall.unlock();
                self.starfall.level += 1;
            },
            Upgrade::StarfallCascade => {
                self.starfall.unlock();
                self.starfall.meteor_count = (self.starfall.meteor_count + 1).max(2);
                self.starfall.blast_radius += 7.0;
                self.starfall.damage *= 1.1;
                self.starfall.level += 1;
            },
            Upgrade::StarfallCataclysm => {
                self.starfall.unlock();
                self.starfall.damage *= 1.18;
                self.starfall.cooldown *= 0.84;
                self.starfall.gravity += 36.0;
                self.starfall.level += 1;
            },
            Upgrade::PhantomNet => {
                self.phantom_net.unlock();
                self.phantom_net.level += 1;
            },
            Upgrade::PhantomNetReach => {
                self.phantom_net.unlock();
                self.phantom_net.thread_count = (self.phantom_net.thread_count + 1).max(3);
                self.phantom_net.blast_radius += 8.0;
                self.phantom_net.orbit_radius += 4.0;
                self.phantom_net.level += 1;
            },
            Upgrade::PhantomNetRage => {
                self.phantom_net.unlock();
                self.phantom_net.damage *= 1.2;
                self.phantom_net.seek += 0.06;
                self.phantom_net.level += 1;
            },
            Upgrade::LuminousLance => {
                self.luminous_lance.unlock();
                self.luminous_lance.level += 1;
            },
            Upgrade::LuminousLanceFork => {
                self.luminous_lance.unlock();
                self.luminous_lance.spear_count = (self.luminous_lance.spear_count + 1).max(2);
                self.luminous_lance.damage *= 1.08;
                self.luminous_lance.range += 26.0;
                self.luminous_lance.level += 1;
            },
            Upgrade::LuminousLanceRend => {
                self.luminous_lance.unlock();
                self.luminous_lance.pierce = (self.luminous_lance.pierce + 1).max(2);
                self.luminous_lance.damage *= 1.16;
                self.luminous_lance.speed += 32.0;
                self.luminous_lance.level += 1;
            },
            Upgrade::TemporalRift => {
                self.temporal_rift.unlock();
                self.temporal_rift.level += 1;
            },
            Upgrade::TemporalRiftAnchor => {
                self.temporal_rift.unlock();
                self.temporal_rift.gate_count = (self.temporal_rift.gate_count + 1).max(2);
                self.temporal_rift.radius += 10.0;
                self.temporal_rift.pull += 9.0;
                self.temporal_rift.duration += 0.12;
                self.temporal_rift.level += 1;
            },
            Upgrade::TemporalRiftSurge => {
                self.temporal_rift.unlock();
                self.temporal_rift.interval *= 0.8;
                self.temporal_rift.damage *= 1.2;
                self.temporal_rift.cooldown *= 0.87;
                self.temporal_rift.duration += 0.1;
                self.temporal_rift.level += 1;
            },
            Upgrade::ScytheCyclone => {
                self.scythe_cyclone.unlock();
                self.scythe_cyclone.level += 1;
            },
            Upgrade::ScytheCycloneSpiral => {
                self.scythe_cyclone.unlock();
                self.scythe_cyclone.blades = (self.scythe_cyclone.blades + 1).max(2);
                self.scythe_cyclone.width += 1.0;
                self.scythe_cyclone.radius += 7.0;
                self.scythe_cyclone.level += 1;
            },
            Upgrade::ScytheCycloneRavage => {
                self.scythe_cyclone.unlock();
                self.scythe_cyclone.damage *= 1.26;
                self.scythe_cyclone.blades = (self.scythe_cyclone.blades + 1).max(6);
                self.scythe_cyclone.level += 1;
            },
            Upgrade::GloomVolley => {
                self.gloom_volley.unlock();
                self.gloom_volley.level += 1;
            },
            Upgrade::GloomVolleyCage => {
                self.gloom_volley.unlock();
                self.gloom_volley.shard_count = (self.gloom_volley.shard_count + 1).max(2);
                self.gloom_volley.range += 24.0;
                self.gloom_volley.piercing += 1;
                self.gloom_volley.level += 1;
            },
            Upgrade::GloomVolleyEcho => {
                self.gloom_volley.unlock();
                self.gloom_volley.damage *= 1.17;
                self.gloom_volley.piercing = (self.gloom_volley.piercing + 1).max(2);
                self.gloom_volley.speed += 25.0;
                self.gloom_volley.level += 1;
            },
            Upgrade::PulseLance => {
                self.pulse_lance.unlock();
                self.pulse_lance.damage *= 1.16;
                self.pulse_lance.range += 38.0;
                self.pulse_lance.level += 1;
            },
            Upgrade::PulseLanceSurge => {
                self.pulse_lance.unlock();
                self.pulse_lance.chain_count = (self.pulse_lance.chain_count + 1).max(2);
                self.pulse_lance.width += 4.0;
                self.pulse_lance.damage *= 1.12;
                self.pulse_lance.level += 1;
            },
            Upgrade::PulseLanceCataclysm => {
                self.pulse_lance.unlock();
                self.pulse_lance.chain_count = (self.pulse_lance.chain_count + 1).max(4);
                self.pulse_lance.cooldown *= 0.84;
                self.pulse_lance.damage *= 1.24;
                self.pulse_lance.level += 1;
            },
            Upgrade::ShardStorm => {
                self.shard_storm.unlock();
                self.shard_storm.shard_count = (self.shard_storm.shard_count + 1).max(2);
                self.shard_storm.damage *= 1.1;
                self.shard_storm.speed += 12.0;
                self.shard_storm.level += 1;
            },
            Upgrade::ShardStormCascade => {
                self.shard_storm.unlock();
                self.shard_storm.shard_count = (self.shard_storm.shard_count + 1).max(3);
                self.shard_storm.range += 38.0;
                self.shard_storm.pierce += 1;
                self.shard_storm.level += 1;
            },
            Upgrade::ShardStormCataclysm => {
                self.shard_storm.unlock();
                self.shard_storm.damage *= 1.22;
                self.shard_storm.cooldown *= 0.84;
                self.shard_storm.range += 42.0;
                self.shard_storm.level += 1;
            },
            Upgrade::PrismBolts => {
                self.prism_bolts.unlock();
                self.prism_bolts.shot_count = (self.prism_bolts.shot_count + 1).max(2);
                self.prism_bolts.damage *= 1.12;
                self.prism_bolts.level += 1;
            },
            Upgrade::PrismBoltsTwin => {
                self.prism_bolts.unlock();
                self.prism_bolts.shot_count = (self.prism_bolts.shot_count + 1).max(3);
                self.prism_bolts.homing = (self.prism_bolts.homing + 0.05).min(0.45);
                self.prism_bolts.level += 1;
            },
            Upgrade::PrismBoltsRift => {
                self.prism_bolts.unlock();
                self.prism_bolts.speed += 28.0;
                self.prism_bolts.range += 70.0;
                self.prism_bolts.damage *= 1.2;
                self.prism_bolts.level += 1;
            },
            Upgrade::InfernoBomb => {
                self.inferno_bombs.unlock();
                self.inferno_bombs.meteor_count = (self.inferno_bombs.meteor_count + 1).max(2);
                self.inferno_bombs.damage *= 1.08;
                self.inferno_bombs.blast_radius += 8.0;
                self.inferno_bombs.level += 1;
            },
            Upgrade::InfernoBombScatter => {
                self.inferno_bombs.unlock();
                self.inferno_bombs.meteor_count = (self.inferno_bombs.meteor_count + 1).max(3);
                self.inferno_bombs.gravity += 8.0;
                self.inferno_bombs.level += 1;
            },
            Upgrade::InfernoBombCataclysm => {
                self.inferno_bombs.unlock();
                self.inferno_bombs.damage *= 1.22;
                self.inferno_bombs.cooldown *= 0.84;
                self.inferno_bombs.blast_radius += 12.0;
                self.inferno_bombs.level += 1;
            },
            Upgrade::GravityWeave => {
                self.gravity_weave.unlock();
                self.gravity_weave.well_count = (self.gravity_weave.well_count + 1).max(2);
                self.gravity_weave.pull += 10.0;
                self.gravity_weave.damage *= 1.08;
                self.gravity_weave.level += 1;
            },
            Upgrade::GravityWeaveAnchor => {
                self.gravity_weave.unlock();
                self.gravity_weave.interval *= 0.82;
                self.gravity_weave.duration += 0.16;
                self.gravity_weave.well_count = (self.gravity_weave.well_count + 1).max(2);
                self.gravity_weave.level += 1;
            },
            Upgrade::GravityWeaveCollapse => {
                self.gravity_weave.unlock();
                self.gravity_weave.max_radius += 12.0;
                self.gravity_weave.damage *= 1.2;
                self.gravity_weave.pull += 12.0;
                self.gravity_weave.level += 1;
            },
            Upgrade::SigilNet => {
                self.sigil_net.unlock();
                self.sigil_net.thread_count = (self.sigil_net.thread_count + 1).max(2);
                self.sigil_net.damage *= 1.08;
                self.sigil_net.level += 1;
            },
            Upgrade::SigilNetReach => {
                self.sigil_net.unlock();
                self.sigil_net.orbit_radius += 8.0;
                self.sigil_net.pull += 6.0;
                self.sigil_net.level += 1;
            },
            Upgrade::SigilNetRuin => {
                self.sigil_net.unlock();
                self.sigil_net.damage *= 1.2;
                self.sigil_net.blast_radius += 4.0;
                self.sigil_net.level += 1;
            },
        }
    }

    fn nearest_enemy_index(&self, origin: Vec2, max_distance: Option<f32>) -> Option<usize> {
        let max_distance = max_distance.map(|value| value * value);
        self.enemies
            .iter()
            .enumerate()
            .filter(|(_, enemy)| enemy.health > 0.0)
            .filter(|(_, enemy)| {
                max_distance
                    .is_none_or(|limit_sq| enemy.position.distance_squared(origin) <= limit_sq)
            })
            .min_by(|(_, left), (_, right)| {
                left.position
                    .distance_squared(origin)
                    .total_cmp(&right.position.distance_squared(origin))
            })
            .map(|(index, _)| index)
    }

    fn update_effects(&mut self, dt: f32) {
        self.phase_flash = (self.phase_flash - dt).max(0.0);
        for particle in &mut self.particles {
            particle.life -= dt;
            particle.position += particle.velocity * dt;
            particle.velocity *= (-4.5 * dt).exp();
        }
        self.particles.retain(|particle| particle.life > 0.0);

        for lightning in &mut self.lightning {
            lightning.life -= dt;
        }
        self.lightning.retain(|lightning| lightning.life > 0.0);

        for pulse in &mut self.flare_pulses {
            pulse.life -= dt;
        }
        self.flare_pulses.retain(|pulse| pulse.life > 0.0);

        for trace in &mut self.wraith_traces {
            trace.life -= dt;
        }
        self.wraith_traces.retain(|trace| trace.life > 0.0);

        for number in &mut self.damage_numbers {
            number.life -= dt;
            number.position.y -= 34.0 * dt;
        }
        self.damage_numbers.retain(|number| number.life > 0.0);

        for ring in &mut self.impact_rings {
            ring.life -= dt;
        }
        self.impact_rings.retain(|ring| ring.life > 0.0);
    }

    fn night_state(&self) -> (&'static str, &'static str, Color, f32) {
        let profile = night_profile(self.elapsed);
        (profile.title, profile.omen, profile.accent, profile.menace)
    }

    fn ritual_warning(&self) -> f32 {
        ritual_warning_level(self.elapsed)
    }

    fn ritual_window_countdown(&self) -> f32 {
        ritual_window_countdown(self.elapsed)
    }

    fn ritual_time_remaining(&self) -> f32 {
        ritual_time_remaining(self.elapsed)
    }

    fn is_ritual_window(&self) -> bool {
        is_ritual_window(self.elapsed)
    }

    fn flavor_vector(&self) -> &'static str {
        let profile = night_profile(self.elapsed);
        let pressures = self.combat_pressures();
        let time_shift = (self.elapsed / (FLAVOR_SHIFT_SECS + 1.0)).floor() as usize;
        let phase_shift = phase_bucket(self.elapsed) as usize * 23;
        let wave_step = (self.elapsed / ENEMY_WAVE_WINDOW_SECS).floor() as usize;
        let tempo_step = (pressures.tempo * 6.0) as usize;
        let cluster_step = (self.elapsed / ENEMY_CLUSTER_WINDOW_SECS).floor() as usize;
        let pulse_step = (pressures.time_pulse * 7.0) as usize;
        let omen_shift = profile.omen.len();
        let threat_step = ((pressures.wave
            + pressures.enrage * 1.2
            + pressures.cluster * 0.7
            + pressures.tempo * 0.6
            + pressures.time_pulse * 1.1)
            * 2.0) as usize;
        let bank: &[&str] = if self.is_ritual_window() || self.ritual_warning() > 0.0 {
            FLAVOR_RITUAL
        } else if pressures.enrage > 0.35 {
            FLAVOR_TENSION
        } else if pressures.tempo > 0.52 {
            FLAVOR_TEMPO
        } else {
            &profile.vector
        };

        let slot = (time_shift
            + phase_shift
            + wave_step
            + tempo_step
            + cluster_step
            + pulse_step
            + omen_shift
            + threat_step)
            % bank.len();
        bank[slot]
    }

    fn combat_pressures(&self) -> CombatPulse {
        let elapsed = self.elapsed;
        CombatPulse {
            wave: enemy_wave_pressure(elapsed),
            beat: enemy_beat_pressure(elapsed),
            tempo: enemy_tempo_pressure(elapsed),
            cluster: enemy_cluster_pressure(elapsed),
            enrage: enemy_enrage_pressure(elapsed),
            time_pulse: enemy_time_pressure(elapsed),
        }
    }

    fn burst(&mut self, position: Vec2, color: Color, count: usize, speed: f32) {
        for _ in 0..count {
            let life = rand::gen_range(0.16, 0.34);
            self.particles.push(Particle {
                position,
                velocity: Vec2::from_angle(rand::gen_range(0.0, TAU))
                    * rand::gen_range(speed * 0.45, speed),
                life,
                max_life: life,
                size: rand::gen_range(2.0, 5.0),
                color,
            });
        }
    }

    fn impact_ring(&mut self, position: Vec2, magnitude: f32, color: Color) {
        self.impact_rings.push(ImpactRing {
            position,
            life: IMPACT_RING_LIFE,
            max_radius: 7.0 + magnitude * 0.36,
            color,
        });
    }

    fn damage_number(&mut self, position: Vec2, damage: f32, color: Color) {
        self.damage_numbers.push(DamageNumber {
            position,
            value: damage.round() as u32,
            life: DAMAGE_NUMBER_LIFE,
            color,
        });
    }
}
fn night_profile(elapsed: f32) -> NightProfile {
    let profile = phase_bucket(elapsed) as usize;
    NIGHT_PROFILES[profile.min(NIGHT_PROFILES.len() - 1)]
}

fn chain_targets(enemies: &[Enemy], origin: Vec2, initial_range: f32, count: usize) -> Vec<usize> {
    let mut targets = Vec::with_capacity(count);
    let mut current = origin;
    let mut range = initial_range;

    while targets.len() < count {
        let next = enemies
            .iter()
            .enumerate()
            .filter(|(index, _)| !targets.contains(index))
            .filter(|(_, enemy)| enemy.health > 0.0)
            .filter(|(_, enemy)| enemy.position.distance_squared(current) <= range * range)
            .min_by(|(_, left), (_, right)| {
                left.position
                    .distance_squared(current)
                    .total_cmp(&right.position.distance_squared(current))
            })
            .map(|(index, _)| index);
        let Some(next) = next else {
            break;
        };
        targets.push(next);
        current = enemies[next].position;
        range = initial_range * 0.62;
    }

    targets
}

fn random_offers() -> [Upgrade; OFFER_SLOT_COUNT] {
    let mut offers = [Upgrade::ExtraKnife; OFFER_SLOT_COUNT];
    let mut index = 0usize;
    let mut families = UpgradeFamily::ALL.to_vec();
    while index < offers.len() {
        if families.is_empty() {
            break;
        }
        let family_slot = rand::gen_range(0, families.len());
        let family = families.swap_remove(family_slot);

        if family == UpgradeFamily::Witch
            && offers[..index].iter().any(|offer| offer.is_witchcraft())
        {
            continue;
        }
        let options = Upgrade::family_pool(family);
        let candidate = options[rand::gen_range(0, options.len())];
        if !offers[..index].contains(&candidate) {
            offers[index] = candidate;
            index += 1;
        }
    }
    offers
}

#[allow(
    clippy::too_many_arguments,
    reason = "spawn_kind keeps gameplay tuning explicit and keeps call sites readable"
)]
fn spawn_kind(
    elapsed: f32,
    roll: f32,
    ritual_window: bool,
    enrage_pressure: f32,
    wave_pressure: f32,
    tempo_pressure: f32,
    beat_pressure: f32,
    timing_pressure: f32,
    cluster_pressure: f32,
    pulse_pressure: f32,
) -> EnemyKind {
    let phase = phase_bucket(elapsed);
    let roll = (roll
        + enrage_pressure * 0.22
        + wave_pressure * 0.18
        + tempo_pressure * 0.22
        + beat_pressure * 0.12
        + pulse_pressure * 0.09)
        .min(0.9999);
    let mut profile = spawn_profile(phase, ritual_window);
    let timing_factors = enemy_archetype_timing_factors(timing_pressure);
    if beat_pressure > 0.0 {
        for item in &mut profile {
            if matches!(
                item.0,
                EnemyKind::Brute | EnemyKind::Howl | EnemyKind::Revenant
            ) {
                item.1 *= 1.0 + beat_pressure * 0.14;
            }
        }
    }
    let mut total = 0.0;

    for item in &mut profile {
        let index = item.0.timing_index();
        item.1 *= timing_factors[index];
        item.1 *= enemy_kind_time_gate(timing_pressure, item.0);
        total += item.1;
    }
    if cluster_pressure > 0.0 {
        for item in &mut profile {
            if matches!(item.0, EnemyKind::Revenant | EnemyKind::Brute) {
                item.1 *= 1.0 + cluster_pressure * 0.22;
            }
        }
    }
    if pulse_pressure > 0.0 {
        for item in &mut profile {
            if matches!(item.0, EnemyKind::Brute | EnemyKind::Revenant) {
                item.1 *= 1.0 + pulse_pressure * 0.24;
            }
        }
    }

    let mut bucket = roll * total;
    for (kind, chance) in profile {
        if bucket < chance {
            return kind;
        }
        bucket -= chance;
    }

    profile[4].0
}

fn spawn_profile(phase: u8, ritual_window: bool) -> [(EnemyKind, f32); ENEMY_KIND_COUNT] {
    if ritual_window {
        match phase {
            0 => [
                (EnemyKind::Shade, 0.42),
                (EnemyKind::Wisp, 0.24),
                (EnemyKind::Howl, 0.13),
                (EnemyKind::Brute, 0.10),
                (EnemyKind::Revenant, 0.11),
            ],
            1 => [
                (EnemyKind::Shade, 0.30),
                (EnemyKind::Wisp, 0.30),
                (EnemyKind::Howl, 0.20),
                (EnemyKind::Brute, 0.14),
                (EnemyKind::Revenant, 0.06),
            ],
            2 => [
                (EnemyKind::Shade, 0.20),
                (EnemyKind::Wisp, 0.25),
                (EnemyKind::Howl, 0.25),
                (EnemyKind::Revenant, 0.21),
                (EnemyKind::Brute, 0.09),
            ],
            3 => [
                (EnemyKind::Shade, 0.08),
                (EnemyKind::Howl, 0.28),
                (EnemyKind::Wisp, 0.22),
                (EnemyKind::Revenant, 0.30),
                (EnemyKind::Brute, 0.12),
            ],
            _ => [
                (EnemyKind::Shade, 0.08),
                (EnemyKind::Howl, 0.22),
                (EnemyKind::Revenant, 0.26),
                (EnemyKind::Brute, 0.35),
                (EnemyKind::Wisp, 0.09),
            ],
        }
    } else {
        match phase {
            0 => [
                (EnemyKind::Shade, 0.64),
                (EnemyKind::Wisp, 0.24),
                (EnemyKind::Howl, 0.0),
                (EnemyKind::Brute, 0.12),
                (EnemyKind::Revenant, 0.0),
            ],
            1 => [
                (EnemyKind::Shade, 0.36),
                (EnemyKind::Wisp, 0.32),
                (EnemyKind::Howl, 0.19),
                (EnemyKind::Brute, 0.13),
                (EnemyKind::Revenant, 0.0),
            ],
            2 => [
                (EnemyKind::Shade, 0.26),
                (EnemyKind::Wisp, 0.28),
                (EnemyKind::Howl, 0.26),
                (EnemyKind::Brute, 0.11),
                (EnemyKind::Revenant, 0.09),
            ],
            3 => [
                (EnemyKind::Shade, 0.13),
                (EnemyKind::Howl, 0.28),
                (EnemyKind::Wisp, 0.18),
                (EnemyKind::Revenant, 0.25),
                (EnemyKind::Brute, 0.16),
            ],
            _ => [
                (EnemyKind::Shade, 0.08),
                (EnemyKind::Howl, 0.24),
                (EnemyKind::Revenant, 0.21),
                (EnemyKind::Brute, 0.29),
                (EnemyKind::Wisp, 0.18),
            ],
        }
    }
}

fn ritual_cycle(elapsed: f32) -> f32 {
    (elapsed % RITUAL_PERIOD_SECS + RITUAL_PERIOD_SECS) % RITUAL_PERIOD_SECS
}

fn is_ritual_window(elapsed: f32) -> bool {
    let cycle = ritual_cycle(elapsed);
    cycle >= RITUAL_WARNING_START_SECS + RITUAL_WARNING_SECS
}

fn ritual_warning_level(elapsed: f32) -> f32 {
    let cycle = ritual_cycle(elapsed);
    if (RITUAL_WARNING_START_SECS..RITUAL_WARNING_START_SECS + RITUAL_WARNING_SECS).contains(&cycle)
    {
        1.0f32.min((cycle - RITUAL_WARNING_START_SECS) / RITUAL_WARNING_SECS)
    } else {
        0.0
    }
}

fn ritual_window_countdown(elapsed: f32) -> f32 {
    let cycle = ritual_cycle(elapsed);
    if cycle < RITUAL_WARNING_START_SECS {
        RITUAL_WARNING_START_SECS - cycle
    } else if cycle < RITUAL_WARNING_START_SECS + RITUAL_WARNING_SECS {
        RITUAL_WARNING_START_SECS + RITUAL_WARNING_SECS - cycle
    } else {
        0.0
    }
}

fn ritual_time_remaining(elapsed: f32) -> f32 {
    let cycle = ritual_cycle(elapsed);
    if is_ritual_window(elapsed) {
        RITUAL_PERIOD_SECS - cycle
    } else {
        0.0
    }
}

fn enemy_enrage_pressure(elapsed: f32) -> f32 {
    let cycle = (elapsed % ENRAGE_WAVE_SECS + ENRAGE_WAVE_SECS) % ENRAGE_WAVE_SECS;
    if cycle >= ENRAGE_WINDOW_SECS {
        return 0.0;
    }

    let pulse = cycle / ENRAGE_WINDOW_SECS;
    let rise = (pulse / 0.33).min(1.0);
    let fall = ((pulse - 0.67) / 0.33).max(0.0);
    (rise.min(1.0 - fall)).powf(1.15)
}

fn enemy_wave_pressure(elapsed: f32) -> f32 {
    let cycle =
        (elapsed % ENEMY_WAVE_PERIOD_SECS + ENEMY_WAVE_PERIOD_SECS) % ENEMY_WAVE_PERIOD_SECS;
    let window_half = ENEMY_WAVE_WINDOW_SECS * 0.5;
    let center = ENEMY_WAVE_PERIOD_SECS * 0.5;
    let distance = (cycle - center).abs();
    if distance >= window_half {
        0.0
    } else {
        (1.0 - distance / window_half).powf(1.3)
    }
}

fn enemy_tempo_pressure(elapsed: f32) -> f32 {
    let cycle =
        (elapsed % ENEMY_TEMPO_PERIOD_SECS + ENEMY_TEMPO_PERIOD_SECS) % ENEMY_TEMPO_PERIOD_SECS;
    let rise = (ENEMY_TEMPO_PERIOD_SECS * 0.2).max(0.001);
    let fall_start = ENEMY_TEMPO_PERIOD_SECS - rise;
    let pressure = if cycle < rise {
        (cycle / rise).powf(1.3)
    } else if cycle < fall_start {
        1.0
    } else {
        ((ENEMY_TEMPO_PERIOD_SECS - cycle) / rise).powf(1.3)
    };
    (pressure * ENEMY_TEMPO_PEAK_SHIFT).clamp(0.0, 1.0)
}

fn enemy_time_pressure(elapsed: f32) -> f32 {
    let cycle =
        (elapsed % ENEMY_PULSE_PERIOD_SECS + ENEMY_PULSE_PERIOD_SECS) % ENEMY_PULSE_PERIOD_SECS;
    let center = ENEMY_PULSE_PERIOD_SECS * 0.5;
    let window = (ENEMY_PULSE_WINDOW_SECS * 0.5).max(0.001);
    let distance = (cycle - center).abs();
    if distance >= window {
        0.0
    } else {
        (1.0 - distance / window).powf(1.35)
    }
}

fn enemy_beat_pressure(elapsed: f32) -> f32 {
    let cycle =
        (elapsed % ENEMY_BEAT_PERIOD_SECS + ENEMY_BEAT_PERIOD_SECS) % ENEMY_BEAT_PERIOD_SECS;
    let center = ENEMY_BEAT_PERIOD_SECS * 0.5;
    let distance = (cycle - center).abs();
    let half_window = ENEMY_BEAT_WINDOW_SECS * 0.5;
    if distance >= half_window {
        0.0
    } else {
        (1.0 - distance / half_window).powf(1.45)
    }
}

fn enemy_cluster_pressure(elapsed: f32) -> f32 {
    let cycle = (elapsed % ENEMY_CLUSTER_PERIOD_SECS + ENEMY_CLUSTER_PERIOD_SECS)
        % ENEMY_CLUSTER_PERIOD_SECS;
    let center = ENEMY_CLUSTER_PERIOD_SECS * 0.55;
    let span = ENEMY_CLUSTER_WINDOW_SECS * 0.5;
    let distance = (cycle - center).abs();
    if distance >= span {
        0.0
    } else {
        (1.0 - distance / span).powf(1.25)
    }
}

fn enemy_spawn_funnel(elapsed: f32) -> f32 {
    let drift = (elapsed / ENEMY_ORIGIN_SPIN_SECS) * TAU;
    let tide = (enemy_timing_pressure(elapsed) - 0.5) * 0.8;
    drift + tide
}

fn enemy_spawn_lane_index(elapsed: f32) -> usize {
    if elapsed <= 0.0 {
        return 0;
    }

    let lane_window = (elapsed / ENEMY_LANE_CYCLE_SECS).floor();
    if lane_window.is_finite() {
        (lane_window as usize) % ENEMY_LANE_COUNT
    } else {
        0
    }
}

fn enemy_spawn_lane_center(elapsed: f32) -> f32 {
    let index = enemy_spawn_lane_index(elapsed) as f32;
    let base = index / ENEMY_LANE_COUNT as f32 * TAU;
    let drift = (elapsed / ENEMY_LANE_SWAY_SECS).sin() * 0.28
        + (enemy_spawn_funnel(elapsed) * 0.05).sin() * 0.12;
    (base + drift).rem_euclid(TAU)
}

fn enemy_spawn_direction_pressure(elapsed: f32) -> f32 {
    let cluster = enemy_cluster_pressure(elapsed);
    let beat = enemy_beat_pressure(elapsed);
    let enrage = enemy_enrage_pressure(elapsed);
    let tempo = enemy_tempo_pressure(elapsed);
    (cluster * 0.72 + beat * 0.48 + enrage * 0.55 + tempo * 0.15).clamp(0.0, 1.0)
}

fn enemy_spawn_lane_span(elapsed: f32, lane_pressure: f32) -> f32 {
    let lane_pressure = lane_pressure.clamp(0.0, 1.0);
    let lane_width = TAU / ENEMY_LANE_COUNT as f32;
    let sway = ((elapsed / ENEMY_LANE_SWAY_SECS).sin() * 0.5 + 0.5) * ENEMY_LANE_WIDTH_SWAY;
    let modulation = (1.0 - lane_pressure * 0.55 + sway).max(0.0);
    lane_width * modulation.max(0.20)
}

fn enemy_timing_pressure(elapsed: f32) -> f32 {
    (elapsed / ENEMY_TIMING_PERIOD_SECS).fract()
}

fn enemy_timing_spawn_pressure(elapsed: f32) -> f32 {
    (enemy_timing_pressure(elapsed) * TAU).sin().abs()
}

fn enemy_kind_time_gate(pressure: f32, kind: EnemyKind) -> f32 {
    if pressure <= 0.0 {
        return 1.0;
    }

    let index = kind.timing_index();
    let phase = pressure * TAU + ENEMY_TIMING_PHASE_OFFSETS[index];
    1.0 + phase.sin() * (ENEMY_KIND_TIME_SWAY * 0.5)
}

fn enemy_kind_movement_gate(elapsed: f32, kind: EnemyKind) -> f32 {
    let index = kind.timing_index();
    let phase = elapsed * 0.88 + ENEMY_TIMING_PHASE_OFFSETS[index];
    1.0 + phase.sin() * (ENEMY_KIND_MOVEMENT_SWAY * 0.5)
}

fn enemy_archetype_timing_factors(pressure: f32) -> [f32; ENEMY_KIND_COUNT] {
    if pressure <= 0.0 {
        return [1.0; ENEMY_KIND_COUNT];
    }

    let phase = pressure * TAU;
    let mut factors = [0.0; ENEMY_KIND_COUNT];
    for index in 0..ENEMY_KIND_COUNT {
        let wave = (phase + ENEMY_TIMING_PHASE_OFFSETS[index]).sin() * 0.5 + 0.5;
        factors[index] = 1.0 + (wave - 0.5) * ENEMY_TIMING_STRENGTH;
    }
    factors
}

fn phase_bucket(elapsed: f32) -> u8 {
    match elapsed {
        t if t < 20.0 => 0,
        t if t < 45.0 => 1,
        t if t < 70.0 => 2,
        t if t < 95.0 => 3,
        t if t < 120.0 => 4,
        _ => 5,
    }
}

fn spawn_interval(elapsed: f32) -> f32 {
    let base = (0.64 - elapsed * 0.0038).max(0.14);
    let profile = night_profile(elapsed);
    let timing_wave = enemy_timing_spawn_pressure(elapsed);
    let timing_pressure = 1.0 - timing_wave * ENEMY_TIMING_SPAWN_SWAY;
    (base * profile.spawn_pressure * (1.0 + profile.health_scale * 0.02) * timing_pressure)
        .max(0.14)
}

fn spawn_distance(width: f32) -> f32 {
    // Half the viewport diagonal clears every edge; the margin hides pop-in.
    width.hypot(VIEW_HEIGHT) * 0.5 + 54.0
}

fn view_width() -> f32 {
    // The world stays 720 units tall while widening with the window's aspect ratio.
    VIEW_HEIGHT * screen_width() / screen_height().max(1.0)
}

fn smoothing_weight(rate: f32, dt: f32) -> f32 {
    // Exponential smoothing converges at the same rate regardless of frame rate.
    1.0 - (-rate * dt).exp()
}

fn enemy_to_line_distance(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let segment_len2 = segment.length_squared();
    if segment_len2 < 0.01 {
        return point.distance(start);
    }
    let projection = ((point - start).dot(segment) / segment_len2).clamp(0.0, 1.0);
    let closest = start + segment * projection;
    point.distance(closest)
}

fn format_time(seconds: f32) -> String {
    let total = seconds as u32;
    format!("{:02}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enemy_at(x: f32, y: f32) -> Enemy {
        Enemy {
            kind: EnemyKind::Shade,
            position: vec2(x, y),
            velocity: Vec2::ZERO,
            health: 10.0,
            radius: 10.0,
            flash: 0.0,
            moon_immunity: 0.0,
            phase: 0.0,
        }
    }

    fn player_y_after_one_second(movement_y: f32, frame_rate: usize) -> f32 {
        let mut game = Game::new_running();
        game.spawn_timer = f32::INFINITY;
        let input = Input {
            movement: vec2(0.0, movement_y),
            ..Input::default()
        };

        for _ in 0..frame_rate {
            assert_eq!(
                game.update(1.0 / frame_rate as f32, input),
                Control::Continue
            );
        }

        game.player.position.y
    }

    fn enemy_gap_after_50_ms(frame_rate: usize) -> f32 {
        let mut enemies = [enemy_at(-4.0, 0.0), enemy_at(4.0, 0.0)];
        for _ in 0..frame_rate / 20 {
            Game::separate_enemies(&mut enemies, 1.0 / frame_rate as f32);
        }

        enemies[0].position.distance(enemies[1].position)
    }

    #[test]
    fn lightning_chains_through_nearest_living_enemies_without_repeats() {
        let enemies = [
            Enemy {
                health: 0.0,
                ..enemy_at(50.0, 0.0)
            },
            enemy_at(100.0, 0.0),
            enemy_at(180.0, 0.0),
            enemy_at(260.0, 0.0),
            enemy_at(500.0, 0.0),
        ];

        assert_eq!(chain_targets(&enemies, Vec2::ZERO, 200.0, 4), [1, 2, 3]);
    }

    #[test]
    fn extra_knife_adds_one_blade_without_changing_storm() {
        let mut game = Game::new_running();
        let storm_before = game.storm;

        game.apply_upgrade(Upgrade::ExtraKnife);

        assert_eq!(game.moon.count, 3);
        assert!((game.storm.damage - storm_before.damage).abs() < f32::EPSILON);
        assert_eq!(game.storm.jumps, storm_before.jumps);
    }

    #[test]
    fn time_is_always_formatted_as_minutes_and_seconds() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(125.9), "02:05");
    }

    #[test]
    fn spawn_pacing_starts_nearby_and_accelerates_to_a_safe_floor() {
        let half_diagonal = 1280.0_f32.hypot(VIEW_HEIGHT) * 0.5;

        assert!((spawn_interval(0.0) - 0.64).abs() < f32::EPSILON);
        assert!(spawn_interval(60.0) < spawn_interval(0.0));
        assert!((spawn_interval(10_000.0) - 0.14).abs() < f32::EPSILON);
        assert!(spawn_distance(1280.0) > half_diagonal);
        assert!(spawn_distance(1280.0) < half_diagonal + 100.0);
    }

    #[test]
    fn spawn_interval_pulses_with_timing_wave() {
        let base = spawn_interval(0.0);
        let surge = spawn_interval(ENEMY_TIMING_PERIOD_SECS * 0.25);
        let drift = spawn_interval(ENEMY_TIMING_PERIOD_SECS * 0.45);

        assert!(surge < base);
        assert!((drift - surge).abs() > 0.001);
    }

    #[test]
    fn movement_direction_and_distance_are_stable_at_high_frame_rates() {
        let down_at_60_hz = player_y_after_one_second(1.0, 60);
        let down_at_400_hz = player_y_after_one_second(1.0, 400);
        let up_at_400_hz = player_y_after_one_second(-1.0, 400);

        assert!(down_at_400_hz > 0.0);
        assert!(up_at_400_hz < 0.0);
        assert!((down_at_60_hz - down_at_400_hz).abs() < 3.0);
        assert!((down_at_400_hz + up_at_400_hz).abs() < f32::EPSILON);
    }

    #[test]
    fn enemy_separation_is_stable_at_high_frame_rates() {
        let gap_at_60_hz = enemy_gap_after_50_ms(60);
        let gap_at_400_hz = enemy_gap_after_50_ms(400);

        assert!((gap_at_60_hz - gap_at_400_hz).abs() < 0.01);
    }

    #[test]
    fn night_state_progresses_in_expected_phases() {
        let mut game = Game::new();
        let early = game.night_state();
        game.elapsed = 35.0;
        let middle = game.night_state();
        game.elapsed = 70.0;
        let late = game.night_state();
        game.elapsed = 140.0;
        let final_state = game.night_state();

        assert_ne!(early.0, middle.0);
        assert_ne!(middle.0, late.0);
        assert_ne!(late.0, final_state.0);
        assert!((late.3 - early.3).abs() > f32::EPSILON);
    }

    #[test]
    fn deeper_night_states_bring_greater_threat() {
        let early = spawn_interval(0.0);
        let first_waning = spawn_interval(30.0);
        let moonless = spawn_interval(60.0);
        let late = spawn_interval(95.0);
        let abyss = spawn_interval(150.0);

        assert!(first_waning < early);
        assert!(moonless <= first_waning);
        assert!(late <= moonless);
        assert!(abyss <= late);
    }

    #[test]
    fn flavor_vectors_cycle_with_time() {
        let mut game = Game::new_running();
        let first = game.flavor_vector();
        game.elapsed = FLAVOR_SHIFT_SECS + 1.0;
        let second = game.flavor_vector();

        assert_ne!(first, second);
    }

    #[test]
    fn phase_transition_sets_flash_timer() {
        let mut game = Game::new_running();
        assert_eq!(game.phase_bucket, 0);
        assert!(game.phase_flash.abs() < f32::EPSILON);

        game.elapsed = 21.0;
        game.update_phase_progress();

        assert_eq!(game.phase_bucket, 1);
        assert!((game.phase_flash - PHASE_FLASH_TIME).abs() < f32::EPSILON);
    }

    #[test]
    fn wraith_lash_respects_strike_cap() {
        let mut game = Game::new_running();
        game.wraith_lash = WraithLash {
            level: 1,
            cooldown: 0.8,
            timer: 0.0,
            range: 255.0,
            width: 12.0,
            damage: 18.0,
        };
        game.enemies = [
            Enemy {
                kind: EnemyKind::Revenant,
                position: vec2(80.0, 0.0),
                health: 200.0,
                ..enemy_at(0.0, 0.0)
            },
            Enemy {
                kind: EnemyKind::Shade,
                position: vec2(160.0, 0.0),
                health: 200.0,
                ..enemy_at(0.0, 0.0)
            },
            Enemy {
                kind: EnemyKind::Wisp,
                position: vec2(240.0, 0.0),
                health: 200.0,
                ..enemy_at(0.0, 0.0)
            },
        ]
        .into_iter()
        .collect();
        game.resolve_wraith_lash();

        let damaged = game
            .enemies
            .iter()
            .filter(|enemy| enemy.health < 200.0)
            .count();
        assert_eq!(damaged, 1);
        assert!((game.wraith_lash.timer - game.wraith_lash.cooldown).abs() < f32::EPSILON);
    }

    #[test]
    fn ritual_windows_pulse_periodically() {
        let warning_anchor = RITUAL_WARNING_START_SECS + RITUAL_WARNING_SECS * 0.5;
        let active_anchor = RITUAL_WARNING_START_SECS + RITUAL_WARNING_SECS + 0.10;

        assert!(!is_ritual_window(warning_anchor));
        assert!(ritual_warning_level(warning_anchor) > 0.45);
        assert!(ritual_warning_level(warning_anchor + 0.1) > ritual_warning_level(warning_anchor));
        assert!(is_ritual_window(active_anchor));
        assert!(ritual_time_remaining(active_anchor) < RITUAL_ACTIVE_SECS);
        assert!((ritual_time_remaining(active_anchor) - (RITUAL_ACTIVE_SECS - 0.10)).abs() < 0.01);
        assert!(ritual_window_countdown(active_anchor) < 0.01);
    }

    #[test]
    fn ritual_mode_shifts_enemy_mix_toward_heavy_types() {
        let quiet = spawn_kind(150.0, 0.55, false, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let ritual = spawn_kind(150.0, 0.55, true, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let tempo_only = spawn_kind(
            150.0,
            0.95,
            false,
            0.0,
            0.0,
            enemy_tempo_pressure(150.0),
            0.0,
            0.0,
            0.0,
            0.0,
        );

        assert_eq!(quiet, EnemyKind::Brute);
        assert_eq!(ritual, EnemyKind::Revenant);
        assert_ne!(tempo_only, EnemyKind::Revenant);
    }

    #[test]
    fn enemy_enrage_windows_push_spawn_intensity() {
        assert!((enemy_enrage_pressure(17.4) - 0.0).abs() < f32::EPSILON);
        assert!(
            enemy_enrage_pressure(2.0) > enemy_enrage_pressure(6.0),
            "wave should ramp then decay"
        );
        assert!(enemy_enrage_pressure(2.0) > 0.1);
        assert!(enemy_enrage_pressure(17.9) < 0.1);

        let calm = spawn_kind(
            150.0,
            0.30,
            false,
            0.0,
            enemy_wave_pressure(150.0),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let enraged = spawn_kind(
            150.0,
            0.30,
            false,
            enemy_enrage_pressure(2.0),
            enemy_wave_pressure(150.0),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert_ne!(calm, enraged);
        assert_eq!(calm, EnemyKind::Howl);
    }

    #[test]
    fn enemy_wave_pressure_forms_midnight_tide_spikes() {
        assert!((enemy_wave_pressure(0.0) - 0.0).abs() < f32::EPSILON);
        assert!(
            enemy_wave_pressure(ENEMY_WAVE_PERIOD_SECS * 0.5) > 0.97,
            "enemy wave should crest at the center of every pressure window",
        );
        assert!(
            (enemy_wave_pressure(ENEMY_WAVE_PERIOD_SECS * 0.5 + ENEMY_WAVE_WINDOW_SECS * 0.5))
                < 0.02
        );
        assert!(
            (enemy_wave_pressure(ENEMY_WAVE_PERIOD_SECS * 0.5 - ENEMY_WAVE_WINDOW_SECS * 0.5))
                < 0.02
        );
    }

    #[test]
    fn enemy_cluster_pressure_forms_repeated_bursts() {
        let peak = enemy_cluster_pressure(ENEMY_CLUSTER_PERIOD_SECS * 0.55);
        assert!(peak > 0.97);
        assert!((enemy_cluster_pressure(ENEMY_CLUSTER_PERIOD_SECS * 0.02)).abs() < 0.05);
    }

    #[test]
    fn enemy_time_pressure_forms_short_tight_pulses() {
        let peak = ENEMY_PULSE_PERIOD_SECS * 0.5;
        assert!((enemy_time_pressure(0.0) - 0.0).abs() < f32::EPSILON);
        assert!(enemy_time_pressure(peak) > 0.97);
        assert!((enemy_time_pressure(peak + ENEMY_PULSE_WINDOW_SECS * 0.45)) < 0.05);
        assert!(enemy_time_pressure(peak + ENEMY_PULSE_WINDOW_SECS * 0.2) > 0.45);
    }

    #[test]
    fn enemy_spawn_lane_cycles_and_direction_pressure_varies() {
        assert_eq!(enemy_spawn_lane_index(0.0), 0);
        assert_eq!(enemy_spawn_lane_index(ENEMY_LANE_CYCLE_SECS * 0.9), 0);
        assert_eq!(enemy_spawn_lane_index(ENEMY_LANE_CYCLE_SECS * 1.1), 1);
        assert_eq!(
            enemy_spawn_lane_index(ENEMY_LANE_CYCLE_SECS * 5.0 + 0.12),
            5
        );

        let calm = enemy_spawn_direction_pressure(ENEMY_CLUSTER_PERIOD_SECS * 0.02);
        let active = enemy_spawn_direction_pressure(ENEMY_CLUSTER_PERIOD_SECS * 0.55);
        assert!(active >= calm);
        assert!(
            (enemy_spawn_lane_span(1.0, calm) - enemy_spawn_lane_span(1.0, active)).abs()
                > f32::EPSILON
        );
        assert!(enemy_spawn_lane_span(1.0, active) > 0.0);
    }

    #[test]
    fn enemy_spawn_funnel_remains_directional_and_time_based() {
        let first = enemy_spawn_funnel(37.0);
        let second = enemy_spawn_funnel(37.0 + ENEMY_ORIGIN_SPIN_SECS / 8.0);
        assert!((second - first).abs() > 0.001);
        assert!(first.is_finite());
        assert!(second.is_finite());
        assert!((enemy_spawn_funnel(130.0)).abs() > 0.0);
    }

    #[test]
    fn spawn_kind_shifts_with_wave_pressure() {
        let tide_elapsed = 149.5 + ENEMY_WAVE_PERIOD_SECS * 0.5;
        let calm = spawn_kind(150.0, 0.80, false, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let wave = spawn_kind(
            tide_elapsed,
            0.80,
            false,
            0.0,
            enemy_wave_pressure(tide_elapsed),
            enemy_tempo_pressure(tide_elapsed),
            0.0,
            0.0,
            0.0,
            0.0,
        );

        assert_ne!(calm, wave, "wave pressure should alter enemy mix");
        assert!(enemy_wave_pressure(tide_elapsed) > 0.97);
    }

    #[test]
    fn spawn_kind_has_timing_profile_influence() {
        let calm = spawn_kind(
            130.0,
            0.06,
            false,
            0.0,
            enemy_wave_pressure(130.0),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        let timed = spawn_kind(
            130.0,
            0.06,
            false,
            0.0,
            enemy_wave_pressure(130.0),
            0.0,
            0.77,
            0.0,
            0.0,
            0.0,
        );
        assert_ne!(calm, timed);
    }

    #[test]
    fn spawn_kind_time_pressure_pushes_heavy_archetypes() {
        let mut calm = 0usize;
        let mut pulsed = 0usize;
        let base_tempo = enemy_tempo_pressure(120.0);
        let base_wave = enemy_wave_pressure(120.0);
        for tick in 0..180 {
            let roll = tick as f32 / 180.0;
            let calm_pick = spawn_kind(
                120.0, roll, false, 0.0, base_wave, base_tempo, 0.0, 0.0, 0.0, 0.0,
            );
            let pulsed_pick = spawn_kind(
                120.0,
                roll,
                false,
                0.0,
                base_wave,
                base_tempo,
                0.0,
                0.0,
                0.0,
                enemy_time_pressure(ENEMY_PULSE_PERIOD_SECS * 0.5),
            );
            if matches!(calm_pick, EnemyKind::Brute | EnemyKind::Revenant) {
                calm += 1;
            }
            if matches!(pulsed_pick, EnemyKind::Brute | EnemyKind::Revenant) {
                pulsed += 1;
            }
        }
        assert!(pulsed > calm);
    }

    #[test]
    fn enemy_archetype_timing_factors_oscillate_between_archetypes() {
        let neutral = enemy_archetype_timing_factors(0.0);
        let shifted = enemy_archetype_timing_factors(0.77);
        assert!(neutral.iter().all(|value| (*value - 1.0).abs() < 0.0001));
        assert!(shifted.iter().any(|value| (*value - 1.0).abs() > 0.05));
    }

    #[test]
    fn enemy_kind_time_gate_varies_by_kind_and_pressure() {
        let shade_a = enemy_kind_time_gate(0.16, EnemyKind::Shade);
        let howl_a = enemy_kind_time_gate(0.16, EnemyKind::Howl);
        let revenant_a = enemy_kind_time_gate(0.16, EnemyKind::Revenant);
        let shifted = enemy_kind_time_gate(0.63, EnemyKind::Shade);

        assert!((shade_a - howl_a).abs() > 0.0001);
        assert!((shade_a - revenant_a).abs() > 0.0001);
        assert!((shade_a - shifted).abs() > 0.0001);
        assert!((shade_a - 1.0).abs() <= 0.13);
    }

    #[test]
    fn enemy_kind_movement_gate_changes_with_kind_phase() {
        let shade = enemy_kind_movement_gate(84.0, EnemyKind::Shade);
        let howl = enemy_kind_movement_gate(84.0, EnemyKind::Howl);
        let brutish = enemy_kind_movement_gate(84.0, EnemyKind::Brute);

        assert!(shade.is_finite());
        assert!(howl.is_finite());
        assert!(brutish.is_finite());
        assert!((shade - howl).abs() > 0.0001);
        assert!((howl - brutish).abs() > 0.0001);
    }

    #[test]
    fn spawn_profile_weights_are_normalized_and_unique() {
        for phase in 0_u8..6 {
            let base = spawn_profile(phase, false);
            let ritual = spawn_profile(phase, true);

            for profile in [base, ritual] {
                let mut seen = [false; ENEMY_KIND_COUNT];
                let mut total = 0.0;
                for (kind, weight) in profile {
                    let index = kind.timing_index();
                    assert!(
                        !seen[index],
                        "duplicate archetype in spawn profile for phase {phase}"
                    );
                    seen[index] = true;
                    assert!(weight >= 0.0, "negative weight in spawn profile");
                    total += weight;
                }
                assert!((total - 1.0).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn grave_mine_variants_bootstrap_when_first_picked() {
        let mut via_rage = Game::new_running();
        via_rage.apply_upgrade(Upgrade::GraveMineRage);
        assert!(via_rage.grave_mines.unlocked());
        assert_eq!(via_rage.grave_mines.shard_count, 3);
        assert!(via_rage.grave_mines.damage > 26.0);

        let mut via_reach = Game::new_running();
        via_reach.apply_upgrade(Upgrade::GraveMineReach);
        assert!(via_reach.grave_mines.unlocked());
        assert!(via_reach.grave_mines.shard_count >= 3);
        assert!(via_reach.grave_mines.shard_count > via_rage.grave_mines.shard_count);
        assert!(via_reach.grave_mines.blast_radius > 76.0);
    }

    #[test]
    fn upgrade_level_progression_is_consistent_across_family_variants() {
        let mut grave = Game::new_running();
        grave.apply_upgrade(Upgrade::GraveMineReach);
        grave.apply_upgrade(Upgrade::GraveMineRage);
        assert_eq!(grave.grave_mines.level, 3);
        assert_eq!(grave.grave_mines.shard_count, 4);

        let mut flare = Game::new_running();
        flare.apply_upgrade(Upgrade::AstralFlareBloom);
        flare.apply_upgrade(Upgrade::AstralFlarePulse);
        assert_eq!(flare.astral_flare.level, 3);

        let mut wraith = Game::new_running();
        wraith.apply_upgrade(Upgrade::WraithLashReach);
        wraith.apply_upgrade(Upgrade::WraithLashRend);
        assert_eq!(wraith.wraith_lash.level, 3);

        let mut harrow = Game::new_running();
        harrow.apply_upgrade(Upgrade::HarrowVolleyAim);
        harrow.apply_upgrade(Upgrade::HarrowVolleyPierce);
        assert_eq!(harrow.harrow_volley.level, 3);
        assert_eq!(harrow.harrow_volley.dart_count, 3);

        let mut aether = Game::new_running();
        aether.apply_upgrade(Upgrade::AetherSpearSplit);
        aether.apply_upgrade(Upgrade::AetherSpearRage);
        assert_eq!(aether.aether_spears.level, 3);
        assert_eq!(aether.aether_spears.spear_count, 3);
        assert_eq!(aether.aether_spears.pierce, 2);

        let mut rift = Game::new_running();
        rift.apply_upgrade(Upgrade::RiftPulseAnchor);
        rift.apply_upgrade(Upgrade::RiftPulseCascade);
        assert_eq!(rift.rift_pulse.level, 3);
        assert!((rift.rift_pulse.radius - 116.0).abs() < f32::EPSILON);
        assert!((rift.rift_pulse.duration - 1.06).abs() < 0.001);

        let mut echo = Game::new_running();
        echo.apply_upgrade(Upgrade::EchoCannonResonance);
        echo.apply_upgrade(Upgrade::EchoCannonCataclysm);
        assert_eq!(echo.echo_cannon.level, 3);
        assert_eq!(echo.echo_cannon.shard_count, 4);
        assert_eq!(echo.echo_cannon.bounces, 1);
        assert!((echo.echo_cannon.speed - 363.0).abs() < f32::EPSILON);

        let mut ward = Game::new_running();
        ward.apply_upgrade(Upgrade::RuneWardEcho);
        ward.apply_upgrade(Upgrade::RuneWardCataclysm);
        assert_eq!(ward.rune_wards.level, 3);
        assert_eq!(ward.rune_wards.ward_count, 3);

        let mut halo = Game::new_running();
        halo.apply_upgrade(Upgrade::CrescentHaloSpiral);
        halo.apply_upgrade(Upgrade::CrescentHaloCataclysm);
        assert_eq!(halo.crescent_halo.level, 3);
        assert_eq!(halo.crescent_halo.blades, 7);
        assert!(halo.crescent_halo.radius > 132.0);

        let mut bloom = Game::new_running();
        bloom.apply_upgrade(Upgrade::VoidBloomReach);
        bloom.apply_upgrade(Upgrade::VoidBloomCascade);
        assert_eq!(bloom.void_bloom.level, 3);
        assert_eq!(bloom.void_bloom.seed_count, 5);

        let mut solar = Game::new_running();
        solar.apply_upgrade(Upgrade::SolarNovaBloom);
        solar.apply_upgrade(Upgrade::SolarNovaCataclysm);
        assert_eq!(solar.solar_nova.level, 3);
        assert!(solar.solar_nova.rings >= 4);

        let mut abyssal = Game::new_running();
        abyssal.apply_upgrade(Upgrade::AbyssalMineReach);
        abyssal.apply_upgrade(Upgrade::AbyssalMineRage);
        assert_eq!(abyssal.abyssal_mines.level, 3);
        assert!(abyssal.abyssal_mines.mine_count >= 4);
    }

    #[test]
    fn random_offers_keep_weapon_family_variety() {
        for _ in 0..256 {
            let offers = random_offers();
            let mut family_counts = [0u8; OFFER_FAMILY_COUNT];
            let mut unique_families = 0u8;
            for offer in offers {
                let family = offer.family() as u8 as usize;
                if family_counts[family] == 0 {
                    unique_families += 1;
                }
                family_counts[family] += 1;

                assert_eq!(family_counts[family], 1);
            }
            assert_eq!(unique_families, OFFER_SLOT_COUNT as u8);
            assert_eq!(
                family_counts
                    .iter()
                    .copied()
                    .filter(|count| *count > 0)
                    .count() as u8,
                OFFER_SLOT_COUNT as u8,
            );
            assert_eq!(
                family_counts.iter().copied().sum::<u8>(),
                OFFER_SLOT_COUNT as u8
            );
        }
    }
}
