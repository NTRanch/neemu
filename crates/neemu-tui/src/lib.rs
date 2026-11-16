//! neemu-tui — the media-extraction console for the neemu MCP server.
//!
//! Fleet TUI pattern (DESIGN D2): the UI programs against a narrow
//! [`facade::NeemuFacade`] trait; panels are pure widget constructors;
//! the same execute path serves the full-screen ratatui mode and the
//! non-TTY line mode. Screens: Overview, Workflows, Review, Media
//! (DESIGN D5 ladder rung 1), Log/Live, Fleet.
//!
//! Lane discipline: this crate lives in the NTRanch neemu repo and
//! handles NSFW-lane extraction records. Nothing here leaves the lane;
//! the one cross-MCP seam (the torori voice-cue handoff) is a declared
//! trigger candidate stamped `origin_lane = "ntranch"`,
//! `content_label = "nsfw"` from birth.
//!
//! Audio silence: voice cues are data rows. This crate has no audio
//! output path and never plays anything.

#![forbid(unsafe_code)]

pub mod app;
pub mod events;
pub mod facade;
pub mod line;
pub mod media;
pub mod panels;
pub mod screens;
pub mod triggers;

/// Maximum works listed from the registry scan.
pub const WORKS_MAX: usize = 256;
/// Maximum records held for the Review screen in one load.
pub const RECORDS_MAX: usize = 512;
/// Maximum entries in the bounded event log.
pub const LOG_ENTRIES_MAX: usize = 500;
/// Maximum characters in one rendered log line.
pub const LOG_LINE_CHARS_MAX: usize = 240;
/// Maximum characters in one Review record summary line.
pub const RECORD_LINE_CHARS_MAX: usize = 160;
/// Maximum characters of a staged argument preview.
pub const ARGS_PREVIEW_CHARS_MAX: usize = 600;
/// Maximum trigger audit lines retained in memory.
pub const AUDIT_LINES_MAX: usize = 200;
/// Maximum bytes accepted for one media file (mirrors the server cap).
pub const MEDIA_FILE_BYTES_MAX: u64 = 64 * 1024 * 1024;
/// Maximum decoded pixels accepted for one media frame.
pub const MEDIA_PIXELS_MAX: usize = 16 * 1024 * 1024;
/// Maximum half-block columns rendered for one frame.
pub const MEDIA_COLS_MAX: usize = 160;
/// Maximum half-block rows rendered for one frame.
pub const MEDIA_ROWS_MAX: usize = 80;

/// Truncate `text` to at most `max_chars` characters (char-boundary
/// safe), appending an ellipsis marker when truncation happened.
pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let kept: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{kept}…")
}
