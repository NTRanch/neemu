//! Media file primitives: SHA-256 hashing, format sniffing from magic
//! bytes, and image dimension reads from headers. Everything here is
//! deterministic and file-metric only — neemu never decodes media beyond
//! headers in S1, and never returns source media through the protocol
//! (SPEC §5).

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Image formats the S1 image lane accepts, by extension and by content.
pub const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// Audio formats the S3 voice lane accepts, by extension and by content.
pub const AUDIO_EXTENSIONS: [&str; 4] = ["wav", "mp3", "flac", "ogg"];

/// Video formats the S3 voice lane accepts, by extension and by content.
pub const VIDEO_EXTENSIONS: [&str; 3] = ["mp4", "webm", "mkv"];

/// One media file in a work's manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaFile {
    /// File name within the work's media directory (no directories).
    pub name: String,
    /// Absolute path at registration time.
    pub path: PathBuf,
    /// Size in bytes when hashed.
    pub size: u64,
    /// Lowercase hex SHA-256 of the file's bytes.
    pub sha256: String,
    /// Sniffed format: `png` / `jpeg` / `webp` / `gif` / `unknown`.
    pub format: String,
    /// Pixel dimensions when the header carries them; `(0, 0)` when the
    /// format is recognised but the dimension fields are not readable.
    pub width: u32,
    pub height: u32,
}

/// SHA-256 of `bytes` as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Hash a file's bytes, streaming in fixed chunks so large pages stay
/// bounded in memory. Returns `(size_bytes, sha256_hex)`.
pub fn hash_file(path: &Path) -> std::io::Result<(u64, String)> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut size: u64 = 0;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok((size, out))
}

/// Sniff `(format, width, height)` from a file's leading bytes. Reads at
/// most the first 64 KiB (enough for PNG IHDR, GIF header, WebP headers,
/// and a JPEG marker scan for the first SOF segment).
pub fn sniff_image(path: &Path) -> std::io::Result<(String, u32, u32)> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut head = vec![0u8; 64 * 1024];
    let mut filled = 0usize;
    while filled < head.len() {
        let read = file.read(&mut head[filled..])?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    head.truncate(filled);
    Ok(sniff_bytes(&head))
}

/// Pure-bytes variant of [`sniff_image`], for tests and in-memory checks.
pub fn sniff_bytes(bytes: &[u8]) -> (String, u32, u32) {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        // PNG: IHDR width/height are big-endian u32 at offsets 16 and 20.
        if bytes.len() >= 24 {
            let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
            let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
            return ("png".to_string(), width, height);
        }
        return ("png".to_string(), 0, 0);
    }
    if bytes.starts_with(b"GIF8") {
        // GIF: logical screen width/height are little-endian u16 at 6 and 8.
        if bytes.len() >= 10 {
            let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
            let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
            return ("gif".to_string(), width, height);
        }
        return ("gif".to_string(), 0, 0);
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return (
            "webp".to_string(),
            webp_dimensions(bytes),
            webp_height(bytes),
        );
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        let (width, height) = jpeg_dimensions(bytes);
        return ("jpeg".to_string(), width, height);
    }
    ("unknown".to_string(), 0, 0)
}

fn webp_dimensions(bytes: &[u8]) -> u32 {
    // VP8X: canvas width-1 is a 24-bit little-endian value at offset 24.
    if bytes.len() >= 30 && &bytes[12..16] == b"VP8X" {
        return (bytes[24] as u32 | (bytes[25] as u32) << 8 | (bytes[26] as u32) << 16) + 1;
    }
    0
}

fn webp_height(bytes: &[u8]) -> u32 {
    if bytes.len() >= 30 && &bytes[12..16] == b"VP8X" {
        return (bytes[27] as u32 | (bytes[28] as u32) << 8 | (bytes[29] as u32) << 16) + 1;
    }
    0
}

/// Scan JPEG markers for the first SOF0/SOF2 segment, which carries the
/// frame height and width as big-endian u16s. Returns `(width, height)`,
/// `(0, 0)` when no SOF segment appears in the scanned bytes.
fn jpeg_dimensions(bytes: &[u8]) -> (u32, u32) {
    let mut i = 2usize; // skip SOI
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xC0 || marker == 0xC2 {
            if i + 9 <= bytes.len() {
                let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                return (width, height);
            }
            return (0, 0);
        }
        // Standalone markers carry no length field.
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if marker == 0xDA {
            break; // start of scan: no SOF beyond this point matters here
        }
        let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if length < 2 {
            break;
        }
        i += 2 + length;
    }
    (0, 0)
}

/// List the image files directly inside `dir` (non-recursive), sorted by
/// file name — the deterministic page order for a page-sequence work.
pub fn list_media_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_image = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false);
        if is_image {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// List every media file the registry accepts (images + S3 audio/video),
/// sorted by file name — the registration manifest order.
pub fn list_all_media_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let known = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let ext = ext.to_lowercase();
                IMAGE_EXTENSIONS.contains(&ext.as_str())
                    || AUDIO_EXTENSIONS.contains(&ext.as_str())
                    || VIDEO_EXTENSIONS.contains(&ext.as_str())
            })
            .unwrap_or(false);
        if known {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

// ---- S3 audio/video header facts (file-metric only; nothing decodes
// ---- beyond headers except the WAV PCM metrics below, and nothing
// ---- here ever plays audio — SPEC §7 S3 silence gate) ----

/// Header facts for one audio/video file. `duration_seconds` is present
/// only when the container header states it honestly (WAV data-chunk
/// arithmetic, FLAC STREAMINFO, MP4 mvhd); formats whose headers carry
/// no duration (MP3, OGG, EBML) report `None` rather than an estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct AvInfo {
    /// Sniffed format: `wav` / `mp3` / `flac` / `ogg` / `mp4` /
    /// `webm` / `mkv` / `unknown`.
    pub format: String,
    /// `audio` or `video`, from the extension class.
    pub media_kind: &'static str,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default()
}

/// Sniff an audio/video file from its leading bytes (at most 256 KiB —
/// enough for the WAV chunk walk, FLAC STREAMINFO, and the MP4 mvhd
/// search window).
pub fn sniff_av(path: &Path) -> std::io::Result<AvInfo> {
    use std::io::Read;
    let ext = extension_of(path);
    let mut file = fs::File::open(path)?;
    let mut head = vec![0u8; 256 * 1024];
    let mut filled = 0usize;
    while filled < head.len() {
        let read = file.read(&mut head[filled..])?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    head.truncate(filled);
    Ok(sniff_av_bytes(&head, &ext))
}

/// Pure-bytes variant of [`sniff_av`]; `ext` is the lowercase file
/// extension, used only to label EBML as webm vs mkv (the container
/// magic is shared).
pub fn sniff_av_bytes(bytes: &[u8], ext: &str) -> AvInfo {
    let media_kind: &'static str = if VIDEO_EXTENSIONS.contains(&ext) {
        "video"
    } else {
        "audio"
    };
    let unknown = AvInfo {
        format: "unknown".to_string(),
        media_kind,
        duration_seconds: None,
        sample_rate: None,
        channels: None,
    };
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        let (duration, rate, channels) = wav_header(bytes);
        return AvInfo {
            format: "wav".to_string(),
            media_kind,
            duration_seconds: duration,
            sample_rate: rate,
            channels,
        };
    }
    if bytes.starts_with(b"fLaC") {
        let (duration, rate, channels) = flac_header(bytes);
        return AvInfo {
            format: "flac".to_string(),
            media_kind,
            duration_seconds: duration,
            sample_rate: rate,
            channels,
        };
    }
    if bytes.starts_with(b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0)
    {
        return AvInfo {
            format: "mp3".to_string(),
            ..unknown
        };
    }
    if bytes.starts_with(b"OggS") {
        return AvInfo {
            format: "ogg".to_string(),
            ..unknown
        };
    }
    if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
        return AvInfo {
            format: "mp4".to_string(),
            media_kind,
            duration_seconds: mp4_duration(bytes),
            sample_rate: None,
            channels: None,
        };
    }
    if bytes.len() >= 4 && bytes[0..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return AvInfo {
            format: if ext == "webm" {
                "webm".to_string()
            } else {
                "mkv".to_string()
            },
            ..unknown
        };
    }
    unknown
}

/// WAV (RIFF) header walk: returns `(duration_seconds, sample_rate,
/// channels)` from the `fmt ` + `data` chunks; duration = data bytes /
/// byte rate. Any piece may be absent (then it is `None`).
fn wav_header(bytes: &[u8]) -> (Option<f64>, Option<u32>, Option<u32>) {
    let mut byte_rate: Option<u32> = None;
    let mut sample_rate: Option<u32> = None;
    let mut channels: Option<u32> = None;
    let mut data_bytes: Option<u64> = None;
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let body = i + 8;
        if id == b"fmt " && body + 16 <= bytes.len() {
            channels = Some(u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]) as u32);
            sample_rate = Some(u32::from_le_bytes([
                bytes[body + 4],
                bytes[body + 5],
                bytes[body + 6],
                bytes[body + 7],
            ]));
            byte_rate = Some(u32::from_le_bytes([
                bytes[body + 8],
                bytes[body + 9],
                bytes[body + 10],
                bytes[body + 11],
            ]));
        }
        if id == b"data" {
            data_bytes = Some(size as u64);
            break;
        }
        // Chunks are word-aligned; a truncated tail ends the walk.
        i = body + size + (size % 2);
    }
    let duration = match (data_bytes, byte_rate) {
        (Some(data), Some(rate)) if rate > 0 => Some(data as f64 / rate as f64),
        _ => None,
    };
    (duration, sample_rate, channels)
}

/// FLAC STREAMINFO walk: sample rate (20 bits), channels (3 bits,
/// stored minus one), total samples (36 bits) → duration.
fn flac_header(bytes: &[u8]) -> (Option<f64>, Option<u32>, Option<u32>) {
    // "fLaC" (4) + block header (4) → STREAMINFO data at offset 8.
    if bytes.len() < 8 + 18 {
        return (None, None, None);
    }
    let b = &bytes[8..];
    let sample_rate = ((b[10] as u32) << 12) | ((b[11] as u32) << 4) | ((b[12] as u32) >> 4);
    let channels = (((b[12] >> 1) & 0x7) as u32) + 1;
    let total_samples = (((b[13] & 0x0F) as u64) << 32)
        | ((b[14] as u64) << 24)
        | ((b[15] as u64) << 16)
        | ((b[16] as u64) << 8)
        | (b[17] as u64);
    let duration = if sample_rate > 0 && total_samples > 0 {
        Some(total_samples as f64 / sample_rate as f64)
    } else {
        None
    };
    let rate = if sample_rate > 0 {
        Some(sample_rate)
    } else {
        None
    };
    (duration, rate, Some(channels))
}

/// MP4 duration from the `mvhd` box (version 0 and 1), searched in the
/// sniffed head window.
fn mp4_duration(bytes: &[u8]) -> Option<f64> {
    let idx = bytes.windows(4).position(|w| w == b"mvhd")?;
    let version = *bytes.get(idx + 4)?;
    // Offsets are relative to the start of the "mvhd" type bytes:
    // version/flags (4) then creation/modification (4+4 in v0,
    // 8+8 in v1), then timescale, then duration.
    let (ts_off, dur_off, wide) = if version == 0 {
        (idx + 16, idx + 20, false)
    } else if version == 1 {
        (idx + 28, idx + 32, true)
    } else {
        return None;
    };
    let timescale = u32::from_be_bytes([
        *bytes.get(ts_off)?,
        *bytes.get(ts_off + 1)?,
        *bytes.get(ts_off + 2)?,
        *bytes.get(ts_off + 3)?,
    ]);
    if timescale == 0 {
        return None;
    }
    let duration = if wide {
        let mut buf = [0u8; 8];
        for (i, slot) in buf.iter_mut().enumerate() {
            *slot = *bytes.get(dur_off + i)?;
        }
        u64::from_be_bytes(buf) as f64
    } else {
        u32::from_be_bytes([
            *bytes.get(dur_off)?,
            *bytes.get(dur_off + 1)?,
            *bytes.get(dur_off + 2)?,
            *bytes.get(dur_off + 3)?,
        ]) as f64
    };
    Some(duration / timescale as f64)
}

/// File-metric measurements over one time range of a WAV file,
/// computed from the PCM samples themselves (SPEC §3.4: features
/// computed from the file). Deterministic; decode is limited to
/// PCM (format 1: 8/16-bit, format 3: 32-bit float) — any other WAV
/// encoding returns `None` (the caller records decode as
/// unavailable rather than estimating).
#[derive(Debug, Clone, PartialEq)]
pub struct WavSegmentMetrics {
    pub sample_rate: u32,
    pub channels: u32,
    /// Peak absolute sample, 0..=1 of full scale.
    pub peak: f64,
    /// Root-mean-square over the range, 0..=1 of full scale.
    pub rms: f64,
    /// Fraction of samples at ≥ 0.999 of full scale.
    pub clipped_fraction: f64,
    /// Count of pauses (sub-threshold runs ≥ 150 ms).
    pub pause_count: u32,
    /// Longest pause inside the range, seconds.
    pub longest_pause_s: f64,
    /// Non-pause seconds inside the range (the envelope's speech
    /// estimate; labelled as an estimate on the record).
    pub speech_seconds: f64,
    /// Breath candidates: short, quiet bursts bounded by pauses
    /// (envelope estimate, labelled as such on the record).
    pub breath_events: u32,
}

struct WavLayout {
    audio_format: u16,
    channels: u32,
    sample_rate: u32,
    bits_per_sample: u16,
    data_start: usize,
    data_len: usize,
}

fn wav_layout(bytes: &[u8]) -> Option<WavLayout> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut fmt: Option<(u16, u32, u32, u16)> = None;
    let mut data: Option<(usize, usize)> = None;
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let body = i + 8;
        if id == b"fmt " && body + 16 <= bytes.len() {
            fmt = Some((
                u16::from_le_bytes([bytes[body], bytes[body + 1]]),
                u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]) as u32,
                u32::from_le_bytes([
                    bytes[body + 4],
                    bytes[body + 5],
                    bytes[body + 6],
                    bytes[body + 7],
                ]),
                u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]),
            ));
        }
        if id == b"data" {
            data = Some((body, size.min(bytes.len().saturating_sub(body))));
            break;
        }
        i = body + size + (size % 2);
    }
    let (audio_format, channels, sample_rate, bits_per_sample) = fmt?;
    let (data_start, data_len) = data?;
    if channels == 0 || sample_rate == 0 {
        return None;
    }
    Some(WavLayout {
        audio_format,
        channels,
        sample_rate,
        bits_per_sample,
        data_start,
        data_len,
    })
}

/// Measure one `[start_s, end_s)` range of a WAV file's bytes.
/// Returns `None` when the file is not a supported PCM WAV or the
/// range is empty. Channels are mixed down by max-absolute value
/// per frame (the envelope a listener's loudness follows).
pub fn wav_segment_metrics(bytes: &[u8], start_s: f64, end_s: f64) -> Option<WavSegmentMetrics> {
    let layout = wav_layout(bytes)?;
    let bytes_per_sample = match (layout.audio_format, layout.bits_per_sample) {
        (1, 8) => 1usize,
        (1, 16) => 2usize,
        (3, 32) => 4usize,
        _ => return None,
    };
    let frame_bytes = bytes_per_sample * layout.channels as usize;
    let total_frames = layout.data_len / frame_bytes;
    let rate = layout.sample_rate as f64;
    let start_frame = (start_s * rate).floor() as usize;
    let end_frame = ((end_s * rate).ceil() as usize).min(total_frames);
    if start_frame >= end_frame {
        return None;
    }
    // 10 ms envelope windows.
    let window_frames = (layout.sample_rate / 100).max(1) as usize;
    let mut window_peak: Vec<f64> = Vec::new();
    let mut window_rms: Vec<f64> = Vec::new();
    let mut peak = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut sample_count = 0u64;
    let mut clipped = 0u64;
    let mut win_peak = 0.0f64;
    let mut win_sum_sq = 0.0f64;
    let mut win_count = 0usize;
    let flush_window = |win_peak: f64,
                        win_sum_sq: f64,
                        win_count: usize,
                        window_peak: &mut Vec<f64>,
                        window_rms: &mut Vec<f64>| {
        if win_count > 0 {
            window_peak.push(win_peak);
            window_rms.push((win_sum_sq / win_count as f64).sqrt());
        }
    };
    for frame in start_frame..end_frame {
        let base = layout.data_start + frame * frame_bytes;
        let mut frame_value = 0.0f64;
        for channel in 0..layout.channels as usize {
            let off = base + channel * bytes_per_sample;
            let value = match (layout.audio_format, layout.bits_per_sample) {
                (1, 8) => (bytes[off] as f64 - 128.0) / 128.0,
                (1, 16) => i16::from_le_bytes([bytes[off], bytes[off + 1]]) as f64 / 32768.0,
                (3, 32) => {
                    f32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
                        as f64
                }
                _ => return None,
            };
            if value.abs() > frame_value.abs() {
                frame_value = value;
            }
        }
        let magnitude = frame_value.abs();
        peak = peak.max(magnitude);
        sum_sq += frame_value * frame_value;
        sample_count += 1;
        if magnitude >= 0.999 {
            clipped += 1;
        }
        win_peak = win_peak.max(magnitude);
        win_sum_sq += frame_value * frame_value;
        win_count += 1;
        if win_count == window_frames {
            flush_window(
                win_peak,
                win_sum_sq,
                win_count,
                &mut window_peak,
                &mut window_rms,
            );
            win_peak = 0.0;
            win_sum_sq = 0.0;
            win_count = 0;
        }
    }
    flush_window(
        win_peak,
        win_sum_sq,
        win_count,
        &mut window_peak,
        &mut window_rms,
    );
    if sample_count == 0 {
        return None;
    }
    // Pause/breath envelope analysis. Threshold ≈ −44 dBFS RMS.
    const PAUSE_RMS: f64 = 0.006;
    const PAUSE_MIN_WINDOWS: usize = 15; // 150 ms
    let is_sound: Vec<bool> = window_rms.iter().map(|rms| *rms >= PAUSE_RMS).collect();
    let duration_s = (end_frame - start_frame) as f64 / rate;
    let mut pause_count = 0u32;
    let mut longest_pause = 0usize;
    let mut pause_windows = 0usize;
    let mut i = 0usize;
    while i < is_sound.len() {
        if is_sound[i] {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < is_sound.len() && !is_sound[j] {
            j += 1;
        }
        let run = j - i;
        if run >= PAUSE_MIN_WINDOWS {
            pause_count += 1;
            longest_pause = longest_pause.max(run);
            pause_windows += run;
        }
        i = j;
    }
    // Median peak of sound windows (the speech reference level).
    let mut sound_peaks: Vec<f64> = window_peak
        .iter()
        .zip(is_sound.iter())
        .filter(|(_, sound)| **sound)
        .map(|(peak, _)| *peak)
        .collect();
    sound_peaks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_peak = if sound_peaks.is_empty() {
        0.0
    } else {
        sound_peaks[sound_peaks.len() / 2]
    };
    // Breath candidates: sound bursts of 50–450 ms, quieter than
    // half the median speech peak, with a pause on at least one side.
    let mut breath_events = 0u32;
    let mut i = 0usize;
    while i < is_sound.len() {
        if !is_sound[i] {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut burst_peak = 0.0f64;
        while j < is_sound.len() && is_sound[j] {
            burst_peak = burst_peak.max(window_peak[j]);
            j += 1;
        }
        let run = j - i;
        let bounded_by_pause = (i >= PAUSE_MIN_WINDOWS
            && window_rms[..i]
                .iter()
                .rev()
                .take(PAUSE_MIN_WINDOWS)
                .all(|r| *r < PAUSE_RMS))
            || (j + PAUSE_MIN_WINDOWS <= is_sound.len()
                && window_rms[j..j + PAUSE_MIN_WINDOWS]
                    .iter()
                    .all(|r| *r < PAUSE_RMS));
        if (5..=45).contains(&run)
            && median_peak > 0.0
            && burst_peak < 0.5 * median_peak
            && bounded_by_pause
        {
            breath_events += 1;
        }
        i = j;
    }
    Some(WavSegmentMetrics {
        sample_rate: layout.sample_rate,
        channels: layout.channels,
        peak,
        rms: (sum_sq / sample_count as f64).sqrt(),
        clipped_fraction: clipped as f64 / sample_count as f64,
        pause_count,
        longest_pause_s: longest_pause as f64 * 0.01,
        speech_seconds: duration_s - pause_windows as f64 * 0.01,
        breath_events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_dimensions_from_ihdr() {
        let mut bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&[0, 0, 0, 13]);
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&800u32.to_be_bytes());
        bytes.extend_from_slice(&600u32.to_be_bytes());
        assert_eq!(sniff_bytes(&bytes), ("png".to_string(), 800, 600));
    }

    #[test]
    fn gif_dimensions_from_header() {
        let mut bytes = b"GIF89a".to_vec();
        bytes.extend_from_slice(&320u16.to_le_bytes());
        bytes.extend_from_slice(&240u16.to_le_bytes());
        assert_eq!(sniff_bytes(&bytes), ("gif".to_string(), 320, 240));
    }

    #[test]
    fn unknown_bytes_sniff_unknown() {
        assert_eq!(sniff_bytes(b"not an image"), ("unknown".to_string(), 0, 0));
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
