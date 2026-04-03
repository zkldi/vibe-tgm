//! Tetris: The Grand Master 1–style rules (wiki-accurate tables, deterministic core).

mod board;
mod constants;
mod game;
mod grade;
mod gravity;
mod level;
mod piece;
mod randomizer;
mod rotation;
mod score;

pub use board::{Board, Cell, EMPTY};
pub use constants::{
    ARE_FRAMES, BOARD_HEIGHT, BOARD_WIDTH, DAS_FRAMES, LINE_CLEAR_FRAMES, LOCK_DELAY_FRAMES,
    TLS_MAX_LEVEL, VISIBLE_ROWS,
};
pub use game::{Game, Input, Phase, PieceState};
pub use grade::Grade;
pub use gravity::internal_gravity;
pub use level::{level_after_line_clear, level_after_piece_spawn, line_clear_only_for_increment};
pub use piece::{piece_cells, spawn_origin, PieceKind, RotIndex};
pub use randomizer::TgmRandomizer;
pub use rotation::{try_rotate_ccw, try_rotate_cw};
pub use score::{add_score, bravo_factor};
