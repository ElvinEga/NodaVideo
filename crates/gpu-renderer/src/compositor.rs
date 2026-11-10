//! Software CPU compositor providing offline, deterministic rendering and fallback execution.

use nodavideo_timeline::{ClipTransform, TransitionType};

use crate::error::GpuError;
use crate::types::RgbaColor;

/// Raw 32-bit RGBA pixel buffer in CPU memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelBuffer {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Interleaved RGBA8 pixel data (4 bytes per pixel).
    pub pixels: Vec<u8>,
}

impl PixelBuffer {
    /// Create a zero-initialized black/transparent pixel buffer.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let total_bytes = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![0; total_bytes],
        }
    }

    /// Create a pixel buffer filled with a solid color.
    #[must_use]
    pub fn from_color(width: u32, height: u32, color: RgbaColor) -> Self {
        let mut buf = Self::new(width, height);
        buf.fill(color);
        buf
    }

    /// Read pixel RGBA values at `(x, y)`.
    #[must_use]
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0, 0];
        }
        let index = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ]
    }

    /// Write pixel RGBA values at `(x, y)`.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x < self.width && y < self.height {
            let index = ((y * self.width + x) * 4) as usize;
            self.pixels[index..index + 4].copy_from_slice(&rgba);
        }
    }

    /// Fill the entire buffer with the specified color.
    pub fn fill(&mut self, color: RgbaColor) {
        let r = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
        let a = (color.a.clamp(0.0, 1.0) * 255.0) as u8;

        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
    }
}

/// Software compositor for applying 2D transforms, opacity blending, and transition effects.
#[derive(Debug, Clone, Copy, Default)]
pub struct SoftwareCompositor;

impl SoftwareCompositor {
    /// Composite a transformed source layer over a destination buffer using alpha blending.
    #[allow(clippy::similar_names, clippy::suboptimal_flops)]
    pub fn composite_layer(
        &self,
        dest: &mut PixelBuffer,
        src: &PixelBuffer,
        transform: &ClipTransform,
    ) -> Result<(), GpuError> {
        let opacity = transform.opacity as f32;
        if opacity <= 0.0 {
            return Ok(());
        }

        let sx = transform.scale_x as f32;
        let sy = transform.scale_y as f32;
        if sx.abs() <= 1e-4 || sy.abs() <= 1e-4 {
            return Ok(());
        }

        let rad = (-(transform.rotation_degrees as f32)).to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        let center_x_dst = dest.width as f32 / 2.0;
        let center_y_dst = dest.height as f32 / 2.0;
        let center_x_src = src.width as f32 / 2.0;
        let center_y_src = src.height as f32 / 2.0;

        let tx = transform.position_x as f32;
        let ty = transform.position_y as f32;

        for dy in 0..dest.height {
            for dx in 0..dest.width {
                // Map destination pixel back to untransformed source coordinate space
                let x_rel = dx as f32 - center_x_dst - tx;
                let y_rel = dy as f32 - center_y_dst - ty;

                let x_rot = x_rel.mul_add(cos_r, -y_rel * sin_r);
                let y_rot = x_rel.mul_add(sin_r, y_rel * cos_r);

                let src_x = (x_rot / sx) + center_x_src;
                let src_y = (y_rot / sy) + center_y_src;

                if src_x >= 0.0
                    && src_x < src.width as f32
                    && src_y >= 0.0
                    && src_y < src.height as f32
                {
                    let sampled_pixel = src.get_pixel(src_x as u32, src_y as u32);
                    let dst_px = dest.get_pixel(dx, dy);

                    let src_a = (sampled_pixel[3] as f32 / 255.0) * opacity;
                    let dst_a = dst_px[3] as f32 / 255.0;

                    let out_a = src_a + dst_a * (1.0 - src_a);
                    if out_a > 0.0 {
                        let out_r = ((sampled_pixel[0] as f32 * src_a)
                            + (dst_px[0] as f32 * dst_a * (1.0 - src_a)))
                            / out_a;
                        let out_g = ((sampled_pixel[1] as f32 * src_a)
                            + (dst_px[1] as f32 * dst_a * (1.0 - src_a)))
                            / out_a;
                        let out_b = ((sampled_pixel[2] as f32 * src_a)
                            + (dst_px[2] as f32 * dst_a * (1.0 - src_a)))
                            / out_a;

                        dest.set_pixel(
                            dx,
                            dy,
                            [
                                out_r.round() as u8,
                                out_g.round() as u8,
                                out_b.round() as u8,
                                (out_a * 255.0).round() as u8,
                            ],
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Composite an edit cut transition between two frame buffers into a destination buffer.
    pub fn composite_transition(
        &self,
        dest: &mut PixelBuffer,
        from: &PixelBuffer,
        to: &PixelBuffer,
        transition_type: TransitionType,
        progress: f32,
    ) -> Result<(), GpuError> {
        let p = progress.clamp(0.0, 1.0);
        let width = dest.width.min(from.width).min(to.width);
        let height = dest.height.min(from.height).min(to.height);

        for y in 0..height {
            for x in 0..width {
                let px_from = from.get_pixel(x, y);
                let px_to = to.get_pixel(x, y);

                let blended = match transition_type {
                    TransitionType::Crossfade | TransitionType::Dissolve => {
                        Self::lerp_pixels(px_from, px_to, p)
                    }
                    TransitionType::FadeToBlack => {
                        if p < 0.5 {
                            let fade = p.mul_add(-2.0, 1.0);
                            Self::scale_pixel(px_from, fade)
                        } else {
                            let fade = (p - 0.5) * 2.0;
                            Self::scale_pixel(px_to, fade)
                        }
                    }
                    TransitionType::DipToWhite => {
                        let white = [255, 255, 255, 255];
                        if p < 0.5 {
                            let dip = p * 2.0;
                            Self::lerp_pixels(px_from, white, dip)
                        } else {
                            let dip = (p - 0.5).mul_add(-2.0, 1.0);
                            Self::lerp_pixels(px_to, white, dip)
                        }
                    }
                    TransitionType::WipeLeft => {
                        let cut_x = (1.0 - p) * (width as f32);
                        if x as f32 >= cut_x { px_to } else { px_from }
                    }
                    TransitionType::WipeRight => {
                        let cut_x = p * (width as f32);
                        if x as f32 <= cut_x { px_to } else { px_from }
                    }
                };

                dest.set_pixel(x, y, blended);
            }
        }

        Ok(())
    }

    fn lerp_pixels(p0: [u8; 4], p1: [u8; 4], t: f32) -> [u8; 4] {
        let inv = 1.0 - t;
        [
            (p1[0] as f32).mul_add(t, p0[0] as f32 * inv).round() as u8,
            (p1[1] as f32).mul_add(t, p0[1] as f32 * inv).round() as u8,
            (p1[2] as f32).mul_add(t, p0[2] as f32 * inv).round() as u8,
            (p1[3] as f32).mul_add(t, p0[3] as f32 * inv).round() as u8,
        ]
    }

    fn scale_pixel(p: [u8; 4], scale: f32) -> [u8; 4] {
        [
            (p[0] as f32 * scale).round() as u8,
            (p[1] as f32 * scale).round() as u8,
            (p[2] as f32 * scale).round() as u8,
            p[3],
        ]
    }
}
