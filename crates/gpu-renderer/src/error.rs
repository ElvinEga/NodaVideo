//! Error types for the GPU rendering and compositing engine.

use thiserror::Error;

/// Domain errors occurring within the GPU rendering pipeline.
#[derive(Debug, Error)]
pub enum GpuError {
    /// Failed to find a suitable GPU adapter.
    #[error("No suitable GPU adapter found")]
    AdapterNotFound,

    /// Failed to request a logical device and command queue from the GPU adapter.
    #[error("Failed to request GPU device: {0}")]
    DeviceRequestFailed(String),

    /// Shader compilation or validation failed.
    #[error("Shader compilation error: {0}")]
    ShaderError(String),

    /// Pipeline creation or configuration failed.
    #[error("Render pipeline creation failed: {0}")]
    PipelineCreationFailed(String),

    /// Dimension or viewport bounds error.
    #[error("Invalid dimensions: {width}x{height}")]
    InvalidDimensions {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },

    /// Software compositing or buffer allocation error.
    #[error("Compositor error: {0}")]
    CompositorFailed(String),
}
