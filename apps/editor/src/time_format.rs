pub(crate) fn format_time(seconds: f64, padded_minutes: bool) -> String {
    let total = seconds.max(0.0).round() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    let minutes_text = if hours > 0 || padded_minutes {
        format!("{minutes:02}")
    } else {
        format!("{minutes}")
    };
    if hours > 0 {
        format!("{hours}:{minutes_text}:{seconds:02}")
    } else {
        format!("{minutes_text}:{seconds:02}")
    }
}

pub(crate) fn format_time_precise(seconds: f64) -> String {
    let seconds_clamped = seconds.max(0.0);
    let total = seconds_clamped.floor() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = seconds_clamped % 60.0;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{secs:04.1}")
    } else {
        format!("{minutes:02}:{secs:04.1}")
    }
}

#[cfg(test)]
#[path = "tests/time_format.test.rs"]
mod tests;
