//! Offline audio analysis for the AI gateway: decode a library asset's audio with
//! the waveform engine and derive loudness and silence regions. The gateway only
//! validates the asset and emits a request; this runs on the background executor,
//! so decoding never blocks the UI thread.

use nodavideo_mcp_server::McpError;
use serde_json::{Value, json};
use std::path::Path;
use timeline::Asset;

const SILENCE_THRESHOLD_DBFS: f64 = -40.0; // 低于此电平视为静音（约 0.01 线性振幅）。
const MIN_SILENCE_SECONDS: f64 = 0.1; // 过滤过短的静音段，避免噪声抖动产生大量碎片。
const COLUMNS_PER_SECOND: f64 = 100.0; // 约 10ms 分辨率；长文件由 waveform 内部上限收敛。
const DBFS_FLOOR: f64 = -120.0; // 数字静音的下限，避免 log10(0) 产生负无穷。

/// Finalize the gateway's provisional analyze_media result with measured audio.
/// Assets without audio keep `audio: null`; a decode failure becomes a real error
/// so the agent never sees a fabricated success.
pub(crate) fn analyze_media_asset(asset: &Asset, mut summary: Value) -> Result<Value, McpError> {
    let audio = if asset.has_audio() {
        Some(audio_analysis_json(
            asset.path(),
            asset.duration().unwrap_or(0.0),
        )?)
    } else {
        None
    };
    if let Some(object) = summary.as_object_mut() {
        object.insert("status".into(), json!("analyzed"));
        object.insert("audio".into(), audio.unwrap_or(Value::Null));
    }
    Ok(summary)
}

fn audio_analysis_json(path: &Path, duration: f64) -> Result<Value, McpError> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err(McpError::ExecutionFailed(format!(
            "cannot analyze audio with duration {duration}"
        )));
    }
    let waveform = ::waveform::generate_waveform(path)
        .map_err(|error| McpError::ExecutionFailed(format!("{error:#}")))?;
    let column_count = ((duration * COLUMNS_PER_SECOND).ceil() as usize).max(1);
    let columns = waveform.columns(0.0, duration, column_count);
    if columns.is_empty() {
        return Err(McpError::ExecutionFailed(format!(
            "could not decode audio from {}",
            path.display()
        )));
    }
    let column_seconds = duration / columns.len() as f64;
    let silence_threshold = 10.0_f64.powf(SILENCE_THRESHOLD_DBFS / 20.0);

    let mut peak_amplitude = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut silence_regions: Vec<(f64, f64)> = Vec::new();
    let mut silence_start: Option<usize> = None; // 当前静音段的起始列；None 表示不在静音中。
    for (index, column) in columns.iter().enumerate() {
        let amplitude = f64::from(column.min.abs().max(column.max.abs()));
        peak_amplitude = peak_amplitude.max(amplitude);
        sum_squares += amplitude * amplitude;
        if amplitude < silence_threshold {
            silence_start.get_or_insert(index);
        } else if let Some(start) = silence_start.take() {
            silence_regions.extend(silence_region(start, index, column_seconds));
        }
    }
    if let Some(start) = silence_start {
        silence_regions.extend(silence_region(start, columns.len(), column_seconds));
    }

    // RMS 由各列峰值包络估算（waveform 只存 min/max，不存原始样本），故略高于真实 RMS。
    let rms_amplitude = (sum_squares / columns.len() as f64).sqrt();
    let total_silence_seconds = silence_regions
        .iter()
        .map(|(start, end)| end - start)
        .sum::<f64>();
    let silence_json = silence_regions
        .iter()
        .map(|(start, end)| json!({ "start_seconds": start, "end_seconds": end }))
        .collect::<Vec<_>>();
    Ok(json!({
        "peak_dbfs": amplitude_to_dbfs(peak_amplitude),
        "rms_dbfs": amplitude_to_dbfs(rms_amplitude),
        "silence_threshold_dbfs": SILENCE_THRESHOLD_DBFS,
        "min_silence_seconds": MIN_SILENCE_SECONDS,
        "column_seconds": column_seconds,
        "total_silence_seconds": total_silence_seconds,
        "silence_regions": silence_json,
    }))
}

fn silence_region(
    start_column: usize,
    end_column: usize,
    column_seconds: f64,
) -> Option<(f64, f64)> {
    let start_seconds = start_column as f64 * column_seconds;
    let end_seconds = end_column as f64 * column_seconds;
    if end_seconds - start_seconds < MIN_SILENCE_SECONDS {
        return None;
    }
    Some((start_seconds, end_seconds))
}

fn amplitude_to_dbfs(amplitude: f64) -> f64 {
    if amplitude <= 0.0 {
        return DBFS_FLOOR;
    }
    (20.0 * amplitude.log10()).max(DBFS_FLOOR)
}
