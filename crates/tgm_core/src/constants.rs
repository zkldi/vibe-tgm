//! TGM1 timing constants (60 Hz frames).
//!
//! Player-observed values from [TetrisWiki — Tetris The Grand Master](https://tetris.wiki/Tetris_The_Grand_Master)
//! (“Speed timings”, levels 000–999, inclusive DAS counting).

/// ARE (entry delay) after lock / line clear collapse.
pub const ARE_FRAMES: u32 = 30;
/// DAS: frames direction must be held (inclusive) before auto-repeat begins.
pub const DAS_FRAMES: u32 = 16;
/// After DAS charges, move one column every this many frames (TGM1: every frame).
pub const DAS_REPEAT_FRAMES: u32 = 1;
/// Lock delay (frames a piece can rest on the stack before locking).
pub const LOCK_DELAY_FRAMES: u32 = 30;
/// Line clear animation delay.
pub const LINE_CLEAR_FRAMES: u32 = 41;

pub const BOARD_WIDTH: usize = 10;
/// Rows 0..=19 visible (bottom..top), row 20 = buffer above visible top.
pub const BOARD_HEIGHT: usize = 21;

/// Visible rows (excluding buffer).
pub const VISIBLE_ROWS: usize = 20;

/// Big mode: 5-wide logical field, 10 visible + 1 buffer row.
pub const BIG_BOARD_WIDTH: usize = 5;
pub const BIG_BOARD_HEIGHT: usize = 11;
pub const BIG_VISIBLE_ROWS: usize = 10;

/// TLS ghost piece shown for levels 0..=100 inclusive.
pub const TLS_MAX_LEVEL: u16 = 100;
