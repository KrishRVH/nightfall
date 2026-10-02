//! Game state and frame-by-frame simulation.
//!
//! [`Game::update`] is the central teaching path: it applies input, enemy AI,
//! weapon damage, deaths, pickups, and effects in an explicit order. Drawing
//! lives in the private `render` child module and only reads simulation state.

mod render;

pub(crate) use render::TextCache;

use std::f32::consts::TAU;

use macroquad::{
    prelude::{Color, Vec2, vec2},
    rand::RandGenerator,
};

const VIEW_HEIGHT: f32 = 720.0;
const PLAYER_RADIUS: f32 = 18.0;
const MAX_ENEMIES: usize = 360;
const DAMAGE_NUMBER_LIFE: f32 = 0.62;
const LIGHTNING_LIFE: f32 = 0.18;
const PICKUP_FLASH_LIFE: f32 = 0.24;
const ENEMY_SEPARATION_RATE: f32 = 27.0;

const INK: Color = Color::new(0.07, 0.055, 0.12, 1.0);
const BACKGROUND: Color = Color::new(0.055, 0.045, 0.10, 1.0);
const BONE: Color = Color::new(0.96, 0.91, 0.79, 1.0);
const MOON_GOLD: Color = Color::new(1.0, 0.74, 0.25, 1.0);
const ARCANE_VIOLET: Color = Color::new(0.58, 0.38, 0.98, 1.0);
const STORM_CYAN: Color = Color::new(0.31, 0.88, 0.96, 1.0);
const DAMAGE_RED: Color = Color::new(0.94, 0.25, 0.32, 1.0);
const GEM_GREEN: Color = Color::new(0.35, 0.87, 0.52, 1.0);

/// A frame of device input translated into game concepts.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Input actions are independent button edges; mutually exclusive state lives in Phase"
)]
pub(crate) struct Input {
    /// Desired movement direction, normalized by the application.
    pub(crate) movement: Vec2,
    /// Keyboard upgrade selection, indexed from zero.
    pub(crate) choice: Option<usize>,
    /// Cursor position in screen coordinates, also used for menu hover.
    pub(crate) pointer: Vec2,
    /// Positive screen dimensions in the same coordinates as the cursor.
    pub(crate) viewport: Vec2,
    /// Physical pixels per screen coordinate, used to prepare font glyphs.
    pub(crate) dpi_scale: f32,
    /// Whether the primary pointer button was pressed this frame.
    pub(crate) click: bool,
    /// Whether the player pressed the confirm action this frame.
    pub(crate) accept: bool,
    /// Whether the player pressed the pause action this frame.
    pub(crate) pause: bool,
    /// Whether the player pressed the retry action this frame.
    pub(crate) retry: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            movement: Vec2::ZERO,
            choice: None,
            pointer: Vec2::ZERO,
            viewport: vec2(1280.0, VIEW_HEIGHT),
            dpi_scale: 1.0,
            click: false,
            accept: false,
            pause: false,
            retry: false,
        }
    }
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
    LevelUp([Upgrade; 3]),
    Paused,
    GameOver,
}

#[derive(Debug)]
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

    fn lantern_position(&self) -> Vec2 {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnemyKind {
    Shade,
    Wisp,
    Brute,
}

impl EnemyKind {
    const fn radius(self) -> f32 {
        match self {
            Self::Shade => 16.0,
            Self::Wisp => 12.0,
            Self::Brute => 25.0,
        }
    }

    const fn base_health(self) -> f32 {
        match self {
            Self::Shade => 34.0,
            Self::Wisp => 22.0,
            Self::Brute => 105.0,
        }
    }

    const fn speed(self) -> f32 {
        match self {
            Self::Shade => 72.0,
            Self::Wisp => 112.0,
            Self::Brute => 43.0,
        }
    }

    const fn contact_damage(self) -> f32 {
        match self {
            Self::Shade => 13.0,
            Self::Wisp => 9.0,
            Self::Brute => 22.0,
        }
    }

    const fn experience(self) -> u32 {
        match self {
            Self::Shade | Self::Wisp => 3,
            Self::Brute => 10,
        }
    }
}

#[derive(Debug)]
struct Enemy {
    kind: EnemyKind,
    position: Vec2,
    velocity: Vec2,
    health: f32,
    radius: f32,
    flash: f32,
    // One cooldown per enemy prevents an overlapping knife from damaging every frame.
    moon_immunity: f32,
    phase: f32,
}

#[derive(Debug)]
struct Gem {
    position: Vec2,
    velocity: Vec2,
    value: u32,
}

#[derive(Debug)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: Color,
}

#[derive(Debug)]
struct Lightning {
    points: Vec<Vec2>,
    life: f32,
}

#[derive(Debug)]
struct DamageNumber {
    position: Vec2,
    value: u32,
    life: f32,
    color: Color,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Upgrade {
    ExtraKnife,
    SharpenedMoon,
    WiderOrbit,
    FastStorm,
    ForkedStorm,
    PotentStorm,
    Fleet,
    Vitality,
    Magnet,
}

impl Upgrade {
    // Each row supplies one guaranteed choice on every level-up.
    const SCHOOLS: [[Self; 3]; 3] = [
        [Self::ExtraKnife, Self::SharpenedMoon, Self::WiderOrbit],
        [Self::FastStorm, Self::ForkedStorm, Self::PotentStorm],
        [Self::Fleet, Self::Vitality, Self::Magnet],
    ];
}

/// The complete mutable state of one run.
pub(crate) struct Game {
    // Separate streams keep changes to cosmetic effects from changing gameplay.
    rng: RandGenerator,
    effects_rng: RandGenerator,
    phase: Phase,
    player: Player,
    moon: MoonKnives,
    storm: StormLantern,
    enemies: Vec<Enemy>,
    gems: Vec<Gem>,
    particles: Vec<Particle>,
    lightning: Vec<Lightning>,
    damage_numbers: Vec<DamageNumber>,
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
}

impl Game {
    /// Creates a fresh run on its title screen.
    pub(crate) fn new(seed: u64) -> Self {
        Self::with_phase(Phase::Title, seed)
    }

    /// Creates a fresh run that starts immediately.
    pub(crate) fn new_running(seed: u64) -> Self {
        Self::with_phase(Phase::Running, seed)
    }

    fn with_phase(phase: Phase, seed: u64) -> Self {
        let rng = RandGenerator::new();
        rng.srand(seed);
        let effects_rng = RandGenerator::new();
        effects_rng.srand(seed ^ 0x9e37_79b9_7f4a_7c15);

        Self {
            rng,
            effects_rng,
            phase,
            player: Player::new(),
            moon: MoonKnives::new(),
            storm: StormLantern::new(),
            enemies: Vec::with_capacity(MAX_ENEMIES),
            gems: Vec::with_capacity(128),
            particles: Vec::with_capacity(512),
            lightning: Vec::with_capacity(8),
            damage_numbers: Vec::with_capacity(32),
            elapsed: 0.0,
            visual_time: 0.0,
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
    ///
    /// `dt` is finite, nonnegative seconds, capped by the application to avoid
    /// tunneling through collisions after a long frame. All device and viewport
    /// state enters through `input`; this method requires no graphics context.
    pub(crate) fn update(&mut self, dt: f32, input: Input) -> Control {
        debug_assert!(dt.is_finite() && dt >= 0.0);
        self.visual_time += dt;

        match self.phase {
            Phase::Title => {
                if input.accept || input.click {
                    self.phase = Phase::Running;
                }
            },
            Phase::Paused => {
                if input.pause || input.accept {
                    self.phase = Phase::Running;
                }
            },
            Phase::LevelUp(offers) => {
                let choice = input.choice.or_else(|| {
                    input
                        .click
                        .then(|| render::upgrade_at(input.pointer, input.viewport))
                        .flatten()
                });
                if let Some(choice) = choice.filter(|choice| *choice < offers.len()) {
                    self.apply_upgrade(offers[choice]);
                    if self.player.experience >= self.player.next_level {
                        self.begin_level_up();
                    } else {
                        self.phase = Phase::Running;
                    }
                }
            },
            Phase::GameOver => {
                if input.retry || input.accept || input.click {
                    return Control::Restart;
                }
            },
            Phase::Running => {
                if input.pause {
                    self.phase = Phase::Paused;
                } else {
                    self.update_running(dt, input.movement, input.viewport);
                }
            },
        }

        Control::Continue
    }

    fn update_running(&mut self, dt: f32, movement: Vec2, viewport: Vec2) {
        // Hit stop freezes gameplay but lets short-lived impact effects finish.
        // Consume only the stopped portion so the duration does not depend on refresh rate.
        let stopped_dt = self.hit_stop.min(dt);
        self.hit_stop -= stopped_dt;
        if stopped_dt > 0.0 {
            self.update_effects(stopped_dt);
        }
        let dt = dt - stopped_dt;
        if dt == 0.0 {
            return;
        }

        self.elapsed += dt;
        self.shake = (self.shake - 24.0 * dt).max(0.0);
        self.pickup_flash = (self.pickup_flash - dt).max(0.0);
        self.player.invulnerability = (self.player.invulnerability - dt).max(0.0);
        self.moon_angle = (self.moon_angle + self.moon.speed * dt) % TAU;
        self.storm_timer -= dt;

        // This order is intentional: both weapons resolve before dead enemies drop XP.
        self.update_player(dt, movement);
        self.spawn_enemies(dt, viewport);
        self.update_enemies(dt);
        self.resolve_moon_knives();
        self.resolve_storm_lantern();
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
        let velocity_weight = smoothing_weight(18.0, dt);
        self.player.velocity = self.player.velocity.lerp(target_velocity, velocity_weight);
        self.player.position += self.player.velocity * dt;
        if movement.x.abs() > 0.1 {
            self.player.facing = movement.x.signum();
        }
    }

    fn spawn_enemies(&mut self, dt: f32, viewport: Vec2) {
        self.spawn_timer -= dt;
        let mut spawned = 0;
        // Catch up after a slow frame, but cap the work to prevent a spawn spiral.
        while self.spawn_timer <= 0.0 && spawned < 6 {
            self.spawn_timer += spawn_interval(self.elapsed);
            spawned += 1;
            if self.enemies.len() >= MAX_ENEMIES {
                continue;
            }

            let roll = self.rng.gen_range(0.0, 1.0);
            let kind = if self.elapsed > 65.0 && roll < 0.12 {
                EnemyKind::Brute
            } else if self.elapsed > 22.0 && roll < 0.34 {
                EnemyKind::Wisp
            } else {
                EnemyKind::Shade
            };
            let angle = self.rng.gen_range(0.0, TAU);
            // Clear both the current viewport and its movement toward the player.
            let spawn_radius =
                spawn_distance(view_width(viewport)) + self.player.position.distance(self.camera);
            let position = self.camera + Vec2::from_angle(angle) * spawn_radius;
            let health_scale = 1.0 + self.elapsed / 150.0;

            self.enemies.push(Enemy {
                kind,
                position,
                velocity: Vec2::ZERO,
                health: kind.base_health() * health_scale,
                radius: kind.radius(),
                flash: 0.0,
                moon_immunity: 0.0,
                phase: self.rng.gen_range(0.0, TAU),
            });
        }
    }

    fn update_enemies(&mut self, dt: f32) {
        let steering = smoothing_weight(8.0, dt);
        for enemy in &mut self.enemies {
            enemy.flash = (enemy.flash - dt).max(0.0);
            enemy.moon_immunity = (enemy.moon_immunity - dt).max(0.0);

            let direction = (self.player.position - enemy.position).normalize_or_zero();
            let desired_direction = match enemy.kind {
                EnemyKind::Wisp => {
                    let weave = (self.elapsed * 5.0 + enemy.phase).sin() * 0.42;
                    (direction + vec2(-direction.y, direction.x) * weave).normalize_or_zero()
                },
                EnemyKind::Shade | EnemyKind::Brute => direction,
            };
            let desired_velocity = desired_direction * enemy.kind.speed();
            enemy.velocity = enemy.velocity.lerp(desired_velocity, steering);
            enemy.position += enemy.velocity * dt;
        }

        Self::separate_enemies(&mut self.enemies, dt);
    }

    fn separate_enemies(enemies: &mut [Enemy], dt: f32) {
        // Split a frame-rate-independent overlap correction equally between both enemies.
        let correction = smoothing_weight(ENEMY_SEPARATION_RATE, dt) * 0.5;
        // The bounded enemy count keeps this pairwise pass simpler than spatial bins.
        // `split_at_mut` visits each pair once while proving the borrows cannot overlap.
        for left_index in 0..enemies.len() {
            let (left, right) = enemies.split_at_mut(left_index + 1);
            let first = &mut left[left_index];
            for second in right {
                let offset = second.position - first.position;
                let distance_squared = offset.length_squared();
                let minimum = (first.radius + second.radius) * 0.72;
                if distance_squared >= minimum * minimum {
                    continue;
                }
                let distance = distance_squared.sqrt();
                // Coincident centers have no direction; choose a stable axis.
                let direction = if distance > 0.0 {
                    offset / distance
                } else {
                    Vec2::X
                };
                let push = direction * (minimum - distance);
                first.position -= push * correction;
                second.position += push * correction;
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
        let mut falloff = 1.0;
        for index in targets {
            let enemy = &mut self.enemies[index];
            let damage = self.storm.damage * falloff;
            falloff *= 0.88;
            enemy.health -= damage;
            enemy.flash = 0.13;
            enemy.velocity += (enemy.position - self.player.position).normalize_or_zero() * 55.0;
            points.push(enemy.position);
            hits.push((enemy.position, damage));
        }

        self.lightning.push(Lightning {
            points,
            life: LIGHTNING_LIFE,
        });
        for (position, damage) in hits {
            self.damage_number(position, damage, STORM_CYAN);
            self.burst(position, STORM_CYAN, 4, 75.0);
        }
        self.storm_timer = self.storm.cooldown;
        self.shake = self.shake.max(5.5);
        self.hit_stop = 0.025;
    }

    fn resolve_player_contact(&mut self) {
        if self.player.invulnerability > 0.0 {
            return;
        }

        let closest = self
            .enemies
            .iter_mut()
            .filter(|enemy| enemy.health > 0.0)
            .filter(|enemy| {
                let contact = PLAYER_RADIUS + enemy.radius;
                enemy.position.distance_squared(self.player.position) < contact * contact
            })
            .min_by(|left, right| {
                left.position
                    .distance_squared(self.player.position)
                    .total_cmp(&right.position.distance_squared(self.player.position))
            });

        if let Some(enemy) = closest {
            let away = (self.player.position - enemy.position).normalize_or_zero();
            let damage = enemy.kind.contact_damage();
            self.player.health -= damage;
            self.player.velocity += away * 280.0;
            enemy.velocity -= away * 170.0;
            self.player.invulnerability = 0.72;
            self.shake = 13.0;
            self.hit_stop = 0.055;
            self.damage_number(self.player.position + vec2(0.0, -28.0), damage, DAMAGE_RED);
            self.burst(self.player.position, DAMAGE_RED, 12, 155.0);
        }
    }

    fn remove_defeated_enemies(&mut self) {
        // Walk backward because `swap_remove` changes the element at the current index.
        for index in (0..self.enemies.len()).rev() {
            if self.enemies[index].health > 0.0 {
                continue;
            }
            let enemy = self.enemies.swap_remove(index);
            self.gems.push(Gem {
                position: enemy.position,
                velocity: Vec2::from_angle(self.rng.gen_range(0.0, TAU))
                    * self.rng.gen_range(35.0, 75.0),
                value: enemy.kind.experience(),
            });
            self.burst(enemy.position, ARCANE_VIOLET, 9, 130.0);
            self.kills += 1;
        }
    }

    fn collect_gems(&mut self, dt: f32) {
        let damping = (-3.2 * dt).exp();
        let mut gained = 0;
        for index in (0..self.gems.len()).rev() {
            let gem = &mut self.gems[index];
            let offset = self.player.position - gem.position;
            let distance = offset.length();
            if distance < self.player.pickup_radius {
                let pull = 560.0 + (self.player.pickup_radius - distance) * 7.0;
                gem.velocity += offset.normalize_or_zero() * pull * dt;
            }
            gem.velocity *= damping;
            gem.position += gem.velocity * dt;

            if gem.position.distance_squared(self.player.position) < (PLAYER_RADIUS + 8.0).powi(2) {
                gained += gem.value;
                self.gems.swap_remove(index);
            }
        }

        if gained > 0 {
            self.pickup_flash = PICKUP_FLASH_LIFE;
            self.burst(
                self.player.position,
                GEM_GREEN,
                gained.min(7) as usize,
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
        self.phase = Phase::LevelUp(random_offers(&self.rng));
    }

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
            Upgrade::Fleet => self.player.speed *= 1.18,
            Upgrade::Vitality => {
                self.player.max_health += 35.0;
                self.player.health = (self.player.health + 35.0).min(self.player.max_health);
            },
            Upgrade::Magnet => self.player.pickup_radius *= 1.45,
        }
    }

    fn update_effects(&mut self, dt: f32) {
        let damping = (-4.5 * dt).exp();
        for particle in &mut self.particles {
            particle.life -= dt;
            particle.position += particle.velocity * dt;
            particle.velocity *= damping;
        }
        self.particles.retain(|particle| particle.life > 0.0);

        for lightning in &mut self.lightning {
            lightning.life -= dt;
        }
        self.lightning.retain(|lightning| lightning.life > 0.0);

        for number in &mut self.damage_numbers {
            number.life -= dt;
            number.position.y -= 34.0 * dt;
        }
        self.damage_numbers.retain(|number| number.life > 0.0);
    }

    fn burst(&mut self, position: Vec2, color: Color, count: usize, speed: f32) {
        for _ in 0..count {
            let life = self.effects_rng.gen_range(0.16, 0.34);
            self.particles.push(Particle {
                position,
                velocity: Vec2::from_angle(self.effects_rng.gen_range(0.0, TAU))
                    * self.effects_rng.gen_range(speed * 0.45, speed),
                life,
                max_life: life,
                size: self.effects_rng.gen_range(2.0, 5.0),
                color,
            });
        }
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

/// Starts within lantern range, then follows shorter nearest-neighbor jumps.
fn chain_targets(enemies: &[Enemy], origin: Vec2, initial_range: f32, count: usize) -> Vec<usize> {
    let count = count.min(enemies.len());
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

fn random_offers(rng: &RandGenerator) -> [Upgrade; 3] {
    Upgrade::SCHOOLS.map(|school| school[rng.gen_range(0, school.len())])
}

fn spawn_interval(elapsed: f32) -> f32 {
    (0.64 - elapsed * 0.0038).max(0.14)
}

fn spawn_distance(width: f32) -> f32 {
    // Half the viewport diagonal clears every edge; the margin hides pop-in.
    width.hypot(VIEW_HEIGHT) * 0.5 + 54.0
}

fn view_width(viewport: Vec2) -> f32 {
    // The world stays 720 units tall while widening with the window's aspect ratio.
    VIEW_HEIGHT * viewport.x / viewport.y
}

fn smoothing_weight(rate: f32, dt: f32) -> f32 {
    // Exponential smoothing converges at the same rate regardless of frame rate.
    -(-rate * dt).exp_m1()
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
        let mut game = Game::new_running(1);
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

    fn game_without_spawning() -> Game {
        let mut game = Game::new_running(1);
        game.spawn_timer = f32::INFINITY;
        game.storm_timer = f32::INFINITY;
        game
    }

    fn enemy_states(game: &Game) -> Vec<(EnemyKind, Vec2, Vec2, f32, f32)> {
        game.enemies
            .iter()
            .map(|enemy| {
                (
                    enemy.kind,
                    enemy.position,
                    enemy.velocity,
                    enemy.health,
                    enemy.phase,
                )
            })
            .collect()
    }

    #[test]
    fn complete_updates_spawn_without_a_graphics_context_and_keep_runs_independent() {
        let mut expected = Game::new_running(42);
        let mut actual = Game::new_running(42);
        let mut unrelated = Game::new_running(99);

        for _ in 0..120 {
            expected.update(1.0 / 60.0, Input::default());
            unrelated.update(1.0 / 60.0, Input::default());
            actual.update(1.0 / 60.0, Input::default());
        }

        assert!(!actual.enemies.is_empty());
        assert_eq!(enemy_states(&actual), enemy_states(&expected));
        assert_ne!(enemy_states(&actual), enemy_states(&unrelated));
        assert_eq!(random_offers(&actual.rng), random_offers(&expected.rng));
    }

    #[test]
    fn cosmetic_randomness_does_not_change_spawns_or_upgrade_offers() {
        let mut expected = Game::new_running(42);
        let mut actual = Game::new_running(42);

        for _ in 0..120 {
            actual.burst(Vec2::ZERO, MOON_GOLD, 1, 10.0);
            expected.update(1.0 / 60.0, Input::default());
            actual.update(1.0 / 60.0, Input::default());
        }

        assert_eq!(enemy_states(&actual), enemy_states(&expected));
        assert_eq!(random_offers(&actual.rng), random_offers(&expected.rng));
    }

    #[test]
    fn moon_defeated_enemies_cannot_deal_contact_damage_in_the_same_frame() {
        let mut game = game_without_spawning();
        game.moon.radius = 0.0;
        game.enemies.push(enemy_at(0.0, 0.0));

        game.update(1.0 / 60.0, Input::default());

        assert_eq!(
            game.player.health.to_bits(),
            game.player.max_health.to_bits()
        );
        assert!(game.enemies.is_empty());
        assert_eq!(game.kills, 1);
    }

    #[test]
    fn storm_defeated_enemies_cannot_deal_contact_damage_in_the_same_frame() {
        let mut game = game_without_spawning();
        game.storm_timer = 0.0;
        game.enemies.push(enemy_at(0.0, 0.0));

        game.update(1.0 / 60.0, Input::default());

        assert_eq!(
            game.player.health.to_bits(),
            game.player.max_health.to_bits()
        );
        assert!(game.enemies.is_empty());
        assert_eq!(game.kills, 1);
    }

    #[test]
    fn gem_entering_pickup_range_is_collected_in_the_same_frame() {
        let mut game = game_without_spawning();
        game.gems.push(Gem {
            position: vec2(27.0, 0.0),
            velocity: vec2(-150.0, 0.0),
            value: 3,
        });

        game.update(1.0 / 60.0, Input::default());

        assert!(game.gems.is_empty());
        assert_eq!(game.player.experience, 3);
    }

    #[test]
    fn hit_stop_consumes_only_its_remaining_duration_and_advances_effects() {
        let mut game = game_without_spawning();
        game.hit_stop = 0.025;
        game.damage_numbers.push(DamageNumber {
            position: Vec2::ZERO,
            value: 1,
            life: 0.5,
            color: MOON_GOLD,
        });

        game.update(0.03, Input::default());

        assert!(game.hit_stop.abs() < f32::EPSILON);
        assert!((game.elapsed - 0.005).abs() < f32::EPSILON);
        assert!((game.damage_numbers[0].life - 0.47).abs() < f32::EPSILON);
        assert!((game.damage_numbers[0].position.y + 34.0 * 0.03).abs() < f32::EPSILON);
    }

    #[test]
    fn hit_stop_duration_is_stable_at_high_frame_rates() {
        let elapsed_after_one_second = |frame_rate: usize| {
            let mut game = game_without_spawning();
            game.hit_stop = 0.025;
            for _ in 0..frame_rate {
                game.update(1.0 / frame_rate as f32, Input::default());
            }
            game.elapsed
        };

        assert!((elapsed_after_one_second(60) - 0.975).abs() < 0.000_01);
        assert!((elapsed_after_one_second(400) - 0.975).abs() < 0.000_01);
    }

    #[test]
    fn coincident_enemies_separate_without_invalid_coordinates() {
        let mut game = game_without_spawning();
        game.enemies = vec![enemy_at(0.0, 0.0), enemy_at(0.0, 0.0)];

        game.update(1.0 / 60.0, Input::default());

        assert!(game.enemies[0].position.is_finite());
        assert!(game.enemies[1].position.is_finite());
        assert!(
            game.enemies[0]
                .position
                .distance_squared(game.enemies[1].position)
                > 0.0
        );
        assert_eq!(
            game.enemies[0].position + game.enemies[1].position,
            Vec2::ZERO
        );
    }

    #[test]
    fn enemies_spawn_outside_the_viewport_even_while_the_camera_catches_up() {
        let input = Input {
            viewport: vec2(1920.0, 720.0),
            ..Input::default()
        };
        let half_viewport = vec2(view_width(input.viewport), VIEW_HEIGHT) * 0.5;

        for seed in 0..32 {
            let mut game = Game::new_running(seed);
            game.player.position = vec2(2000.0, 1000.0);
            game.spawn_timer = 0.0;

            game.update(1.0 / 30.0, input);

            assert_eq!(game.enemies.len(), 1);
            let enemy = &game.enemies[0];
            let offset = enemy.position - game.camera;
            assert!(
                offset.x.abs() > half_viewport.x + enemy.radius
                    || offset.y.abs() > half_viewport.y + enemy.radius
            );
        }
    }

    #[test]
    fn lethal_contact_ends_the_run_and_game_over_freezes_gameplay() {
        let mut game = game_without_spawning();
        game.player.health = 1.0;
        game.enemies.push(enemy_at(0.0, 0.0));

        game.update(1.0 / 60.0, Input::default());

        assert_eq!(game.phase, Phase::GameOver);
        assert_eq!(game.player.health.to_bits(), 0.0_f32.to_bits());
        let elapsed = game.elapsed.to_bits();
        let position = game.player.position;

        // Check beyond the lethal hit's hit stop, which also freezes a running game.
        for _ in 0..6 {
            game.update(
                1.0 / 60.0,
                Input {
                    movement: Vec2::X,
                    ..Input::default()
                },
            );
        }

        assert_eq!(game.elapsed.to_bits(), elapsed);
        assert_eq!(game.player.position, position);
        assert_eq!(
            game.update(
                1.0 / 60.0,
                Input {
                    accept: true,
                    ..Input::default()
                },
            ),
            Control::Restart
        );
    }

    #[test]
    fn phases_only_advance_on_their_corresponding_input_actions() {
        let mut game = Game::new(1);
        let dt = 1.0 / 60.0;

        game.update(dt, Input::default());
        assert_eq!(game.phase, Phase::Title);
        game.update(
            dt,
            Input {
                click: true,
                ..Input::default()
            },
        );
        assert_eq!(game.phase, Phase::Running);

        let pause = Input {
            pause: true,
            ..Input::default()
        };
        game.update(dt, pause);
        assert_eq!(game.phase, Phase::Paused);
        let elapsed = game.elapsed;
        game.update(dt, Input::default());
        assert_eq!(game.elapsed.to_bits(), elapsed.to_bits());
        game.update(dt, pause);
        assert_eq!(game.phase, Phase::Running);

        game.phase = Phase::GameOver;
        assert_eq!(game.update(dt, Input::default()), Control::Continue);
        assert_eq!(
            game.update(
                dt,
                Input {
                    retry: true,
                    ..Input::default()
                },
            ),
            Control::Restart
        );
    }

    #[test]
    fn invalid_upgrade_choices_do_not_advance_and_leftover_experience_is_preserved() {
        let mut game = game_without_spawning();
        game.gems.push(Gem {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            value: 100,
        });

        game.update(1.0 / 60.0, Input::default());

        assert!(matches!(game.phase, Phase::LevelUp(_)));
        assert_eq!(game.player.level, 2);
        assert_eq!(game.player.experience, 91);

        game.update(
            1.0 / 60.0,
            Input {
                choice: Some(3),
                ..Input::default()
            },
        );
        assert!(matches!(game.phase, Phase::LevelUp(_)));
        assert_eq!(game.player.level, 2);
        assert_eq!(game.player.experience, 91);

        let next_level = game.player.next_level;
        game.update(
            1.0 / 60.0,
            Input {
                choice: Some(0),
                ..Input::default()
            },
        );
        assert!(matches!(game.phase, Phase::LevelUp(_)));
        assert_eq!(game.player.level, 3);
        assert_eq!(game.player.experience, 91 - next_level);
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
        let mut game = Game::new_running(1);
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
    fn viewport_aspect_ratio_is_independent_of_coordinate_scale() {
        for viewport in [vec2(1280.0, 640.0), vec2(2.0, 1.0), vec2(0.5, 0.25)] {
            assert!((view_width(viewport) - 1440.0).abs() < f32::EPSILON);
        }
    }
}
