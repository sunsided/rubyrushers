//! Screenshots as RGB pixels.

use anyhow::{Context, Result, bail};

/// An RGB image, row-major.
#[derive(Clone, Debug)]
pub struct Image {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Three bytes per pixel.
    pub rgb: Vec<u8>,
}

impl Image {
    /// An image of `width` x `height` filled with `color`.
    pub fn filled(width: usize, height: usize, color: [u8; 3]) -> Self {
        Self {
            width,
            height,
            rgb: color.repeat(width * height),
        }
    }

    /// Decodes a PNG (RGB or RGBA, 8 bit after expansion).
    pub fn from_png(bytes: &[u8]) -> Result<Self> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().context("PNG header")?;
        let mut buf = vec![0; reader.output_buffer_size().context("PNG too large")?];
        let info = reader.next_frame(&mut buf).context("PNG data")?;
        let (w, h) = (info.width as usize, info.height as usize);
        let step = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected PNG colour type {other:?}"),
        };
        let rgb = buf[..w * h * step]
            .chunks_exact(step)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        Ok(Self {
            width: w,
            height: h,
            rgb,
        })
    }

    /// The pixel at (`x`, `y`).
    pub fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.width + x) * 3;
        [self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]]
    }

    /// Sets the pixel at (`x`, `y`).
    pub fn set(&mut self, x: usize, y: usize, c: [u8; 3]) {
        let i = (y * self.width + x) * 3;
        self.rgb[i..i + 3].copy_from_slice(&c);
    }
}

/// Hue in degrees `0..360`, saturation and value in `0..=1`.
pub fn hsv(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|v| f32::from(v) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hues_of_primaries() {
        assert_eq!(hsv([255, 0, 0]).0, 0.0);
        assert_eq!(hsv([0, 255, 0]).0, 120.0);
        assert_eq!(hsv([0, 0, 255]).0, 240.0);
        assert_eq!(hsv([128, 128, 128]).1, 0.0);
    }
}
