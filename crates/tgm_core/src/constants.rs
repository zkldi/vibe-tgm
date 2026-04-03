//! TGM1 timing constants (60 Hz frames). Source: Tetris Wiki — TGM.

/// ARE (entry delay) after lock / line clear collapse.
pub const ARE_FRAMES: u32 = 30;
/// DAS initial delay before auto-shift.
pub const DAS_FRAMES: u32 = 16;
/// Lock delay (frames a piece can rest on the stack before locking).
pub const LOCK_DELAY_FRAMES: u32 = 30;
/// Line clear animation delay.
pub const LINE_CLEAR_FRAMES: u32 = 41;

pub const BOARD_WIDTH: usize = 10;
/// Rows 0..=19 visible (bottom..top), row 20 = buffer above visible top.
pub const BOARD_HEIGHT: usize = 21;

/// Visible rows (excluding buffer).
pub const VISIBLE_ROWS: usize = 20;

/// TLS ghost piece shown for levels 0..=100 inclusive.
pub const TLS_MAX_LEVEL: u16 = 100;
