//! Media screen support (DESIGN D5, ladder rung 1).
//!
//! Presentation is protocol-bound: kitty graphics passthrough where
//! the terminal advertises it, the half-block floor otherwise. Sixel
//! is a named later rung — detection reports it, rendering falls
//! back to the floor and says so. Viewing is read-only: no code path
//! here writes to the viewed file. Rendering is a display transform
//! over pinned integer sampling, so the half-block output for a fixed
//! fixture is byte-stable across runs (proven by test).
//!
//! Face frames are images only. Nothing in this module touches
//! audio (AUDIO SILENCE RULE).

use std::path::Path;

use crate::{MEDIA_COLS_MAX, MEDIA_FILE_BYTES_MAX, MEDIA_PIXELS_MAX, MEDIA_ROWS_MAX};

/// Which rung of the D5 ladder produced what is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Kitty graphics protocol: lossless PNG passthrough.
    Kitty,
    /// Sixel advertised but not encoded in rung 1; floor shown.
    SixelFallback,
    /// Half-block floor (upper-half glyph, truecolor fg/bg).
    HalfBlock,
}

impl Protocol {
    /// Operator-facing label naming protocol + source path.
    pub fn label(self, path: &Path) -> String {
        match self {
            Protocol::Kitty => format!("protocol=kitty path={}", path.display()),
            Protocol::SixelFallback => format!(
                "protocol=sixel-advertised, half-block floor shown (sixel encoder is a later rung) path={}",
                path.display()
            ),
            Protocol::HalfBlock => {
                format!("protocol=half-block path={}", path.display())
            }
        }
    }
}

/// Detect the presentation protocol from terminal environment
/// variables (`TERM`, `KITTY_WINDOW_ID`, `TERM_PROGRAM`).
pub fn detect_protocol(get_env: impl Fn(&str) -> Option<String>) -> Protocol {
    if get_env("KITTY_WINDOW_ID").is_some() {
        return Protocol::Kitty;
    }
    let term = get_env("TERM").unwrap_or_default();
    let program = get_env("TERM_PROGRAM").unwrap_or_default();
    if term.contains("kitty") || program.contains("kitty") || program == "WezTerm" {
        return Protocol::Kitty;
    }
    if term.contains("sixel") {
        return Protocol::SixelFallback;
    }
    Protocol::HalfBlock
}

/// Typed media failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    /// File could not be read.
    Io(String),
    /// File exceeds the byte bound.
    TooLarge(u64),
    /// Decoded image exceeds the pixel bound.
    TooManyPixels(usize),
    /// Bytes are not a decodable image.
    Decode(String),
    /// Requested render grid exceeds the cell bounds.
    GridTooLarge(usize, usize),
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaError::Io(m) => write!(f, "media io: {m}"),
            MediaError::TooLarge(n) => write!(f, "media file too large: {n} bytes"),
            MediaError::TooManyPixels(n) => write!(f, "media image too large: {n} pixels"),
            MediaError::Decode(m) => write!(f, "media decode failed: {m}"),
            MediaError::GridTooLarge(c, r) => write!(f, "media grid too large: {c}x{r} cells"),
        }
    }
}

impl std::error::Error for MediaError {}

/// A decoded RGB frame (row-major, 3 bytes per pixel).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// RGB bytes, `width * height * 3`.
    pub rgb: Vec<u8>,
}

impl Frame {
    /// Build a frame from parts, enforcing the pixel bound.
    pub fn new(width: usize, height: usize, rgb: Vec<u8>) -> Result<Frame, MediaError> {
        let pixels = width.saturating_mul(height);
        if pixels > MEDIA_PIXELS_MAX {
            return Err(MediaError::TooManyPixels(pixels));
        }
        if rgb.len() != pixels.saturating_mul(3) {
            return Err(MediaError::Decode(format!(
                "rgb buffer {} bytes for {width}x{height}",
                rgb.len()
            )));
        }
        Ok(Frame { width, height, rgb })
    }

    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let idx = (y * self.width + x) * 3;
        [self.rgb[idx], self.rgb[idx + 1], self.rgb[idx + 2]]
    }
}

/// Load and decode one image file under explicit bounds. Read-only.
pub fn load_frame(path: &Path) -> Result<Frame, MediaError> {
    load_frame_bounded(path, MEDIA_FILE_BYTES_MAX)
}

/// [`load_frame`] with an explicit byte bound (tests use a small one).
pub fn load_frame_bounded(path: &Path, bytes_max: u64) -> Result<Frame, MediaError> {
    let metadata =
        std::fs::metadata(path).map_err(|e| MediaError::Io(format!("{}: {e}", path.display())))?;
    if metadata.len() > bytes_max {
        return Err(MediaError::TooLarge(metadata.len()));
    }
    let bytes =
        std::fs::read(path).map_err(|e| MediaError::Io(format!("{}: {e}", path.display())))?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| MediaError::Decode(e.to_string()))?
        .to_rgb8();
    let (width, height) = (img.width() as usize, img.height() as usize);
    Frame::new(width, height, img.into_raw())
}

/// One half-block cell: foreground = top sub-pixel, background =
/// bottom sub-pixel, glyph is always `▀`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HalfCell {
    /// Top sub-pixel colour.
    pub fg: [u8; 3],
    /// Bottom sub-pixel colour.
    pub bg: [u8; 3],
}

/// Render a frame to a `cols x rows` half-block grid. Sampling is
/// pinned integer nearest-neighbour at cell centres — no floats, no
/// driver-dependent paths — so output is byte-stable for a fixture.
pub fn half_block_cells(
    frame: &Frame,
    cols: usize,
    rows: usize,
) -> Result<Vec<Vec<HalfCell>>, MediaError> {
    if cols > MEDIA_COLS_MAX || rows > MEDIA_ROWS_MAX || cols == 0 || rows == 0 {
        return Err(MediaError::GridTooLarge(cols, rows));
    }
    if frame.width == 0 || frame.height == 0 {
        return Err(MediaError::Decode("empty frame".to_string()));
    }
    let mut grid = Vec::with_capacity(rows);
    for row in 0..rows {
        let mut line = Vec::with_capacity(cols);
        for col in 0..cols {
            // Each cell covers two vertical sub-pixels.
            let sx = ((2 * col + 1) * frame.width) / (2 * cols);
            let sy_top = ((4 * row + 1) * frame.height) / (4 * rows);
            let sy_bot = ((4 * row + 3) * frame.height) / (4 * rows);
            let sx = sx.min(frame.width - 1);
            line.push(HalfCell {
                fg: frame.pixel(sx, sy_top.min(frame.height - 1)),
                bg: frame.pixel(sx, sy_bot.min(frame.height - 1)),
            });
        }
        grid.push(line);
    }
    Ok(grid)
}

/// Render the grid as an ANSI truecolor string (line-mode form).
/// Deterministic: same grid, same bytes, every run.
pub fn half_block_ansi(grid: &[Vec<HalfCell>]) -> String {
    let mut out = String::new();
    for line in grid {
        for cell in line {
            out.push_str(&format!(
                "\x1b[38;2;{};{};{};48;2;{};{};{}m▀",
                cell.fg[0], cell.fg[1], cell.fg[2], cell.bg[0], cell.bg[1], cell.bg[2]
            ));
        }
        out.push_str("\x1b[0m\n");
    }
    out
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 (for the kitty payload), no external crate.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
        out.push(BASE64[(n >> 18) as usize & 63] as char);
        out.push(BASE64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            BASE64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Build the kitty graphics escape for a PNG file's raw bytes
/// (passthrough: the file bytes are transmitted, never re-encoded
/// or rewritten). Chunked at 4096 base64 characters per the spec.
pub fn kitty_escape(png_bytes: &[u8]) -> String {
    let encoded = base64_encode(png_bytes);
    let mut out = String::new();
    let mut chunks = encoded.as_bytes().chunks(4096).peekable();
    let mut first = true;
    while let Some(chunk) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        let text = String::from_utf8_lossy(chunk);
        if first {
            out.push_str(&format!("\x1b_Ga=T,f=100,m={more};{text}\x1b\\"));
            first = false;
        } else {
            out.push_str(&format!("\x1b_Gm={more};{text}\x1b\\"));
        }
    }
    if first {
        out.push_str("\x1b_Ga=T,f=100,m=0;\x1b\\");
    }
    out
}

/// What the Media screen shows for one path: the protocol label plus
/// either the half-block grid (floor) or the kitty byte count.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaView {
    /// Protocol + path label (always shown; honesty about the rung).
    pub label: String,
    /// Half-block grid when the floor produced the picture.
    pub grid: Option<Vec<Vec<HalfCell>>>,
    /// Raw file bytes when kitty passthrough is the presentation
    /// (the terminal receives them via `kitty_escape` in the shell
    /// front end; panels show the label and byte count).
    pub kitty_bytes: Option<usize>,
}

/// Load one face frame for display under the detected protocol.
pub fn view_frame(path: &Path, cols: usize, rows: usize) -> Result<MediaView, MediaError> {
    view_frame_with(path, cols, rows, detect_protocol(|k| std::env::var(k).ok()))
}

/// [`view_frame`] with an explicit protocol (tests pin the floor).
pub fn view_frame_with(
    path: &Path,
    cols: usize,
    rows: usize,
    protocol: Protocol,
) -> Result<MediaView, MediaError> {
    let label = protocol.label(path);
    if protocol == Protocol::Kitty {
        let metadata = std::fs::metadata(path)
            .map_err(|e| MediaError::Io(format!("{}: {e}", path.display())))?;
        if metadata.len() > MEDIA_FILE_BYTES_MAX {
            return Err(MediaError::TooLarge(metadata.len()));
        }
        return Ok(MediaView {
            label,
            grid: None,
            kitty_bytes: Some(metadata.len() as usize),
        });
    }
    let frame = load_frame(path)?;
    let grid = half_block_cells(&frame, cols, rows)?;
    Ok(MediaView {
        label,
        grid: Some(grid),
        kitty_bytes: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed 8x8 fixture: deterministic pattern, no file needed.
    fn fixture() -> Frame {
        let mut rgb = Vec::new();
        for y in 0..8u32 {
            for x in 0..8u32 {
                rgb.push((x * 32) as u8);
                rgb.push((y * 32) as u8);
                rgb.push(((x + y) * 16) as u8);
            }
        }
        Frame::new(8, 8, rgb).expect("fixture within bounds")
    }

    #[test]
    fn half_block_render_is_byte_stable_across_runs() {
        let frame = fixture();
        let first = half_block_ansi(&half_block_cells(&frame, 8, 4).expect("render"));
        let second = half_block_ansi(&half_block_cells(&frame, 8, 4).expect("render"));
        assert_eq!(first, second);
        assert!(first.contains('▀'));
    }

    #[test]
    fn oversized_grid_is_refused() {
        let frame = fixture();
        assert!(matches!(
            half_block_cells(&frame, MEDIA_COLS_MAX + 1, 4),
            Err(MediaError::GridTooLarge(..))
        ));
    }

    #[test]
    fn oversized_pixel_count_is_refused() {
        let huge = Frame::new(MEDIA_PIXELS_MAX + 1, 1, Vec::new());
        assert!(matches!(huge, Err(MediaError::TooManyPixels(..))));
    }

    #[test]
    fn oversized_file_is_refused_by_bound() {
        let dir = std::env::temp_dir();
        let path = dir.join("neemu-tui-media-oversize.bin");
        std::fs::write(&path, vec![0u8; 2048]).expect("write fixture");
        let result = load_frame_bounded(&path, 1024);
        assert!(matches!(result, Err(MediaError::TooLarge(2048))));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn detection_prefers_kitty_then_floor() {
        assert_eq!(
            detect_protocol(|k| (k == "KITTY_WINDOW_ID").then(|| "1".to_string())),
            Protocol::Kitty
        );
        assert_eq!(detect_protocol(|_| None), Protocol::HalfBlock);
        assert_eq!(
            detect_protocol(|k| (k == "TERM").then(|| "xterm-sixel".to_string())),
            Protocol::SixelFallback
        );
    }

    #[test]
    fn base64_and_kitty_escape_shapes() {
        assert_eq!(base64_encode(b"hi"), "aGk=");
        let escape = kitty_escape(b"\x89PNG fake");
        assert!(escape.starts_with("\x1b_Ga=T,f=100"), "{escape:?}");
        assert!(escape.ends_with("\x1b\\"));
    }
}
