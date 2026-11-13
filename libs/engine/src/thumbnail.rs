//! One-shot video thumbnails: software-decode a representative frame, downscale,
//! and write a PNG. Synchronous; callers run it off the UI thread. The playback
//! decoder is lane-local and its GPU surfaces need CPU readback for encoding, so
//! thumbnails decode directly through FFmpeg instead.

use anyhow::{Context as _, Result, bail};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::{Rational, util::error::EAGAIN};
use image::RgbaImage;
use std::path::Path;

/// Writes a PNG of a frame about a tenth into `source`, at most `max_width`
/// pixels wide, rotated to display orientation.
pub fn video_thumbnail(source: &Path, destination: &Path, max_width: u32) -> Result<()> {
    ffmpeg::init().context("initializing FFmpeg")?;
    let mut input = ffmpeg::format::input(source).context("opening video for thumbnail")?;
    let (stream_index, time_base, mut decoder, rotation) = {
        let stream = input
            .streams()
            .best(ffmpeg::media::Type::Video)
            .context("video has no video stream")?;
        let time_base = stream.time_base();
        if time_base.numerator() <= 0 || time_base.denominator() <= 0 {
            bail!("invalid stream time base: {time_base:?}");
        }
        let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
            .context("copying video codec parameters")?;
        let mut opened = context.decoder();
        opened.set_packet_time_base(time_base);
        let decoder = opened.video().context("opening video decoder")?;
        (stream.index(), time_base, decoder, stream_rotation(&stream))
    };

    // A tenth in avoids black fade-in frames; without a duration, take the first frame.
    let origin = unsafe { (*input.as_ptr()).start_time };
    let origin = if origin == ffmpeg::ffi::AV_NOPTS_VALUE {
        0
    } else {
        origin
    };
    let duration = input.duration();
    let target = origin + duration.max(0) / 10;
    if duration > 0 {
        input.seek(target, ..target).context("seeking video")?;
        decoder.flush();
    }
    let frame = decode_frame_at(&mut input, &mut decoder, stream_index, time_base, target)?
        .context("video decoded no frames")?;

    let width = max_width.min(frame.width()).max(1);
    let height = (u64::from(frame.height()) * u64::from(width) / u64::from(frame.width()))
        .max(1)
        .min(u64::from(u32::MAX)) as u32;
    let mut scaler = ffmpeg::software::scaling::Context::get(
        frame.format(),
        frame.width(),
        frame.height(),
        ffmpeg::format::Pixel::RGBA,
        width,
        height,
        ffmpeg::software::scaling::Flags::BILINEAR,
    )
    .context("creating thumbnail scaler")?;
    let coefficients = match frame.color_space() {
        ffmpeg::util::color::Space::BT709 => ffmpeg::ffi::SWS_CS_ITU709,
        ffmpeg::util::color::Space::BT2020NCL => ffmpeg::ffi::SWS_CS_BT2020,
        // 未标注的色彩空间按分辨率推测，与播放路径一致。
        _ if frame.height() >= 720 => ffmpeg::ffi::SWS_CS_ITU709,
        _ => ffmpeg::ffi::SWS_CS_SMPTE170M,
    };
    let full_range = matches!(frame.color_range(), ffmpeg::util::color::Range::JPEG) as i32;
    // SAFETY: scaler owns the context; FFmpeg returns a static coefficient table.
    unsafe {
        let matrix = ffmpeg::ffi::sws_getCoefficients(coefficients);
        if ffmpeg::ffi::sws_setColorspaceDetails(
            scaler.as_mut_ptr(),
            matrix,
            full_range,
            matrix,
            0,
            0,
            1 << 16,
            1 << 16,
        ) < 0
        {
            bail!("configuring thumbnail color conversion");
        }
    }
    let mut rgba = ffmpeg::frame::Video::empty();
    scaler.run(&frame, &mut rgba).context("scaling thumbnail")?;
    let stride = rgba.stride(0);
    let row_bytes = width as usize * 4;
    let data = rgba.data(0);
    let mut bytes = Vec::with_capacity(row_bytes * height as usize);
    for row in 0..height as usize {
        bytes.extend_from_slice(&data[row * stride..row * stride + row_bytes]);
    }
    let image = RgbaImage::from_raw(width, height, bytes).context("thumbnail buffer size")?;
    let image = match (rotation / 90.0).round().rem_euclid(4.0) as u32 {
        1 => image::imageops::rotate90(&image),
        2 => image::imageops::rotate180(&image),
        3 => image::imageops::rotate270(&image),
        _ => image,
    };
    image
        .save_with_format(destination, image::ImageFormat::Png)
        .context("writing thumbnail PNG")?;
    Ok(())
}

// Private APIs only

/// First frame at or after `target` (microseconds, AV_TIME_BASE), or the last
/// decoded frame when the video ends earlier. None means the stream decoded nothing.
fn decode_frame_at(
    input: &mut ffmpeg::format::context::Input,
    decoder: &mut ffmpeg::decoder::Video,
    stream_index: usize,
    time_base: Rational,
    target: i64,
) -> Result<Option<ffmpeg::frame::Video>> {
    let mut selected = None;
    let mut frame = ffmpeg::frame::Video::empty();
    let mut packet = ffmpeg::Packet::empty();
    let mut draining = false;
    loop {
        if !draining {
            match packet.read(input) {
                Ok(()) => {
                    if packet.stream() == stream_index {
                        decoder
                            .send_packet(&packet)
                            .context("sending a video packet")?;
                    }
                    packet = ffmpeg::Packet::empty();
                }
                Err(ffmpeg::Error::Eof) => {
                    decoder.send_eof().context("draining the video decoder")?;
                    draining = true;
                }
                Err(error) => return Err(error).context("reading a video packet"),
            }
        }
        loop {
            match decoder.receive_frame(&mut frame) {
                Ok(()) => {
                    let reached_target = match frame.timestamp().or(frame.pts()) {
                        Some(pts) => pts_microseconds(pts, time_base)? >= target,
                        None => true, // 无 PTS：无法定位，取已解码的帧。
                    };
                    selected = Some(frame);
                    if reached_target {
                        return Ok(selected);
                    }
                    frame = ffmpeg::frame::Video::empty();
                }
                Err(ffmpeg::Error::Eof) => return Ok(selected),
                Err(ffmpeg::Error::Other { errno: EAGAIN }) => break,
                Err(error) => return Err(error).context("receiving a decoded video frame"),
            }
        }
    }
}

fn pts_microseconds(pts: i64, time_base: Rational) -> Result<i64> {
    let scaled = i128::from(pts) * i128::from(time_base.numerator()) * 1_000_000
        / i128::from(time_base.denominator());
    i64::try_from(scaled).context("timestamp exceeds microsecond range")
}

/// Clockwise display rotation in degrees, from the rotate tag or display matrix.
fn stream_rotation(stream: &ffmpeg::Stream) -> f64 {
    let mut rotation = 0.0;
    if let Some(value) = stream.metadata().get("rotate") {
        rotation = value.parse().unwrap_or(0.0);
    }
    for side_data in stream.side_data() {
        if side_data.kind() == ffmpeg::codec::packet::side_data::Type::DisplayMatrix
            && side_data.data().len() >= 36
        {
            let mut matrix = [0_i32; 9];
            let (chunks, _) = side_data.data()[..36].as_chunks::<4>();
            for (i, chunk) in chunks.iter().enumerate() {
                matrix[i] = i32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
            rotation = -unsafe { ffmpeg::ffi::av_display_rotation_get(matrix.as_ptr()) };
        }
    }
    rotation
}