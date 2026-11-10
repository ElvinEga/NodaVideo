//! Hardware-accelerated and software fallback video compositing and transition shaders.

pub mod compositor;
pub mod error;
pub mod pipeline;
pub mod shaders;
pub mod types;

pub use compositor::{PixelBuffer, SoftwareCompositor};
pub use error::GpuError;
pub use pipeline::GpuContext;
pub use shaders::{TRANSFORM_WGSL, TRANSITION_WGSL};
pub use types::{
    QUAD_INDICES, QUAD_VERTICES, RgbaColor, TransformUniform, TransitionUniform, Vertex, Viewport,
};

#[cfg(test)]
mod tests {
    use nodavideo_timeline::{ClipTransform, TransitionType};

    use super::*;

    #[test]
    fn test_viewport_and_transform_uniforms() {
        let vp = Viewport::new(1920, 1080);
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
        assert!((vp.aspect_ratio() - 1.777_777_8).abs() < 1e-4);

        // Identity transform
        let identity = ClipTransform::default();
        let uniform = TransformUniform::from_clip_transform(&identity, vp);
        assert_eq!(uniform.opacity, 1.0);
        assert_eq!(uniform.matrix[0], 1.0); // scale_x
        assert_eq!(uniform.matrix[5], 1.0); // scale_y
        assert_eq!(uniform.matrix[12], 0.0); // tx
        assert_eq!(uniform.matrix[13], 0.0); // ty

        // Custom transform
        let transform = ClipTransform {
            position_x: 960.0, // Center right (+1.0 in NDC)
            position_y: 540.0, // Center bottom (-1.0 in NDC)
            scale_x: 1.5,
            scale_y: 2.0,
            rotation_degrees: 90.0,
            opacity: 0.8,
        };
        let uniform_custom = TransformUniform::from_clip_transform(&transform, vp);
        assert_eq!(uniform_custom.opacity, 0.8);
        assert!((uniform_custom.matrix[12] - 1.0).abs() < 1e-4);
        assert!((uniform_custom.matrix[13] - (-1.0)).abs() < 1e-4);
        // With 90 deg rotation, cos(90) ~= 0, sin(90) ~= 1
        assert!(uniform_custom.matrix[0].abs() < 1e-4);
        assert!((uniform_custom.matrix[1] - 1.5).abs() < 1e-4);
    }

    #[test]
    fn test_transition_uniform_mapping() {
        let vp = Viewport::new(1920, 1080);
        let u_crossfade = TransitionUniform::new(TransitionType::Crossfade, 0.5, vp);
        assert_eq!(u_crossfade.transition_type, 0);
        assert_eq!(u_crossfade.progress, 0.5);

        let u_fade_black = TransitionUniform::new(TransitionType::FadeToBlack, 0.75, vp);
        assert_eq!(u_fade_black.transition_type, 2);
        assert_eq!(u_fade_black.progress, 0.75);

        let u_wipe = TransitionUniform::new(TransitionType::WipeRight, 0.25, vp);
        assert_eq!(u_wipe.transition_type, 5);
        assert_eq!(u_wipe.progress, 0.25);
    }

    #[test]
    fn test_software_compositor_layer_blending() {
        let compositor = SoftwareCompositor;
        let mut dest = PixelBuffer::from_color(100, 100, RgbaColor::new(0.0, 0.0, 0.0, 1.0)); // Solid Black
        let src = PixelBuffer::from_color(50, 50, RgbaColor::new(1.0, 0.0, 0.0, 1.0)); // Solid Red

        // Composite Red square in the center at 50% opacity
        let transform = ClipTransform {
            opacity: 0.5,
            ..Default::default()
        };

        compositor
            .composite_layer(&mut dest, &src, &transform)
            .unwrap();

        // Check center pixel: should be 50% red, 50% black -> [128, 0, 0, 255]
        let center_px = dest.get_pixel(50, 50);
        assert!((center_px[0] as i32 - 128).abs() <= 2);
        assert_eq!(center_px[1], 0);
        assert_eq!(center_px[2], 0);
        assert_eq!(center_px[3], 255);

        // Check corner pixel (5, 5): should still be black
        let corner_px = dest.get_pixel(5, 5);
        assert_eq!(corner_px, [0, 0, 0, 255]);
    }

    #[test]
    fn test_software_compositor_transitions() {
        let compositor = SoftwareCompositor;
        let from = PixelBuffer::from_color(10, 10, RgbaColor::new(1.0, 0.0, 0.0, 1.0)); // Red
        let to = PixelBuffer::from_color(10, 10, RgbaColor::new(0.0, 0.0, 1.0, 1.0)); // Blue
        let mut dest = PixelBuffer::new(10, 10);

        // 1. Crossfade at 50% -> purple [128, 0, 128, 255]
        compositor
            .composite_transition(&mut dest, &from, &to, TransitionType::Crossfade, 0.5)
            .unwrap();
        let mid_px = dest.get_pixel(5, 5);
        assert!((mid_px[0] as i32 - 128).abs() <= 2);
        assert_eq!(mid_px[1], 0);
        assert!((mid_px[2] as i32 - 128).abs() <= 2);

        // 2. Fade to black at 50% -> completely black [0, 0, 0, 255]
        compositor
            .composite_transition(&mut dest, &from, &to, TransitionType::FadeToBlack, 0.5)
            .unwrap();
        let black_px = dest.get_pixel(5, 5);
        assert_eq!(black_px, [0, 0, 0, 255]);

        // 3. Dip to white at 50% -> completely white [255, 255, 255, 255]
        compositor
            .composite_transition(&mut dest, &from, &to, TransitionType::DipToWhite, 0.5)
            .unwrap();
        let white_px = dest.get_pixel(5, 5);
        assert_eq!(white_px, [255, 255, 255, 255]);

        // 4. Wipe Right at 50% -> left half is `to` (Blue), right half is `from` (Red)
        compositor
            .composite_transition(&mut dest, &from, &to, TransitionType::WipeRight, 0.5)
            .unwrap();
        let left_wipe = dest.get_pixel(2, 5);
        assert_eq!(left_wipe, [0, 0, 255, 255]); // Blue
        let right_wipe = dest.get_pixel(8, 5);
        assert_eq!(right_wipe, [255, 0, 0, 255]); // Red
    }

    #[tokio::test]
    async fn test_wgpu_context_pipeline_creation() {
        // Attempt GPU initialization if adapter is available; gracefully pass on headless/virtualized environments
        match GpuContext::new(wgpu::TextureFormat::Rgba8UnormSrgb).await {
            Ok(ctx) => {
                let vp = Viewport::new(640, 360);
                let target = ctx.create_render_target(vp);
                assert_eq!(target.width(), 640);
                assert_eq!(target.height(), 360);
            }
            Err(GpuError::AdapterNotFound) => {
                // Expected and acceptable in headless CI without a hardware GPU driver
            }
            Err(err) => panic!("Unexpected GPU initialization error: {err}"),
        }
    }
}
