//! Tetris: The Grand Master 1–style rules (wiki-accurate tables, deterministic core).

mod autoplay;
mod board;
mod board_big;
mod constants;
mod game;
mod grade;
mod gravity;
mod level;
mod options;
mod piece;
mod piece_big;
mod randomizer;
mod rotation;
mod rotation_big;
mod score;

pub use board::{Board, Cell, EMPTY, clear_lines, count_full_lines, find_full_lines};
pub use board_big::{BoardBig, find_full_lines as find_full_lines_big};
pub use constants::{
	ARE_FRAMES, BIG_BOARD_HEIGHT, BIG_BOARD_WIDTH, BIG_VISIBLE_ROWS, BOARD_HEIGHT, BOARD_WIDTH,
	DAS_FRAMES, DAS_REPEAT_FRAMES, LINE_CLEAR_FRAMES, LOCK_DELAY_FRAMES, TLS_MAX_LEVEL,
	VISIBLE_ROWS,
};
pub use autoplay::autoplay_plan_inputs;
pub use game::{Game, Input, Phase, PieceState, input_pack, input_unpack};
pub use grade::Grade;
pub use gravity::{effective_gravity, internal_gravity};
pub use level::{level_after_line_clear, level_after_piece_spawn, line_clear_only_for_increment};
pub use options::GameOptions;
pub use piece::{PieceKind, RotIndex, piece_cells, rotate_ccw, rotate_cw, spawn_origin};
pub use piece_big::{PieceDefBig, piece_cells_big};
pub use randomizer::TgmRandomizer;
pub use rotation::{try_rotate_ccw, try_rotate_cw};
pub use score::{add_score, bravo_factor};
