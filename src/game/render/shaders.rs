//! GLSL ES 1.00 sources for the deferred renderer.
//!
//! Macroquad compiles custom materials with `#version 100` and a fixed vertex
//! layout, so every shader here declares the same four attributes:
//!
//! - `position` - world or screen position; `.z` is a manual sort key
//! - `texcoord` - local offset from a sprite centre, or world space for the
//!   ground quad
//! - `color0`   - packed RGBA tint
//! - `normal`   - per-vertex parameters: shape id, scale, emission, seed
//!
//! Macroquad injects `Model`, `Projection` and `_Time` ahead of every uniform
//! declared in `MaterialParams`, so custom uniforms start at slot three.
//!
//! Screen-space derivatives are avoided deliberately: `GLSL` needs an extension
//! for them on every backend this game targets, so antialias widths arrive as the
//! `u_pixel` uniform instead, computed exactly on the CPU.
//!
//! GLSL has no preprocessor includes and macroquad exposes none, so the helper
//! libraries below are concatenated onto the shader bodies at load time.

/// Sprite shape ids, shared by the shader and the scene builder. The values must
/// match the constants inside the sprite fragment stage.
pub(super) const SHAPE_SHADE: f32 = 0.0;
pub(super) const SHAPE_WISP: f32 = 1.0;
pub(super) const SHAPE_BRUTE: f32 = 2.0;
pub(super) const SHAPE_WARDEN: f32 = 3.0;
pub(super) const SHAPE_GEM: f32 = 4.0;
pub(super) const SHAPE_BLADE: f32 = 5.0;
/// Soft occlusion ellipse used for contact shadows. Unlike the creature shapes
/// this one is unlit: it only darkens what is beneath it.
pub(super) const SHAPE_SOFT: f32 = 6.0;

/// Vertex stage shared by the sprite, ground and light materials.
pub(super) const SPRITE_VERTEX: &str = r"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;

uniform mat4 Model;
uniform mat4 Projection;

varying vec2 v_local;
varying vec2 v_world;
varying vec4 v_color;
varying vec4 v_params;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    v_local = texcoord;
    v_world = position.xy;
    // `color0` is bound as four unsigned bytes with `normalized = false`, so it
    // reaches the shader as 0..255 integers and has to be scaled here.
    v_color = color0 / 255.0;
    v_params = normal;
}
";

/// Vertex stage for passes that sample a screen-sized target.
///
/// The fragment stages also read `v_world` for the light pass, which resolves
/// lights from world positions rather than screen positions, so both varyings
/// are declared here.
pub(super) const PASS_VERTEX: &str = r"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;

uniform mat4 Model;
uniform mat4 Projection;

varying vec2 v_uv;
varying vec2 v_world;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    v_uv = texcoord;
    v_world = position.xy;
}
";

/// Hash and noise helpers for the ground.
pub(super) const NOISE: &str = r"
float hash21(vec2 p) {
    p = fract(p * vec2(127.317, 311.7));
    p += dot(p, p + 34.213);
    return fract(p.x * p.y);
}

vec2 hash22(vec2 p) {
    vec2 q = vec2(dot(p, vec2(127.317, 311.7)), dot(p, vec2(269.513, 183.117)));
    return fract(sin(q) * 43758.5453);
}

float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    // Quintic interpolant: the second-derivative continuity stops the lighting
    // from creasing along cell borders the way plain smoothstep would.
    vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p) {
    float sum = 0.0;
    float amplitude = 0.5;
    mat2 rotation = mat2(0.8, 0.6, -0.6, 0.8);
    for (int octave = 0; octave < 4; octave++) {
        sum += amplitude * vnoise(p);
        p = rotation * p * 2.03;
        amplitude *= 0.5;
    }
    return sum;
}
";

/// Signed-distance helpers for the sprite shapes.
pub(super) const SDF: &str = r"
float sdEllipse(vec2 p, vec2 radius) {
    float k0 = length(p / radius);
    float k1 = length(p / (radius * radius));
    return k0 * (k0 - 1.0) / max(k1, 0.0001);
}

float sdBox(vec2 p, vec2 half_size) {
    vec2 d = abs(p) - half_size;
    return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
}

float sdSegment(vec2 p, vec2 a, vec2 b, float radius) {
    vec2 pa = p - a;
    vec2 ba = b - a;
    float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - radius;
}

float sdTriangle(vec2 p, vec2 a, vec2 b, vec2 c) {
    vec2 e0 = b - a;
    vec2 e1 = c - b;
    vec2 e2 = a - c;
    vec2 v0 = p - a;
    vec2 v1 = p - b;
    vec2 v2 = p - c;
    vec2 pq0 = v0 - e0 * clamp(dot(v0, e0) / dot(e0, e0), 0.0, 1.0);
    vec2 pq1 = v1 - e1 * clamp(dot(v1, e1) / dot(e1, e1), 0.0, 1.0);
    vec2 pq2 = v2 - e2 * clamp(dot(v2, e2) / dot(e2, e2), 0.0, 1.0);
    float s = sign(e0.x * e2.y - e0.y * e2.x);
    vec2 d = min(min(vec2(dot(pq0, pq0), s * (v0.x * e0.y - v0.y * e0.x)),
                     vec2(dot(pq1, pq1), s * (v1.x * e1.y - v1.y * e1.x))),
                     vec2(dot(pq2, pq2), s * (v2.x * e2.y - v2.y * e2.x)));
    return -sqrt(d.x) * sign(d.y);
}
";

/// Procedural graveyard paving.
///
/// One world-space quad replaces the previous per-cell draw loop, so the
/// surface is continuous, resolution-independent and lit rather than tiled.
pub(super) const GROUND_BODY: &str = r"
varying vec2 v_world;
uniform vec4 _Time;
uniform float u_pixel;

const float CELL = 118.0;

// Outward direction of a rounded box, used to bevel its edges.
//
// The direction is taken from the outside corner region and interpolated toward
// the centre, which avoids the axis-aligned crease that a hard inside/outside
// branch would draw across the middle of every flagstone.
vec2 box_gradient(vec2 p, vec2 half_size) {
    vec2 d = abs(p) - half_size;
    vec2 outer = max(d, 0.0);
    float outside = length(outer);
    if (outside > 0.0) {
        return sign(p) * outer / outside;
    }
    // Inside: blend the two axis directions by which is closer to the border.
    vec2 axis = (d.x > d.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
    vec2 other = (d.x > d.y) ? vec2(0.0, 1.0) : vec2(1.0, 0.0);
    float blend = (d.x > d.y) ? d.x : d.y;
    vec2 direction = mix(other * sign(p), axis * sign(p), blend * 3.0);
    return normalize(direction);
}

void main() {
    vec2 world = v_world;
    float time = _Time.x;

    // Stones nearly fill their cell, so the joints are thin seams rather than
    // the wide black channels a half-extent of 0.30 produced.
    float aa = u_pixel / CELL;
    vec2 cell = floor(world / CELL);
    vec2 local = fract(world / CELL) - 0.5;

    // Jitter and rotate each stone so the paving never reads as a lattice.
    vec2 jitter = (hash22(cell) - 0.5) * 0.07;
    vec2 stone = local - jitter;

    float spin = (hash21(cell + 3.17) - 0.5) * 0.07;
    float cs = cos(spin);
    float sn = sin(spin);
    stone = mat2(cs, -sn, sn, cs) * stone;

    // Half-extents reach almost to the cell edge, leaving a narrow joint.
    vec2 variation = hash22(cell + 19.7) - 0.5;
    vec2 half_size = vec2(0.470) + variation * 0.012;

    vec2 d = abs(stone) - half_size;
    float edge = length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);

    // The joint is a narrow graded recess. `edge` is negative inside a stone and
    // positive outside it, so the ramp runs from fully stone to fully joint over
    // a few centimetres of world space; a wider skirt had been washing darkness
    // across the body of each flagstone.
    float joint = 0.020;
    float mortar = smoothstep(-joint, joint, edge);
    float inside = 1.0 - mortar;


    // Bevel profile: stones dome up from their borders, the joint sits lowest.
    float depth = clamp(-edge / 0.13, 0.0, 1.0);
    float dome = sqrt(depth);

    // Bevel shading: stones are brighter along the edge that catches the moon,
    // which reads as a worn, rounded lip.
    vec2 gradient = box_gradient(stone, half_size);
    float lip = (1.0 - dome) * inside * max(-dot(gradient, vec2(0.6, 0.8)), 0.0);


    // This stage emits reflectance, which the composite multiplies by
    // `ambient + light`. Keeping it inside 0..1 is what lets the bloom threshold
    // separate lit surfaces from genuinely glowing ones.
    float grain = vnoise(world * 1.1);
    float mottle = fbm(world * 0.055);

    vec3 stone_dark = vec3(0.115, 0.104, 0.152);
    vec3 stone_light = vec3(0.215, 0.196, 0.262);
    vec3 base = mix(stone_dark, stone_light, mottle * 0.75 + grain * 0.25);
    base *= 0.88 + hash21(cell + 5.5) * 0.24;

    vec3 mortar_col = vec3(0.098, 0.090, 0.132) * (0.85 + grain * 0.35);
    vec3 albedo = mix(base, mortar_col, mortar);

    // Moss creeps out of the joints where the noise mask allows it.
    float moss_mask = smoothstep(0.54, 0.88, fbm(world * 0.022 + 5.0)) * mortar;
    albedo = mix(albedo, vec3(0.072, 0.118, 0.086), moss_mask * 0.8);

    // Cracks: thin dark veins following the noise field's own iso-lines.
    float veins = abs(fbm(world * 0.014 + 11.0) - 0.5);
    float crack = 1.0 - smoothstep(0.0, 0.022, veins);
    albedo *= 1.0 - crack * 0.42 * inside;

    float shade = 0.88 + dome * 0.12 + lip * 0.16;
    vec3 color = albedo * shade;

    // Ambient occlusion in the joints, plus a slowly drifting mist veil.
    color *= 1.0 - mortar * 0.10;
    float mist = fbm(world * 0.0016 + vec2(time * 0.011, time * 0.006));
    color += vec3(0.020, 0.023, 0.036) * smoothstep(0.42, 0.95, mist);
    gl_FragColor = vec4(color, 1.0);

}
";

/// Signed-distance creatures and effects.
///
/// `v_params.x` selects a shape, `.y` is the sprite's world size, `.z` drives
/// emission and `.w` is a per-instance animation seed. Each shape is shaded from
/// the gradient of its own distance field, which gives the sprites real form and
/// a rim light instead of flat fills.
pub(super) const SPRITE_BODY: &str = r"
varying vec2 v_local;
varying vec4 v_color;
varying vec4 v_params;

uniform vec4 _Time;
uniform float u_pixel;

const float SHAPE_SHADE = 0.0;
const float SHAPE_WISP = 1.0;
const float SHAPE_BRUTE = 2.0;
const float SHAPE_WARDEN = 3.0;
const float SHAPE_GEM = 4.0;
const float SHAPE_BLADE = 5.0;
const float SHAPE_SOFT = 6.0;

// Distance from the leading edge, used for the warden's shoulder highlight.
float across_abs(float x, float seed) {
    return clamp(abs(x - seed * 0.07) * 3.2, 0.0, 1.0);
}

// Polynomial smooth minimum: unions two distance fields without the crease a
// plain `min` leaves where they meet.
float smooth_min(float a, float b, float radius) {
    float h = clamp(0.5 + 0.5 * (b - a) / radius, 0.0, 1.0);
    return mix(b, a, h) - radius * h * (1.0 - h);
}

float shape_sdf(vec2 p, float shape, float seed, float time) {
    if (shape == SHAPE_SHADE) {
        // A tattered ghost. The silhouette is a rounded cowl over a body whose
        // hem is cut by three travelling waves, so the ghost frays into
        // ribbons that trail and sway as it drifts.
        float shoulders = length((p - vec2(0.0, -0.18)) * vec2(1.0, 0.62)) - 0.36;
        float body = sdEllipse(p - vec2(0.0, 0.14), vec2(0.30, 0.40));
        float shape_d = smooth_min(shoulders, body, 0.12);

        // Hem: three detuned waves, so the trailing edge is ragged rather than
        // a single clean scallop.
        float hem = p.y - 0.52
                  + sin(p.x * 5.5 + time * 2.0 + seed) * 0.085
                  + sin(p.x * 11.0 - time * 1.4 + seed * 2.0) * 0.045
                  + sin(p.x * 21.0 + time * 3.1) * 0.018;
        return max(shape_d, hem);
    }
    if (shape == SHAPE_WISP) {
        // A flame: a rounded body that narrows into a tail, with a wobbling
        // edge so the silhouette ripples as it drifts.
        float wobble = sin(p.y * 5.5 - time * 5.0 + seed) * 0.070
                     + sin(p.y * 11.0 + time * 3.1) * 0.030;
        float taper = 0.40 + smoothstep(0.55, -0.45, p.y) * 0.26;
        float body = length(vec2(p.x, p.y * 0.78 + wobble)) - taper;
        // Clip the bottom so the tail reads as a point rather than a ball.
        return max(body, p.y - 0.62);
    }
    if (shape == SHAPE_BRUTE) {
        float mass = sdEllipse(p - vec2(0.0, 0.10), vec2(0.66, 0.54));
        float shoulders = sdBox(p - vec2(0.0, 0.34), vec2(0.46, 0.20));
        float horn_left = sdSegment(p, vec2(-0.40, -0.16), vec2(-0.62, -0.58), 0.085);
        float horn_right = sdSegment(p, vec2(0.40, -0.16), vec2(0.62, -0.58), 0.085);
        return min(min(mass, shoulders), min(horn_left, horn_right));
    }
    if (shape == SHAPE_WARDEN) {
        // The warden, built as three round masses: a robe, shoulders that merge
        // into it, and a cowl. The proportions are the whole design — the cowl is
        // about a third of the robe's width, and it sits centred above the
        // shoulders rather than off to one side.
        float lean = seed * 0.10;

        // Robe: a tall bell falling from the shoulders to the hem.
        vec2 robe_p = p - vec2(0.0, 0.20);
        float robe = sdEllipse(robe_p, vec2(0.30, 0.46));
        robe = max(robe, p.y - 0.66 + sin(p.x * 8.0 + time * 1.7 + seed) * 0.040);

        // Shoulders: a wide, flat mass across the top of the robe.
        float shoulders = length((p - vec2(0.0, -0.10)) * vec2(0.62, 1.15)) - 0.215;

        // Cowl: clearly narrower than the shoulders, centred above them.
        float hood = length((p - vec2(lean, -0.40)) * vec2(1.0, 0.95)) - 0.150;

        // The cowl's peak, swept back from the facing direction.
        float peak = sdSegment(p, vec2(lean, -0.46), vec2(-lean * 3.0, -0.66), 0.060);

        // `smooth_min` keeps the joins between masses from creasing, which would
        // otherwise shade as visible seams across the body.
        float body = smooth_min(robe, shoulders, 0.10);
        float head = smooth_min(hood, peak, 0.05);
        return smooth_min(body, head, 0.045);
    }
    if (shape == SHAPE_GEM) {
        // Faceted rhombus: four triangles meeting at the girdle.
        float front = sdTriangle(p, vec2(0.0, -0.92), vec2(0.56, -0.06), vec2(0.0, 0.86));
        float right = sdTriangle(p, vec2(0.56, -0.06), vec2(0.0, 0.86), vec2(0.0, -0.92));
        float back = sdTriangle(p, vec2(0.0, -0.92), vec2(-0.56, -0.06), vec2(0.0, 0.86));
        float left = sdTriangle(p, vec2(-0.56, -0.06), vec2(0.0, 0.86), vec2(0.0, -0.92));
        return min(min(front, right), min(back, left));
    }
    if (shape == SHAPE_BLADE) {
        // Crescent moon: an outer disc minus an offset disc.
        return max(length(p) - 0.86, -(length(p - vec2(0.40, 0.0)) - 0.72));
    }
    return length(p) - 0.92;
}

void main() {
    float scale = max(v_params.y, 0.0001);
    vec2 p = v_local / scale;
    float shape = v_params.x;
    float seed = v_params.w;
    float time = _Time.x;
    float emission = v_params.z;

    // Antialias width travels with the sprite, so coverage stays crisp at any
    // world scale.
    float aa = max(u_pixel / scale, 0.0005);

    // Contact shadows are pure occlusion: a flattened ellipse whose alpha falls
    // off smoothly from the centre. Nothing about them is lit.
    if (shape == SHAPE_SOFT) {
        float distance = length(p * vec2(1.0, 2.15));
        float falloff = clamp(1.0 - distance, 0.0, 1.0);
        float alpha = falloff * falloff * v_color.a;
        alpha *= 1.0 - smoothstep(1.0 - aa * 2.15, 1.0 + aa * 2.15, distance);
        if (alpha <= 0.003) {
            discard;
        }
        gl_FragColor = vec4(0.0, 0.0, 0.0, alpha);
        return;
    }

    float d = shape_sdf(p, shape, seed, time);
    float coverage = 1.0 - smoothstep(-aa, aa, d);
    if (coverage <= 0.003) {
        discard;
    }

    // The distance-field gradient gives every sprite a shaded surface, at the
    // cost of two extra evaluations of `shape_sdf`. The step is a fraction of the
    // antialias width so it stays well below the crescent's thin features, where a
    // wider tap would sample the far side of the shape.
    float probe = max(aa * 0.5, 0.004);
    vec2 offset = vec2(probe, 0.0);
    float dx = shape_sdf(p + offset.xy, shape, seed, time);
    float dy = shape_sdf(p + offset.yx, shape, seed, time);
    vec2 gradient = vec2(dx - d, dy - d) / probe;
    float slope = clamp(length(gradient), 0.0, 1.5);
    vec2 direction = length(gradient) > 0.0001 ? normalize(gradient) : vec2(0.0, -1.0);
    vec3 normal = normalize(vec3(direction * slope, 1.0));

    vec3 key = normalize(vec3(-0.45, -0.74, 0.50));
    vec3 rim_light = normalize(vec3(0.35, -0.55, 0.76));
    float lambert = max(dot(normal, key), 0.0);
    float rim = pow(1.0 - clamp(-d / 0.55, 0.0, 1.0), 2.4) * max(dot(normal, rim_light), 0.0);

    // Cores run hot, edges fall away into the dark.
    float interior = clamp(-d / 0.75, 0.0, 1.0);

    vec3 half_dir = normalize(key + vec3(0.0, 0.0, 1.0));
    float specular = pow(max(dot(normal, half_dir), 0.0), mix(18.0, 64.0, emission));
    // Emission brightens a sprite in proportion to how much of it is emissive.
    vec3 color = v_color.rgb * mix(0.30, 1.00, pow(interior, 0.85)) * (1.0 + emission * 0.55);
    // A shared glint, applied before the shape branches so they can modulate it.
    color += vec3(0.85, 0.90, 1.0) * specular * 0.16;

    // Each creature wants a different interior: a shade is a translucent shell, a
    // wisp burns from a hot core, a brute is solid armour, and a gem is faceted.
    // Adjusting the shared ramp per shape had them fighting each other, so every
    // shape recomputes its colour here instead, from this shared diffuse term.
    float diffuse = 0.34 + lambert * 0.95;

    if (shape == SHAPE_SHADE) {
        // A shell: dim through the middle, bright and hard at the rim.
        vec3 body_tint = v_color.rgb * mix(0.45, 1.15, pow(interior, 1.2));
        color = body_tint * diffuse
              + v_color.rgb * rim * 1.40
              + v_color.rgb * 0.30;
        // Faint striations drifting with the hem.
        float striation = smoothstep(0.90, 1.0,
            sin(p.x * 8.0 + time * 1.4 + seed) * 0.5 + 0.5);
        color += v_color.rgb * striation * 0.22;
    } else if (shape == SHAPE_WISP) {
        // A flame: a small, very hot core cooling fast to a thin edge.
        vec3 body_tint = v_color.rgb * mix(0.35, 1.55, pow(interior, 1.9));
        color = body_tint * diffuse
              + mix(v_color.rgb, vec3(0.90, 1.00, 1.00), interior) * rim * 0.80;
        color += v_color.rgb * 0.45;
    } else if (shape == SHAPE_BRUTE) {
        // Armour: hard, dark at the underside, with a tight specular.
        vec3 body_tint = v_color.rgb * mix(0.22, 0.95, pow(interior, 0.40));
        color = body_tint * diffuse
              + vec3(1.00, 0.82, 0.86) * specular * 0.70
              + v_color.rgb * rim * 0.50;
    } else if (shape == SHAPE_GEM) {
        // Faceted: brightness steps between planes so each catches the moon
        // differently, which is what makes a gem read as cut.
        float facet = fract(floor((p.y * 0.5 + 0.5) * 3.0) * 0.618);
        vec3 body_tint = v_color.rgb * mix(0.50, 1.45, facet);
        color = body_tint * diffuse
              + v_color.rgb * rim * 1.00
              + v_color.rgb * 0.35;
    } else if (shape == SHAPE_BLADE) {
        // A crescent is too thin for the shared interior ramp, which saturates to
        // a flat fill. Light it from its own edge distance instead: a hot leading
        // edge fading into the dark hollow.
        float blade = clamp(-d / 0.10, 0.0, 1.0);
        vec3 steel = mix(v_color.rgb * 0.16, v_color.rgb * 0.66, blade);
        vec3 edge = mix(vec3(0.96, 0.91, 0.79), v_color.rgb, blade) * pow(blade, 2.2);
        color = steel + edge * 0.55;
    } else if (shape == SHAPE_WARDEN) {
        // The warden: broad shading, then a clear lift of its own so the player
        // stays the most legible thing on screen.
        color *= diffuse;
        color += v_color.rgb * 0.32;
    } else {
        color *= diffuse;
        color += v_color.rgb * 0.14;
    }

    // The warden's face, eyes and hood highlight.
    if (shape == SHAPE_WARDEN) {
        vec2 hood = vec2(seed * 0.10, -0.40);
        const float HOOD_R = 0.150;

        // Face void filling the cowl, inset so a band of lit hood survives.
        float face = length((p - hood - vec2(seed * 0.022, 0.014)) * vec2(1.0, 1.25)) - 0.082;
        // Near-black rather than merely darker: the void is what makes the figure
        // read as hooded instead of as a pale blob with a smudge on it.
        color = mix(vec3(0.030, 0.026, 0.055), color, smoothstep(-0.025, 0.035, face));

        // Eye lights inside the void.
        vec2 eye_c = hood + vec2(seed * 0.022, 0.038);
        float eye = min(
            length((p - eye_c - vec2(-0.034, 0.0)) * vec2(1.0, 2.1)),
            length((p - eye_c - vec2(0.034, 0.0)) * vec2(1.0, 2.1))
        ) - 0.014;
        color += vec3(0.90, 0.94, 1.0) * (1.0 - smoothstep(0.0, 0.016, eye)) * 1.5;

        // A thin lit rim where the cowl meets the void, and a highlight along the
        // cowl's own edge, so the head reads as a form rather than a hole.
        float hood_edge = length((p - hood) * vec2(1.0, 0.95)) - HOOD_R;
        color += v_color.rgb * (1.0 - smoothstep(0.0, 0.030, abs(hood_edge + 0.030))) * 0.30;
        color += v_color.rgb * (1.0 - smoothstep(0.0, 0.016, abs(face + 0.020))) * 0.26;

        // Shoulder rim on the leading edge of the robe.
        color += v_color.rgb * pow(across_abs(p.x, seed), 6.0) * 0.26;
    }

    gl_FragColor = vec4(color, coverage * v_color.a);
}
";

/// Additive glow sprites: soft radial falloff only.
pub(super) const GLOW_FRAGMENT: &str = r"#version 100
precision highp float;

varying vec2 v_local;
varying vec4 v_color;
varying vec4 v_params;

uniform float u_pixel;

void main() {
    float scale = max(v_params.y, 0.0001);
    vec2 p = v_local / scale;
    float radius = length(p);
    float aa = max(u_pixel / scale, 0.0005);
    float falloff = 1.0 - smoothstep(0.0, 1.0, radius);
    // Squaring then re-smoothing concentrates the core and leaves a faint skirt.
    float core = falloff * falloff * (3.0 - 2.0 * falloff);
    float alpha = core * v_color.a;
    if (alpha <= 0.003) {
        discard;
    }
    gl_FragColor = vec4(v_color.rgb * alpha, alpha);
}
";

/// Accumulates the frame's dynamic lights into a half-resolution buffer.
///
/// One quad evaluates every light analytically instead of drawing a sprite per
/// light, so dozens of emitters still cost a single draw call.
pub(super) const LIGHT_FRAGMENT: &str = r"#version 100
precision highp float;

varying vec2 v_world;
uniform vec4 u_lights[24];
uniform vec4 u_light_colors[24];
uniform float u_light_count;

void main() {
    vec3 total = vec3(0.0);
    for (int index = 0; index < 24; index++) {
        if (float(index) >= u_light_count) {
            break;
        }
        vec4 light = u_lights[index];
        vec4 tint = u_light_colors[index];
        float distance = length(v_world - light.xy);
        float reach = max(light.z, 0.001);
        // Quadratic falloff plus a soft core, so lights bloom rather than
        // showing a hard edge.
        float attenuation = clamp(1.0 - distance / reach, 0.0, 1.0);
        attenuation *= attenuation;
        float core = clamp(1.0 - distance / (reach * 0.28), 0.0, 1.0);
        total += tint.rgb * (attenuation * 0.72 + core * core * 0.40) * tint.a * light.w;
    }
    // A bounded multiplier: overlapping emitters stack, and nothing downstream
    // can cope with an unbounded value.
    gl_FragColor = vec4(min(total, vec3(1.30)), 1.0);
}
";

/// Bright pass with a soft knee; also downsamples to the bloom resolution.
pub(super) const BRIGHT_FRAGMENT: &str = r"#version 100
precision highp float;

varying vec2 v_uv;
uniform sampler2D u_scene;
uniform sampler2D u_light;
uniform float u_threshold;

void main() {
    // The world targets store world +Y (down) at the bottom of the image, so
    // they are read upside down relative to screen space. The bloom buffer is
    // written in screen orientation and is therefore sampled directly.
    vec2 world_uv = vec2(v_uv.x, 1.0 - v_uv.y);

    vec3 scene = texture2D(u_scene, world_uv).rgb;
    vec3 light = texture2D(u_light, world_uv).rgb;
    // The bright pass must apply the same ceiling the composite does. Summing
    // many overlapping emitters unclamped produced values in the tens, and the
    // knee pinned to 1.0 turned that into a full-strength bloom that washed the
    // whole lit area to white.
    vec3 lit = min(vec3(0.55, 0.60, 0.76) + light, vec3(1.75));
    vec3 color = scene * lit + light * 0.12;
    float brightness = max(max(color.r, color.g), color.b);
    // Quadratic knee keeps the bloom onset from banding.
    float knee = clamp((brightness - u_threshold) / max(u_threshold, 0.001), 0.0, 1.0);
    gl_FragColor = vec4(color * knee * knee, 1.0);
}
";

/// Separable Gaussian blur, five taps in one direction.
pub(super) const BLUR_FRAGMENT: &str = r"#version 100
precision highp float;

varying vec2 v_uv;
uniform sampler2D u_source;
uniform vec2 u_direction;

void main() {
    vec3 sum = texture2D(u_source, v_uv).rgb * 0.227027;
    sum += texture2D(u_source, v_uv + u_direction * 1.3846).rgb * 0.316216;
    sum += texture2D(u_source, v_uv - u_direction * 1.3846).rgb * 0.316216;
    sum += texture2D(u_source, v_uv + u_direction * 3.2308).rgb * 0.070270;
    sum += texture2D(u_source, v_uv - u_direction * 3.2308).rgb * 0.070270;
    gl_FragColor = vec4(sum, 1.0);
}
";

/// Final grade: lighting combine, bloom, tonemap, aberration and grain.
pub(super) const COMPOSITE_FRAGMENT: &str = r"#version 100
precision highp float;

varying vec2 v_uv;
uniform sampler2D u_scene;
uniform sampler2D u_light;
uniform sampler2D u_bloom;
uniform vec4 _Time;
uniform vec2 u_resolution;
uniform float u_bloom_strength;
uniform float u_hurt;
uniform float u_flash;
uniform float u_health;

vec3 aces(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
    // World targets are stored upside down relative to screen space; the bloom
    // buffer is not.
    vec2 world_uv = vec2(v_uv.x, 1.0 - v_uv.y);
    vec2 uv = v_uv;

    vec2 centred = uv - 0.5;
    float radius = length(centred);

    // Lateral chromatic aberration, strongest at the frame edge.
    vec2 shift = centred * radius * 0.0034;
    vec3 scene;
    scene.r = texture2D(u_scene, world_uv + shift).r;
    scene.g = texture2D(u_scene, world_uv).g;
    scene.b = texture2D(u_scene, world_uv - shift).b;

    vec3 light;
    light.r = texture2D(u_light, world_uv + shift).r;
    light.g = texture2D(u_light, world_uv).g;
    light.b = texture2D(u_light, world_uv - shift).b;

    // Ambient is the floor every pixel sits at; the light buffer lifts whatever
    // the lantern, blades and spell actually reach. The sum is clamped so
    // overlapping emitters cannot blow the frame out before tonemapping.
    // The light buffer is a multiplier, so overlapping emitters are summed under
    // a ceiling; without one, two nearby lights saturate the surface to white.
    vec3 ambient = vec3(0.50, 0.56, 0.74);
    vec3 color = scene * min(ambient + light, vec3(1.75));
    color += texture2D(u_bloom, uv).rgb * u_bloom_strength;

    color = aces(color * 0.92);

    // Night grade: cool shadows, warm highlights, gentle S-curve.
    float luma = dot(color, vec3(0.2126, 0.7152, 0.0722));
    vec3 shadow_tint = vec3(0.86, 0.93, 1.16);
    vec3 high_tint = vec3(1.07, 1.00, 0.90);
    color *= mix(shadow_tint, high_tint, smoothstep(0.02, 0.55, luma));
    color = clamp((color - 0.5) * 1.075 + 0.5, 0.0, 1.0);
    color = mix(vec3(luma), color, 1.14);

    // Low health bleeds a pulsing red into the frame edges.
    float peril = smoothstep(0.34, 0.0, u_health) * smoothstep(0.18, 0.62, radius);
    float pulse = 0.5 + 0.5 * sin(_Time.x * 6.4);
    color = mix(color, color * vec3(1.5, 0.42, 0.46), peril * (0.30 + pulse * 0.28));
    color += vec3(0.55, 0.10, 0.12) * u_hurt * 0.55;
    color += vec3(0.95, 0.88, 0.95) * u_flash * 0.40;

    float vignette = smoothstep(1.25, 0.40, radius);
    color *= 0.72 + vignette * 0.28;

    // Fine grain keeps the large flat gradients from banding.
    float grain = fract(sin(dot(uv * u_resolution + _Time.x * 61.0,
                                 vec2(12.9898, 78.233))) * 43758.5453);
    color += (grain - 0.5) * 0.028;


    gl_FragColor = vec4(clamp(color, 0.0, 1.0), 1.0);
}
";

/// The GLSL ES version every material targets.
const VERSION: &str = "#version 100\n";

/// Shared preamble. GLSL ES requires a precision qualifier for every float type
/// used in a fragment stage, and it has to appear before the first declaration.
const PREAMBLE: &str = "precision highp float;\n";

/// Assembles the ground fragment stage from its helper library.
pub(super) fn ground_fragment() -> String {
    format!("{VERSION}{PREAMBLE}{NOISE}\n{GROUND_BODY}")
}

/// Assembles the sprite fragment stage from its signed-distance library.
pub(super) fn sprite_fragment() -> String {
    format!("{VERSION}{PREAMBLE}{SDF}\n{SPRITE_BODY}")
}
