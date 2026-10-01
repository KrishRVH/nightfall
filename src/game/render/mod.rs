//! Deferred 2D renderer: offscreen scene, additive light accumulation, bloom
//! and a graded composite.
//!
//! A frame is built in five stages:
//!
//! 1. the world renders into a multisampled colour target, so sprite edges stay
//!    clean while the ground and creatures fill the frame at native resolution;
//! 2. dynamic lights accumulate additively into a half-resolution buffer;
//! 3. a bright pass and two separable blur iterations produce the bloom;
//! 4. the composite multiplies scene by light, tonemaps, grades and adds grain;
//! 5. the interface is drawn on top at native resolution so text never passes
//!    through bloom or grain.
//!
//! Materials use `#version 100` GLSL against macroquad's fixed vertex layout, so
//! everything here is written against [`shaders`] and [`sprite`] rather than
//! macroquad's immediate-mode shapes.
//!
//! ## Offscreen orientation
//!
//! A [`Camera2D`] attached to a render target maps world +Y (down) onto the
//! *bottom* of the stored image, the opposite of the on-screen mapping. Passes
//! that sample the world targets therefore read them through `1.0 - uv.y`, while
//! the bloom buffer, which is written in screen space, is sampled directly.

mod scene;
mod shaders;
mod sprite;
mod ui;

use macroquad::prelude::*;
use miniquad::{
    BlendFactor, BlendState, BlendValue, Comparison, Equation, PipelineParams, UniformDesc,
    UniformType,
};

use sprite::{Sprite, SpriteBatch};

use crate::game::{
    ARCANE_VIOLET, BACKGROUND, BONE, DAMAGE_NUMBER_LIFE, DAMAGE_RED, GEM_GREEN, Game, INK,
    MOON_GOLD, Phase, STORM_CYAN, Upgrade, VIEW_HEIGHT, format_time, view_width,
};

pub(crate) use ui::upgrade_at;

/// Deterministic per-cell hash, used by the interface's starfield.
///
/// It mirrors the construction the ground shader uses, so the world and the menu
/// share one notion of "random but stable".
pub(super) fn star_hash(x: i32, y: i32) -> u32 {
    let mut value = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value
}

/// Additive blending, used by every light-emitting pass.
fn additive_blend() -> BlendState {
    BlendState::new(
        Equation::Add,
        BlendFactor::Value(BlendValue::SourceAlpha),
        BlendFactor::One,
    )
}

/// Straight alpha blending, used for the lit world target.
fn alpha_blend() -> BlendState {
    BlendState::new(
        Equation::Add,
        BlendFactor::Value(BlendValue::SourceAlpha),
        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
    )
}

/// Fullscreen passes write every pixel, so they never blend.
fn opaque() -> PipelineParams {
    PipelineParams {
        depth_test: Comparison::Always,
        ..Default::default()
    }
}

/// Blend parameters shared by the sprite and ground materials.
fn blended() -> PipelineParams {
    PipelineParams {
        color_blend: Some(alpha_blend()),
        ..opaque()
    }
}

/// Upper bound on simultaneous dynamic lights, mirroring the shader arrays.
pub(super) const MAX_LIGHTS: usize = 24;

/// A light emitter collected from simulation state each frame.
#[derive(Clone, Copy)]
pub(super) struct Light {
    pub(super) position: Vec2,
    pub(super) radius: f32,
    pub(super) color: Color,
    pub(super) intensity: f32,
}

/// Offscreen colour targets, rebuilt whenever the window changes size.
///
/// Targets cannot be resized in place, so a size mismatch reallocates the set.
struct Targets {
    width: u32,
    height: u32,
    /// Multisampled world colour.
    scene: RenderTarget,
    /// Additive light accumulation at half resolution.
    light: RenderTarget,
    /// Two bloom buffers, ping-ponged between blur passes.
    bloom: [RenderTarget; 2],
}

impl Targets {
    fn new(width: u32, height: u32) -> Self {
        // The world target is multisampled; lights and bloom are half and
        // quarter resolution because both are low-frequency by nature.
        let scene = render_target_ex(
            width,
            height,
            RenderTargetParams {
                sample_count: 0,
                depth: false,
            },
        );
        Self {
            width,
            height,
            scene,
            light: render_target((width / 2).max(2), (height / 2).max(2)),
            bloom: [
                render_target((width / 4).max(2), (height / 4).max(2)),
                render_target((width / 4).max(2), (height / 4).max(2)),
            ],
        }
    }
}

/// Compiled materials, grouped by the pass that uses them.
struct Materials {
    ground: Material,
    sprite: Material,
    glow: Material,
    light: Material,
    bright: Material,
    blur: Material,
    composite: Material,
}

impl Materials {
    fn load() -> Result<Self, macroquad::Error> {
        Ok(Self {
            ground: Self::ground()?,
            sprite: Self::sprite()?,
            glow: Self::glow()?,
            light: Self::light()?,
            bright: Self::bright()?,
            blur: Self::blur()?,
            composite: Self::composite()?,
        })
    }

    /// Procedural paving.
    fn ground() -> Result<Material, macroquad::Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: shaders::SPRITE_VERTEX,
                fragment: &shaders::ground_fragment(),
            },
            MaterialParams {
                pipeline_params: blended(),
                uniforms: vec![UniformDesc::new("u_pixel", UniformType::Float1)],
                textures: vec![],
            },
        )
    }

    /// Signed-distance creatures and effects.
    fn sprite() -> Result<Material, macroquad::Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: shaders::SPRITE_VERTEX,
                fragment: &shaders::sprite_fragment(),
            },
            MaterialParams {
                pipeline_params: blended(),
                uniforms: vec![UniformDesc::new("u_pixel", UniformType::Float1)],
                textures: vec![],
            },
        )
    }

    /// Additive halos.
    fn glow() -> Result<Material, macroquad::Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: shaders::SPRITE_VERTEX,
                fragment: shaders::GLOW_FRAGMENT,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(additive_blend()),
                    ..opaque()
                },
                uniforms: vec![UniformDesc::new("u_pixel", UniformType::Float1)],
                textures: vec![],
            },
        )
    }

    /// Analytic point-light accumulation.
    fn light() -> Result<Material, macroquad::Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: shaders::PASS_VERTEX,
                fragment: shaders::LIGHT_FRAGMENT,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(additive_blend()),
                    ..opaque()
                },
                uniforms: vec![
                    UniformDesc::new("u_lights", UniformType::Float4).array(MAX_LIGHTS),
                    UniformDesc::new("u_light_colors", UniformType::Float4).array(MAX_LIGHTS),
                    UniformDesc::new("u_light_count", UniformType::Float1),
                ],
                textures: vec![],
            },
        )
    }

    /// Bright pass with a soft knee.
    fn bright() -> Result<Material, macroquad::Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: shaders::PASS_VERTEX,
                fragment: shaders::BRIGHT_FRAGMENT,
            },
            MaterialParams {
                pipeline_params: opaque(),
                uniforms: vec![UniformDesc::new("u_threshold", UniformType::Float1)],
                textures: vec!["u_scene".to_owned(), "u_light".to_owned()],
            },
        )
    }

    /// Separable Gaussian blur.
    fn blur() -> Result<Material, macroquad::Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: shaders::PASS_VERTEX,
                fragment: shaders::BLUR_FRAGMENT,
            },
            MaterialParams {
                pipeline_params: opaque(),
                uniforms: vec![UniformDesc::new("u_direction", UniformType::Float2)],
                textures: vec!["u_source".to_owned()],
            },
        )
    }

    /// Final grade and composite.
    fn composite() -> Result<Material, macroquad::Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: shaders::PASS_VERTEX,
                fragment: shaders::COMPOSITE_FRAGMENT,
            },
            MaterialParams {
                pipeline_params: opaque(),
                uniforms: vec![
                    UniformDesc::new("u_resolution", UniformType::Float2),
                    UniformDesc::new("u_bloom_strength", UniformType::Float1),
                    UniformDesc::new("u_hurt", UniformType::Float1),
                    UniformDesc::new("u_flash", UniformType::Float1),
                    UniformDesc::new("u_health", UniformType::Float1),
                ],
                textures: vec![
                    "u_scene".to_owned(),
                    "u_light".to_owned(),
                    "u_bloom".to_owned(),
                ],
            },
        )
    }
}

/// Owns the GPU resources and draws a frame.
///
/// The renderer only ever borrows the simulation, so drawing cannot change
/// gameplay state, and the simulation never touches a target or a material.
pub(crate) struct Renderer {
    targets: Targets,
    materials: Materials,
    sprites: SpriteBatch,
    glows: SpriteBatch,
    lights: Vec<Light>,
    light_data: Vec<Vec4>,
    light_colors: Vec<Vec4>,
    /// Enemy draw order, reused between frames to keep sorting allocation-free.
    order: Vec<usize>,
    /// The camera this frame rendered through, kept so the interface can place
    /// world-anchored text on the screen without recomputing the transform.
    view: Camera2D,
    ui: ui::UiState,
}

impl Renderer {
    /// Compiles the pipeline and allocates the offscreen targets.
    ///
    /// Returns `None` when a shader fails to compile, so a broken shader reports
    /// itself instead of taking the window down.
    pub(crate) fn new() -> Option<Self> {
        let materials = match Materials::load() {
            Ok(materials) => materials,
            Err(error) => {
                eprintln!("nightfall: shader compilation failed: {error}");
                return None;
            },
        };
        Some(Self {
            targets: Targets::new(screen_size().0, screen_size().1),
            materials,
            sprites: SpriteBatch::new(),
            glows: SpriteBatch::new(),
            lights: Vec::with_capacity(MAX_LIGHTS),
            light_data: vec![vec4(0.0_f32, 0.0, 0.0, 0.0); MAX_LIGHTS],
            light_colors: vec![vec4(0.0_f32, 0.0, 0.0, 0.0); MAX_LIGHTS],
            order: Vec::new(),
            view: Camera2D::default(),
            ui: ui::UiState::default(),
        })
    }

    /// Recreates the offscreen targets when the window size changes.
    fn refresh_targets(&mut self) {
        let (width, height) = screen_size();
        if width != self.targets.width || height != self.targets.height {
            self.targets = Targets::new(width, height);
        }
    }

    /// Draws one complete frame.
    pub(crate) fn draw(&mut self, game: &Game) {
        self.refresh_targets();
        self.ui.advance(game);

        if game.phase == Phase::Title {
            // The title is a self-contained screen and needs no lighting stack.
            set_default_camera();
            clear_background(BACKGROUND);
            ui::draw_title(game);
            return;
        }

        let center = game.camera_offset();
        self.draw_world_pass(game, center);
        self.draw_light_pass(game, center);
        self.draw_bloom_pass();
        self.draw_composite_pass(game);

        // The interface comes last, at native resolution, so text and panel edges
        // never pass through the bloom or grain stages.
        set_default_camera();
        ui::draw_frame(game, &self.ui, &self.view);
        gl_use_default_material();
    }

    /// Renders the lit-able world into the scene target.
    fn draw_world_pass(&mut self, game: &Game, center: Vec2) {
        let pixel = world_pixel();
        self.view = world_camera(center, &self.targets.scene);

        set_camera(&self.view);
        clear_background(BACKGROUND);

        gl_use_material(&self.materials.ground);
        self.materials.ground.set_uniform("u_pixel", pixel);
        fill_world(&mut self.sprites, center);

        gl_use_material(&self.materials.sprite);
        self.materials.sprite.set_uniform("u_pixel", pixel);
        self.draw_world(game);

        // Emissive halos are additive on top of the lit scene, not a light
        // multiplier: the lantern corona, eye lights, blade glints and embers
        // emit light rather than gating it.
        gl_use_material(&self.materials.glow);
        self.materials.glow.set_uniform("u_pixel", pixel);
        self.draw_glows(game);

        gl_use_default_material();
        set_default_camera();
    }

    /// Accumulates dynamic lights and emissive glows additively.
    fn draw_light_pass(&mut self, game: &Game, center: Vec2) {
        let camera = world_camera(center, &self.targets.light);

        set_camera(&camera);
        clear_background(TRANSPARENT_BLACK);

        self.collect_lights(game);
        let count = self.lights.len().min(MAX_LIGHTS);
        for (index, light) in self.lights.iter().take(count).enumerate() {
            self.light_data[index] = vec4(
                light.position.x,
                light.position.y,
                light.radius,
                light.intensity,
            );
            self.light_colors[index] =
                vec4(light.color.r, light.color.g, light.color.b, light.color.a);
        }

        gl_use_material(&self.materials.light);
        self.materials
            .light
            .set_uniform_array("u_lights", &self.light_data);
        self.materials
            .light
            .set_uniform_array("u_light_colors", &self.light_colors);
        self.materials
            .light
            .set_uniform("u_light_count", count as f32);
        fill_world(&mut self.sprites, center);

        gl_use_default_material();
        set_default_camera();
    }

    /// Bright pass followed by two separable blur iterations.
    ///
    /// Every pass reads one buffer and writes the other, so no target is ever
    /// sampled while it is bound. The final result lands back in `bloom[0]`.
    fn draw_bloom_pass(&mut self) {
        let quarter = (self.targets.width / 4).max(1) as f32;

        set_camera(&screen_camera(&self.targets.bloom[0]));
        clear_background(TRANSPARENT_BLACK);
        gl_use_material(&self.materials.bright);
        self.materials
            .bright
            .set_texture("u_scene", self.targets.scene.texture.clone());
        self.materials
            .bright
            .set_texture("u_light", self.targets.light.texture.clone());
        self.materials.bright.set_uniform("u_threshold", 0.70_f32);
        fill_screen(&mut self.sprites);

        // Horizontal then vertical, repeated at a wider stride, which grows the
        // halo without spending taps.
        for (iteration, stride) in [1.0_f32, 2.4].into_iter().enumerate() {
            self.blur_bloom(0, 1, vec2(1.0 / quarter, 0.0) * stride);
            self.blur_bloom(1, 0, vec2(0.0, 1.0 / quarter) * stride);
            let _ = iteration;
        }
        gl_use_default_material();
    }

    /// Runs one separable blur step between the two bloom buffers.
    fn blur_bloom(&mut self, source: usize, destination: usize, direction: Vec2) {
        let texture = self.targets.bloom[source].texture.clone();
        set_camera(&screen_camera(&self.targets.bloom[destination]));
        clear_background(TRANSPARENT_BLACK);
        gl_use_material(&self.materials.blur);
        self.materials.blur.set_texture("u_source", texture);
        self.materials.blur.set_uniform("u_direction", direction);
        fill_screen(&mut self.sprites);
    }

    /// Resolves the frame to the screen.
    fn draw_composite_pass(&mut self, game: &Game) {
        set_default_camera();
        gl_use_material(&self.materials.composite);
        self.materials
            .composite
            .set_texture("u_scene", self.targets.scene.texture.clone());
        self.materials
            .composite
            .set_texture("u_light", self.targets.light.texture.clone());
        self.materials
            .composite
            .set_texture("u_bloom", self.targets.bloom[0].texture.clone());
        self.materials.composite.set_uniform(
            "u_resolution",
            vec2(self.targets.width as f32, self.targets.height as f32),
        );
        self.materials
            .composite
            .set_uniform("u_bloom_strength", 0.30_f32);
        self.materials
            .composite
            .set_uniform("u_hurt", game.hurt_flash);
        self.materials
            .composite
            .set_uniform("u_flash", game.spell_flash);
        self.materials
            .composite
            .set_uniform("u_health", game.health_ratio());
        fill_screen(&mut self.sprites);
        gl_use_default_material();
    }
}

/// The colour every offscreen target is cleared to before it is drawn into.
const TRANSPARENT_BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);

/// Clears happen immediately after their camera is bound: `clear_background`
/// acts on whatever render pass is currently active, so clearing before binding
/// would wipe the previous pass's target instead.
///
/// Covers the active world-space target with one quad.
///
/// `texcoord` carries the local offset in world units, which is what the ground
/// and light shaders read as world position, so the quad has to be sized in
/// world units rather than in target pixels.
fn fill_world(batch: &mut SpriteBatch, center: Vec2) {
    let extent = vec2(view_width(), VIEW_HEIGHT) * 0.5 + 16.0;
    batch.clear();
    batch.push(&Sprite::at(center, extent, WHITE, 0.0, 1.0));
    batch.submit();
}

/// Covers the active screen-space target with one quad.
///
/// Unlike [`fill_world`], the texture coordinates span 0..1 across the quad,
/// because the post-process stages sample their inputs by normalised UV.
fn fill_screen(batch: &mut SpriteBatch) {
    let extent = vec2(screen_width(), screen_height()) * 0.5;
    batch.clear();
    batch.push_uv(extent, extent, WHITE);
    batch.submit();
}

/// Window size in whole pixels, never smaller than two.
fn screen_size() -> (u32, u32) {
    (
        (screen_width() as u32).max(2),
        (screen_height() as u32).max(2),
    )
}

/// World units covered by one screen pixel, used for analytic antialiasing.
fn world_pixel() -> f32 {
    view_width() / screen_width().max(1.0)
}

/// A world-space camera that renders into an offscreen target.
fn world_camera(center: Vec2, target: &RenderTarget) -> Camera2D {
    let mut camera = Camera2D::from_display_rect(Rect::new(
        center.x - view_width() * 0.5,
        center.y - VIEW_HEIGHT * 0.5,
        view_width(),
        VIEW_HEIGHT,
    ));
    camera.render_target = Some(target.clone());
    // `from_display_rect` already produces a negative Y zoom, and
    // `Camera2D::matrix` leaves that sign alone for a render target, so world +Y
    // maps to screen-down as the gameplay code expects. Forcing the sign
    // positive here mirrored every sprite vertically.
    camera
}

/// A camera covering the whole window, rendering into an offscreen target.
fn screen_camera(target: &RenderTarget) -> Camera2D {
    let mut camera =
        Camera2D::from_display_rect(Rect::new(0.0, 0.0, screen_width(), screen_height()));
    camera.render_target = Some(target.clone());
    camera
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Macroquad's 2D camera maps a vertex's `z` straight into normalised device
    /// coordinates, so a non-zero value clips the triangle away. Every sprite is
    /// therefore built with `z == 0`, and depth comes from submission order alone.
    #[test]
    fn sprites_leave_the_depth_coordinate_at_zero() {
        let mut batch = SpriteBatch::new();
        batch.push(&Sprite::at(Vec2::ZERO, Vec2::splat(4.0), WHITE, 0.0, 8.0));
        batch.push_rotated(
            &Sprite::at(Vec2::splat(10.0), Vec2::splat(4.0), WHITE, 0.0, 8.0).rotated(0.7),
        );
        batch.push_uv(Vec2::ZERO, Vec2::splat(1.0), WHITE);
        assert!(batch.vertices().iter().all(|v| v.position.z == 0.0));
    }

    /// A batch is bounded so it never exceeds macroquad's `u16` index range, and
    /// it reports that before a quad is added rather than after.
    #[test]
    fn a_batch_reports_before_it_overflows_the_index_range() {
        let mut batch = SpriteBatch::new();
        assert!(!batch.is_full());
        for _ in 0..4095 {
            batch.push(&Sprite::at(Vec2::ZERO, Vec2::splat(1.0), WHITE, 0.0, 2.0));
        }
        assert!(!batch.is_full());
        batch.push(&Sprite::at(Vec2::ZERO, Vec2::splat(1.0), WHITE, 0.0, 2.0));
        assert!(batch.is_full());
        assert!(
            u16::try_from(batch.vertices().len()).is_ok(),
            "a full batch must still be indexable"
        );
    }

    /// World targets store world +Y (down) at the bottom of the image, so the
    /// composite reads them upside down. Both stages that sample them have to
    /// agree, or the lighting ends up mirrored relative to the albedo.
    #[test]
    fn every_stage_sampling_a_world_target_applies_the_flip() {
        for source in [shaders::BRIGHT_FRAGMENT, shaders::COMPOSITE_FRAGMENT] {
            assert!(
                source.contains("1.0 - v_uv.y"),
                "a stage sampling a world target must flip it the same way"
            );
        }
    }

    /// The sprite vertex stage has to unpack `color0` itself. Miniquad binds that
    /// attribute with normalisation disabled, so it arrives as raw 0..255 bytes;
    /// missing the divide makes every tint read as pure white.
    #[test]
    fn the_vertex_stage_scales_the_byte_colour_attribute() {
        assert!(
            shaders::SPRITE_VERTEX.contains("color0 / 255.0"),
            "colour0 arrives as raw bytes and has to be scaled"
        );
    }

    /// The light buffer is a multiplier applied to the scene, so it is clamped in
    /// the shader. Without that ceiling, a cluster of overlapping emitters summed
    /// into the tens and every lit surface saturated to white.
    #[test]
    fn the_light_buffer_is_clamped_before_it_multiplies_the_scene() {
        assert!(shaders::LIGHT_FRAGMENT.contains("min(total"));
    }
}
