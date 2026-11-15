//! Explorer media-library write requests produced by the gateway.
//!
//! The executor validates a call and emits one of these; the editor applies it
//! through its own import pipeline and media-library persistence, since the
//! gateway only owns a snapshot of the timeline, not the live explorer state.

use std::path::PathBuf;
use ulid::Ulid;

/// A validated media-library change for the editor to apply.
#[derive(Clone, Debug)]
pub enum ExplorerChange {
    /// Reference these original files into the media library (no copy). Probing
    /// is asynchronous, so the agent observes the resulting assets via get_media_items.
    Import { paths: Vec<PathBuf> },
    /// Remove an asset from the library; the source file is kept. The editor
    /// rejects this when timeline clips still reference the asset.
    Remove { asset_id: Ulid },
    /// Rename an asset's display name in the library; the source file is untouched.
    Rename { asset_id: Ulid, name: String },
}
