//! Playfield: 10×21 — rows 0..=19 visible (bottom..top), row 20 buffer.

use crate::constants::{BOARD_HEIGHT, BOARD_WIDTH};
use crate::piece::{PieceDef, PieceKind, RotIndex, piece_cells};

pub type Cell = u8;
pub const EMPTY: Cell = 0;

#[derive(Clone, Debug, Default)]
pub struct Board {
	/// rows[0] = bottom row, rows[19] = top visible, rows[20] = buffer.
	pub rows: [[Cell; BOARD_WIDTH]; BOARD_HEIGHT],
}

impl Board {
	pub fn new() -> Self {
		Self {
			rows: [[EMPTY; BOARD_WIDTH]; BOARD_HEIGHT],
		}
	}

	pub fn get(&self, x: i32, y: i32) -> Option<Cell> {
		if x < 0 || y < 0 {
			return None;
		}
		let x = x as usize;
		let y = y as usize;
		if x >= BOARD_WIDTH || y >= BOARD_HEIGHT {
			return None;
		}
		Some(self.rows[y][x])
	}

	pub fn occupied(&self, x: i32, y: i32) -> bool {
		self.get(x, y).map(|c| c != EMPTY).unwrap_or(true)
	}

	/// Collision test for a piece at origin (px, py).
	pub fn collides(&self, px: i32, py: i32, kind: PieceKind, rot: RotIndex) -> bool {
		let def = piece_cells(kind, rot);
		collides_def(self, px, py, &def)
	}

	pub fn lock_piece(&mut self, px: i32, py: i32, kind: PieceKind, rot: RotIndex, color: Cell) {
		let def = piece_cells(kind, rot);
		for (dx, dy) in def.cells {
			let x = px + dx as i32;
			let y = py + dy as i32;
			if x >= 0 && y >= 0 && (x as usize) < BOARD_WIDTH && (y as usize) < BOARD_HEIGHT {
				self.rows[y as usize][x as usize] = color;
			}
		}
	}

	/// Sonic drop / hard bottom: move down (decrease py) until blocked.
	pub fn drop_to_bottom(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		loop {
			if self.collides(px, py - 1, kind, rot) {
				return py;
			}
			py -= 1;
		}
	}

	/// 20G: if overlapping stack, move up until valid.
	pub fn rise_to_valid(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		const MAX_PY: i32 = BOARD_HEIGHT as i32 + 4;
		while self.collides(px, py, kind, rot) && py < MAX_PY {
			py += 1;
		}
		py
	}

	/// Reverse gravity: move up until blocked by ceiling/stack.
	pub fn rise_to_top(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		loop {
			if self.collides(px, py + 1, kind, rot) {
				return py;
			}
			py += 1;
		}
	}

	/// Reverse 20G: if overlapping, move down until valid.
	pub fn sink_to_valid(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		const MIN_PY: i32 = -4;
		while self.collides(px, py, kind, rot) && py > MIN_PY {
			py -= 1;
		}
		py
	}
}

pub fn collides_def(board: &Board, px: i32, py: i32, def: &PieceDef) -> bool {
	for (dx, dy) in def.cells {
		let x = px + dx as i32;
		let y = py + dy as i32;
		if x < 0 || x >= BOARD_WIDTH as i32 {
			return true;
		}
		if y < 0 {
			return true;
		}
		if y >= BOARD_HEIGHT as i32 {
			return true;
		}
		if board.rows[y as usize][x as usize] != EMPTY {
			return true;
		}
	}
	false
}

/// Find full lines (visible rows 0..19). Returns row indices bottom-first order.
pub fn find_full_lines(board: &Board) -> [bool; BOARD_HEIGHT] {
	let mut full = [false; BOARD_HEIGHT];
	for y in 0..crate::constants::VISIBLE_ROWS {
		let row = &board.rows[y];
		if row.iter().all(|&c| c != EMPTY) {
			full[y] = true;
		}
	}
	full
}

pub fn count_full_lines(full: &[bool; BOARD_HEIGHT]) -> u32 {
	full.iter().filter(|&&f| f).count() as u32
}

/// Remove full lines and apply gravity; buffer-row blocks that were in cleared lines
/// follow TGM stack behavior (simplified: standard collapse).
pub fn clear_lines(board: &mut Board, full: &[bool; BOARD_HEIGHT]) {
	let mut write = 0usize;
	for y in 0..crate::constants::VISIBLE_ROWS {
		if full[y] {
			continue;
		}
		if write != y {
			board.rows[write] = board.rows[y];
		}
		write += 1;
	}
	for y in write..crate::constants::VISIBLE_ROWS {
		board.rows[y] = [EMPTY; BOARD_WIDTH];
	}
}
