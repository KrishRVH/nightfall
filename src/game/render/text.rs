//! Bounded font raster sizes and preparation before frame drawing.
//!
//! Macroquad caches glyphs by character and pixel size. Growing its atlas while
//! text is already queued can invalidate those batches, so prepare every glyph
//! used by the game before drawing and scale those rasterized glyphs as needed.

use macroquad::prelude::{
    Color, TextDimensions, TextParams, camera_font_scale, draw_text_ex, get_default_font,
};

const RASTER_SIZES: [u16; 3] = [16, 32, 96];

/// Prepares the default font before drawing at a new display density.
#[derive(Default)]
pub(crate) struct TextCache {
    prepared_dpi_bits: Option<u32>,
}

impl TextCache {
    /// Caches all game glyphs before any frame drawing; repeats are inexpensive.
    ///
    /// `dpi_scale` is the finite, positive display density sampled by the
    /// application. Call this before queuing the frame's text drawing.
    pub(crate) fn prepare(&mut self, dpi_scale: f32) {
        debug_assert!(dpi_scale.is_finite() && dpi_scale > 0.0);
        let dpi_bits = dpi_scale.to_bits();
        if self.prepared_dpi_bits == Some(dpi_bits) {
            return;
        }

        let characters: Vec<_> = (' '..='~').chain(std::iter::once('·')).collect();
        let font = get_default_font();
        for size in RASTER_SIZES {
            // populate_font_cache accepts physical pixels; drawing applies DPI.
            let physical_size = (f32::from(size) * dpi_scale).ceil() as u16;
            font.populate_font_cache(&characters, physical_size);
        }
        self.prepared_dpi_bits = Some(dpi_bits);
    }
}

fn raster_size(size: f32) -> u16 {
    RASTER_SIZES
        .into_iter()
        .find(|raster| size <= f32::from(*raster))
        .unwrap_or(RASTER_SIZES[2])
}

fn text_params(size: f32) -> TextParams<'static> {
    let font_size = raster_size(size);
    TextParams {
        font_size,
        font_scale: size / f32::from(font_size),
        ..TextParams::default()
    }
}

pub(super) fn draw_text(text: impl AsRef<str>, x: f32, baseline: f32, size: f32, color: Color) {
    draw_text_ex(
        text,
        x,
        baseline,
        TextParams {
            color,
            ..text_params(size)
        },
    );
}

pub(super) fn measure_text(text: &str, size: f32) -> TextDimensions {
    let params = text_params(size);
    macroquad::text::measure_text(text, None, params.font_size, params.font_scale)
}

pub(super) fn fitted_text_size(text: &str, mut size: f32, width: f32) -> f32 {
    for _ in 0..RASTER_SIZES.len() {
        let previous_raster = raster_size(size);
        let measured = measure_text(text, size).width.max(1.0);
        size = (size * (width.max(1.0) / measured).min(1.0)).max(1.0);
        // DPI rounds physical rasters differently, so refit after a tier change.
        if raster_size(size) == previous_raster {
            break;
        }
    }
    size
}

pub(super) fn draw_text_center(text: &str, center_x: f32, baseline: f32, size: f32, color: Color) {
    let width = measure_text(text, size).width;
    draw_text(text, center_x - width * 0.5, baseline, size, color);
}

pub(super) fn draw_text_right(text: &str, right: f32, baseline: f32, size: f32, color: Color) {
    let width = measure_text(text, size).width;
    draw_text(text, right - width, baseline, size, color);
}

pub(super) fn draw_world_text_center(
    text: &str,
    center_x: f32,
    baseline: f32,
    size: f32,
    color: Color,
) {
    let (screen_size, scale, aspect) = camera_font_scale(size);
    let font_size = raster_size(f32::from(screen_size));
    let font_scale = scale * f32::from(screen_size) / f32::from(font_size);
    let width = macroquad::text::measure_text(text, None, font_size, font_scale).width * aspect;
    draw_text_ex(
        text,
        center_x - width * 0.5,
        baseline,
        TextParams {
            font_size,
            font_scale,
            font_scale_aspect: aspect,
            color,
            ..TextParams::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::{RASTER_SIZES, text_params};

    #[test]
    fn arbitrary_text_sizes_use_only_prepared_rasters() {
        for size in [1.0, 13.0, 16.0, 16.5, 23.75, 32.0, 32.5, 72.0, 96.0, 192.0] {
            let params = text_params(size);
            assert!(RASTER_SIZES.contains(&params.font_size));
            assert!((f32::from(params.font_size) * params.font_scale - size).abs() < 0.000_01);
        }
    }
}
