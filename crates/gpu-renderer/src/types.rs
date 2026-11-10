//! Data structures, vertex layouts, uniform buffers, and coordinate transforms.

use bytemuck::{Pod, Zeroable};
use nodavideo_timeline::{ClipTransform, TransitionType};
use serde::{Deserialize, Serialize};

/// Target render viewport resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Viewport {
    /// Horizontal resolution in pixels.
    pub width: u32,
    /// Vertical resolution in pixels.
    pub height: u32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

impl Viewport {
    /// Create a new viewport with positive dimensions.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Aspect ratio as width / height.
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }
}

/// Normalized RGBA color value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RgbaColor {
    /// Red component `[0.0, 1.0]`.
    pub r: f32,
    /// Green component `[0.0, 1.0]`.
    pub g: f32,
    /// Blue component `[0.0, 1.0]`.
    pub b: f32,
    /// Alpha opacity `[0.0, 1.0]`.
    pub a: f32,
}

impl RgbaColor {
    /// Black color.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// White color.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Transparent color.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Create an RGBA color.
    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// Standard 2D quad vertex with texture coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    /// 2D position in normalized device coordinates `[-1.0, 1.0]`.
    pub position: [f32; 2],
    /// Texture coordinates `[0.0, 1.0]`.
    pub tex_coords: [f32; 2],
}

impl Vertex {
    /// Descriptor for vertex buffer layout in wgpu pipelines.
    #[must_use]
    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Default quad vertices spanning the viewport `[-1.0, 1.0]`.
pub const QUAD_VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
];

/// Two-triangle quad indices.
pub const QUAD_INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

/// GPU uniform buffer representation for clip 2D transforms and opacity.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TransformUniform {
    /// 4x4 column-major transformation matrix.
    pub matrix: [f32; 16],
    /// Layer opacity factor `[0.0, 1.0]`.
    pub opacity: f32,
    pub pad0: f32,
    pub pad1: f32,
    pub pad2: f32,
}

impl Default for TransformUniform {
    fn default() -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0, 0.0, // col 0
                0.0, 1.0, 0.0, 0.0, // col 1
                0.0, 0.0, 1.0, 0.0, // col 2
                0.0, 0.0, 0.0, 1.0, // col 3
            ],
            opacity: 1.0,
            pad0: 0.0,
            pad1: 0.0,
            pad2: 0.0,
        }
    }
}

impl TransformUniform {
    /// Construct a uniform matrix from a high-level `ClipTransform` and viewport dimensions.
    #[must_use]
    pub fn from_clip_transform(transform: &ClipTransform, viewport: Viewport) -> Self {
        let (vp_w, vp_h) = (viewport.width as f32, viewport.height as f32);

        // Convert pixel translation to normalized device coordinates: [-1.0, 1.0]
        let tx = if vp_w > 0.0 {
            (transform.position_x as f32 / vp_w) * 2.0
        } else {
            0.0
        };
        let ty = if vp_h > 0.0 {
            -(transform.position_y as f32 / vp_h) * 2.0
        } else {
            0.0
        };

        let sx = transform.scale_x as f32;
        let sy = transform.scale_y as f32;
        let rad = (transform.rotation_degrees as f32).to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        // 2D affine transform embedded in 4x4:
        // [ sx*cos  -sy*sin   0   tx ]
        // [ sx*sin   sy*cos   0   ty ]
        // [   0        0      1   0  ]
        // [   0        0      0   1  ]
        // Stored column-major:
        let matrix = [
            sx * cos_r,
            sx * sin_r,
            0.0,
            0.0, // col 0
            -sy * sin_r,
            sy * cos_r,
            0.0,
            0.0, // col 1
            0.0,
            0.0,
            1.0,
            0.0, // col 2
            tx,
            ty,
            0.0,
            1.0, // col 3
        ];

        Self {
            matrix,
            opacity: transform.opacity as f32,
            pad0: 0.0,
            pad1: 0.0,
            pad2: 0.0,
        }
    }
}

/// GPU uniform buffer representation for edit cut transitions.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TransitionUniform {
    /// Normalized transition progress `[0.0, 1.0]`.
    pub progress: f32,
    /// Integer code for transition type.
    pub transition_type: u32,
    /// Target viewport aspect ratio.
    pub aspect_ratio: f32,
    pub pad: f32,
}

impl TransitionUniform {
    /// Create a transition uniform from `TransitionType`, progress, and viewport.
    #[must_use]
    pub fn new(transition_type: TransitionType, progress: f32, viewport: Viewport) -> Self {
        let type_code = match transition_type {
            TransitionType::Crossfade => 0,
            TransitionType::Dissolve => 1,
            TransitionType::FadeToBlack => 2,
            TransitionType::DipToWhite => 3,
            TransitionType::WipeLeft => 4,
            TransitionType::WipeRight => 5,
        };

        Self {
            progress: progress.clamp(0.0, 1.0),
            transition_type: type_code,
            aspect_ratio: viewport.aspect_ratio(),
            pad: 0.0,
        }
    }
}
