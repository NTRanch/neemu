//! Rung-1 integration tests: headless smoke (first screen renders
//! under TestBackend, line mode answers from a pipe) and the media
//! determinism gate (half-block render of a fixed fixture is
//! byte-stable across two runs).

use std::io::Cursor;

use neemu_tui::app::App;
use neemu_tui::facade::FakeFacade;
use neemu_tui::line::{fleet_panel_text, overview_text, run_line_mode};
use neemu_tui::media::{half_block_ansi, half_block_cells, load_frame, view_frame_with, Protocol};

#[test]
fn first_screen_renders_under_test_backend() {
    let app = App::new(Box::new(FakeFacade::seeded()));
    let text = overview_text(&app);
    assert!(text.contains("neemu"), "overview missing member name");
    assert!(
        text.contains("extraction queue"),
        "overview missing queue state"
    );
}

#[test]
fn fleet_panel_lists_the_torori_rule_with_lane_stamps() {
    let app = App::new(Box::new(FakeFacade::seeded()));
    let text = fleet_panel_text(&app);
    assert!(text.contains("torori_bundle_ingest"), "{text}");
    assert!(text.contains("ntranch"), "{text}");
    assert!(text.contains("nsfw"), "{text}");
}

#[test]
fn line_mode_answers_one_command_from_a_pipe() {
    let mut app = App::new(Box::new(FakeFacade::seeded()));
    let mut input = Cursor::new(b"status\nquit\n".to_vec());
    let mut output: Vec<u8> = Vec::new();
    run_line_mode(&mut app, &mut input, &mut output).expect("line mode runs");
    let text = String::from_utf8(output).expect("utf8 output");
    assert!(text.contains("neemu"), "{text}");
}

/// Write the fixed PNG fixture (16x16 pattern) and return its path.
fn fixture_png() -> std::path::PathBuf {
    let path = std::env::temp_dir().join("neemu-tui-fixture-16x16.png");
    let mut img = image::RgbImage::new(16, 16);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgb([(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8]);
    }
    img.save(&path).expect("write fixture png");
    path
}

#[test]
fn media_half_block_of_fixed_fixture_is_byte_stable() {
    let path = fixture_png();
    let first = view_frame_with(&path, 16, 8, Protocol::HalfBlock).expect("first view");
    let second = view_frame_with(&path, 16, 8, Protocol::HalfBlock).expect("second view");
    assert_eq!(first.label, second.label);
    assert!(
        first.label.contains("protocol=half-block"),
        "{}",
        first.label
    );
    assert!(first.label.contains(&path.display().to_string()));
    let grid_a = first.grid.expect("floor grid");
    let grid_b = second.grid.expect("floor grid");
    assert_eq!(half_block_ansi(&grid_a), half_block_ansi(&grid_b));
    // And the underlying loader + renderer agree across calls.
    let frame = load_frame(&path).expect("load fixture");
    let again = half_block_cells(&frame, 16, 8).expect("render again");
    assert_eq!(half_block_ansi(&again), half_block_ansi(&grid_a));
}

#[test]
fn viewing_never_rewrites_the_file() {
    let path = fixture_png();
    let before = std::fs::read(&path).expect("read before");
    let _ = view_frame_with(&path, 16, 8, Protocol::HalfBlock).expect("view");
    let after = std::fs::read(&path).expect("read after");
    assert_eq!(before, after);
}
