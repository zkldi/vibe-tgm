//! Tetris: The Grand Master 1–style rules (wiki-accurate tables, deterministic core).

mod autoplay;
mod board;
mod constants;
mod game;
mod grade;
mod gravity;
mod level;
mod options;
mod piece;
mod randomizer;
mod rotation;
mod score;

pub use autoplay::{
	AutoplayDriver, autoplay_best_placement_fallback, autoplay_plan_inputs, board_heuristic_static,
};
pub use board::{Board, Cell, EMPTY, clear_lines, count_full_lines, find_full_lines};
pub use constants::{
	ARE_FRAMES, BOARD_HEIGHT, BOARD_WIDTH, DAS_FRAMES, DAS_REPEAT_FRAMES, LINE_CLEAR_FRAMES,
	LOCK_DELAY_FRAMES, TLS_MAX_LEVEL, VISIBLE_ROWS,
};
pub use game::{Game, Input, Phase, PieceState, input_pack, input_unpack};
pub use grade::Grade;
pub use gravity::{effective_gravity, internal_gravity};
pub use level::{level_after_line_clear, level_after_piece_spawn, line_clear_only_for_increment};
pub use options::GameOptions;
pub use piece::{PieceKind, RotIndex, piece_cells, rotate_ccw, rotate_cw, spawn_origin};
pub use randomizer::TgmRandomizer;
pub use rotation::{try_rotate_ccw, try_rotate_cw};
pub use score::{add_score, bravo_factor};
