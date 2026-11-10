//! Export coordinator managing offline or background MP4 render jobs.

use nodavideo_media_engine::{ExportReport, ExportSettings, export_sequence};
use nodavideo_project::ProjectDocument;
use nodavideo_timeline::Sequence;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::EditorError;

/// Status of an active or recent export job.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportStatus {
    /// No export is currently in progress.
    Idle,
    /// An export job is running.
    Running {
        /// Target path being written.
        output_path: PathBuf,
        /// Current status or step message.
        message: String,
    },
    /// Export job completed successfully.
    Completed(ExportReport),
    /// Export failed with an error message.
    Failed {
        /// Error details.
        error: String,
    },
}

/// Coordinates export jobs and monitors progress.
#[derive(Debug, Clone)]
pub struct ExportCoordinator {
    /// Current job status.
    pub status: ExportStatus,
    /// Default or active export settings.
    pub settings: ExportSettings,
    /// Cancellation flag for active job.
    cancel_flag: Arc<AtomicBool>,
}

impl Default for ExportCoordinator {
    fn default() -> Self {
        Self {
            status: ExportStatus::Idle,
            settings: ExportSettings::default(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ExportCoordinator {
    /// Create a new export coordinator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether an export job is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self.status, ExportStatus::Running { .. })
    }

    /// Request cancellation of any running job.
    pub fn cancel(&mut self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        if self.is_running() {
            self.status = ExportStatus::Failed {
                error: "Export cancelled by user".into(),
            };
        }
    }

    /// Run an export job synchronously for the given sequence and project document.
    pub fn execute_export(
        &mut self,
        sequence: &Sequence,
        project: &ProjectDocument,
        settings: &ExportSettings,
    ) -> Result<ExportReport, EditorError> {
        self.cancel_flag.store(false, Ordering::SeqCst);
        self.settings = settings.clone();
        self.status = ExportStatus::Running {
            output_path: settings.output_path.clone(),
            message: "Encoding multitrack sequence to MP4...".into(),
        };

        match export_sequence(sequence, project, settings) {
            Ok(report) => {
                self.status = ExportStatus::Completed(report.clone());
                Ok(report)
            }
            Err(e) => {
                let err_msg = e.to_string();
                self.status = ExportStatus::Failed {
                    error: err_msg.clone(),
                };
                Err(EditorError::ExportFailed(err_msg))
            }
        }
    }
}
