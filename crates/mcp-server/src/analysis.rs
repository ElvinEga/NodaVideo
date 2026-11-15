//! Media-analysis requests produced by the gateway.
//!
//! The executor validates the asset id against its media-library snapshot and
//! emits one of these; the editor runs the real decode off the UI thread, since
//! the gateway crate owns no media engine. The result is provisional until the
//! editor finalizes it with the measured audio analysis.

use timeline::Asset;

/// A validated request to analyze one library asset's audio and metadata.
#[derive(Clone, Debug)]
pub struct MediaAnalysisRequest {
    /// The library asset to analyze; its path is absolute and its metadata is
    /// already probed, so the editor only needs to decode the audio.
    pub asset: Asset,
}
