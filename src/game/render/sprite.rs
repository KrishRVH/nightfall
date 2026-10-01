//! Batched quad geometry for the signed-distance sprite material.
//!
//! Macroquad's vertex layout is fixed, so per-vertex data is packed as:
//!
//! - `position` - world position; `.z` is always zero, because macroquad's 2D
//!   camera matrix maps it straight into NDC and a non-zero value would clip the
//!   triangle. Painter order comes from submission order instead.
//! - `texcoord` - offset from the sprite centre, in world units, which lets the
//!   shader evaluate its distance field at true world scale and antialias
//!   against the exact pixel footprint
//! - `color0`   - RGBA tint
//! - `normal`   - `(shape, world size, emission, seed)`
//!
//! A batch is flushed whenever it fills, which keeps every draw call inside the
//! `u16` index range while preserving submission order.

use macroquad::prelude::*;

/// Vertices per flush. Macroquad indexes vertices with `u16`, so a batch must
/// stay well inside `u16::MAX`.
const VERTICES_PER_BATCH: usize = 16_384;

/// Everything the sprite fragment stage needs to draw one quad.
///
/// Bundling these keeps the batch API within the project's argument-count limit
/// and makes each call site read as a description of the sprite it draws.
#[derive(Clone, Copy)]
pub(super) struct Sprite {
    /// Centre in world units.
    pub(super) center: Vec2,
    /// Half-extents in world units.
    pub(super) half_size: Vec2,
    pub(super) color: Color,
    /// Shape id, matching the constants in the sprite shader.
    pub(super) shape: f32,
    /// The sprite's world size, which the shader normalises its local space by.
    pub(super) size: f32,
    /// How much of the sprite glows, from 0 to 1.
    pub(super) emission: f32,
    /// Per-instance animation seed.
    pub(super) seed: f32,
    /// Rotation about the centre, in radians. Only used by some shapes.
    pub(super) rotation: f32,
}

impl Sprite {
    pub(super) fn at(center: Vec2, half_size: Vec2, color: Color, shape: f32, size: f32) -> Self {
        Self {
            center,
            half_size,
            color,
            shape,
            size,
            emission: 0.0,
            seed: 0.0,
            rotation: 0.0,
        }
    }

    /// Rotated about its centre.
    pub(super) fn rotated(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Glowing, by a fraction of full emission.
    pub(super) fn emitting(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }

    /// Animated from a per-instance seed.
    pub(super) fn seeded(mut self, seed: f32) -> Self {
        self.seed = seed;
        self
    }
}

/// Accumulates quads and submits them as a single mesh.
pub(super) struct SpriteBatch {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl SpriteBatch {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(VERTICES_PER_BATCH),
            indices: Vec::with_capacity(VERTICES_PER_BATCH / 2 * 3),
        }
    }

    /// Vertices accumulated so far.
    ///
    /// Exposed for the tests, which assert the packing invariants the sprite
    /// shader depends on: a zero depth coordinate and an indexable vertex range.
    #[cfg(test)]
    pub(super) fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// True once another quad would overflow the index range.
    pub(super) fn is_full(&self) -> bool {
        self.vertices.len() + 4 > VERTICES_PER_BATCH
    }

    pub(super) fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// Adds an axis-aligned quad.
    pub(super) fn push(&mut self, sprite: &Sprite) {
        let (hx, hy) = (sprite.half_size.x, sprite.half_size.y);
        let corners = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)];
        let base = self.vertices.len() as u16;
        for (x, y) in corners {
            self.push_vertex(sprite.center.x + x, sprite.center.y + y, x, y, sprite);
        }
        self.link(base);
    }

    /// Adds a quad whose texture coordinates span the unit square.
    ///
    /// The post-process stages address their inputs in normalised UV, and macroquad
    /// derives UVs from `texcoord`, so the corners are written explicitly rather
    /// than in world units.
    pub(super) fn push_uv(&mut self, center: Vec2, half_size: Vec2, color: Color) {
        let (hx, hy) = (half_size.x, half_size.y);
        let corners = [
            (-hx, -hy, 0.0, 0.0),
            (hx, -hy, 1.0, 0.0),
            (hx, hy, 1.0, 1.0),
            (-hx, hy, 0.0, 1.0),
        ];
        let base = self.vertices.len() as u16;
        for (x, y, u, v) in corners {
            self.vertices.push(Vertex {
                position: vec3(center.x + x, center.y + y, 0.0),
                uv: vec2(u, v),
                color: color.into(),
                normal: vec4(0.0, 1.0, 0.0, 0.0),
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Adds a quad rotated about its centre.
    pub(super) fn push_rotated(&mut self, sprite: &Sprite) {
        let (sin, cos) = sprite.rotation.sin_cos();
        let (hx, hy) = (sprite.half_size.x, sprite.half_size.y);
        let corners = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)];
        let base = self.vertices.len() as u16;
        for (x, y) in corners {
            self.push_vertex(
                sprite.center.x + x * cos - y * sin,
                sprite.center.y + x * sin + y * cos,
                x,
                y,
                sprite,
            );
        }
        self.link(base);
    }

    /// Adds a quad spanning two world points, used for lightning segments.
    pub(super) fn push_segment(&mut self, sprite: &Sprite, from: Vec2, to: Vec2) {
        let delta = to - from;
        let length = delta.length();
        if length < 0.0001 {
            return;
        }
        let span = Sprite {
            center: from + delta * 0.5,
            half_size: vec2(length * 0.5, sprite.half_size.y),
            rotation: delta.y.atan2(delta.x),
            ..*sprite
        };
        self.push_rotated(&span);
    }

    /// Writes one corner vertex carrying its sprite's parameters.
    fn push_vertex(&mut self, x: f32, y: f32, u: f32, v: f32, sprite: &Sprite) {
        self.vertices.push(Vertex {
            position: vec3(x, y, 0.0),
            uv: vec2(u, v),
            color: sprite.color.into(),
            normal: vec4(sprite.shape, sprite.size, sprite.emission, sprite.seed),
        });
    }

    /// Adds the two triangles for a quad whose first vertex is at `base`.
    fn link(&mut self, base: u16) {
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Submits the accumulated quads, keeping the buffers for reuse.
    pub(super) fn submit(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        let mesh = Mesh {
            vertices: std::mem::take(&mut self.vertices),
            indices: std::mem::take(&mut self.indices),
            texture: None,
        };
        draw_mesh(&mesh);
        self.vertices = mesh.vertices;
        self.indices = mesh.indices;
        self.vertices.clear();
        self.indices.clear();
    }
}
