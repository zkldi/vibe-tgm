//! 5×11 playfield for TGM1 Big mode.

use crate::constants::{BIG_BOARD_HEIGHT, BIG_BOARD_WIDTH, BIG_VISIBLE_ROWS};
use crate::piece::{PieceKind, RotIndex};
use crate::piece_big::{PieceDefBig, piece_cells_big};

pub type Cell = u8;
pub const EMPTY: Cell = 0;

#[derive(Clone, Debug, Default)]
pub struct BoardBig {
	pub rows: [[Cell; BIG_BOARD_WIDTH]; BIG_BOARD_HEIGHT],
}

impl BoardBig {
	pub fn new() -> Self {
		Self {
			rows: [[EMPTY; BIG_BOARD_WIDTH]; BIG_BOARD_HEIGHT],
		}
	}

	pub fn get(&self, x: i32, y: i32) -> Option<Cell> {
		if x < 0 || y < 0 {
			return None;
		}
		let x = x as usize;
		let y = y as usize;
		if x >= BIG_BOARD_WIDTH || y >= BIG_BOARD_HEIGHT {
			return None;
		}
		Some(self.rows[y][x])
	}

	pub fn collides(&self, px: i32, py: i32, kind: PieceKind, rot: RotIndex) -> bool {
		let def = piece_cells_big(kind, rot);
		collides_def_big(self, px, py, &def)
	}

	pub fn lock_piece(&mut self, px: i32, py: i32, kind: PieceKind, rot: RotIndex, color: Cell) {
		let def = piece_cells_big(kind, rot);
		for (dx, dy) in def.cells {
			let x = px + dx as i32;
			let y = py + dy as i32;
			if x >= 0 && y >= 0 && (x as usize) < BIG_BOARD_WIDTH && (y as usize) < BIG_BOARD_HEIGHT
			{
				self.rows[y as usize][x as usize] = color;
			}
		}
	}

	pub fn drop_to_bottom(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		loop {
			if self.collides(px, py - 1, kind, rot) {
				return py;
			}
			py -= 1;
		}
	}

	pub fn rise_to_top(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		loop {
			if self.collides(px, py + 1, kind, rot) {
				return py;
			}
			py += 1;
		}
	}

	pub fn rise_to_valid(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		const MAX_PY: i32 = BIG_BOARD_HEIGHT as i32 + 4;
		while self.collides(px, py, kind, rot) && py < MAX_PY {
			py += 1;
		}
		py
	}

	pub fn sink_to_valid(&self, px: i32, mut py: i32, kind: PieceKind, rot: RotIndex) -> i32 {
		const MIN_PY: i32 = -4;
		while self.collides(px, py, kind, rot) && py > MIN_PY {
			py -= 1;
		}
		py
	}
}

pub fn collides_def_big(board: &BoardBig, px: i32, py: i32, def: &PieceDefBig) -> bool {
	for (dx, dy) in def.cells {
		let x = px + dx as i32;
		let y = py + dy as i32;
		if x < 0 || x >= BIG_BOARD_WIDTH as i32 {
			return true;
		}
		if y < 0 {
			return true;
		}
		if y >= BIG_BOARD_HEIGHT as i32 {
			return true;
		}
		if board.rows[y as usize][x as usize] != EMPTY {
			return true;
		}
	}
	false
}

pub fn find_full_lines(board: &BoardBig) -> [bool; BIG_BOARD_HEIGHT] {
	let mut full = [false; BIG_BOARD_HEIGHT];
	for y in 0..BIG_VISIBLE_ROWS {
		let row = &board.rows[y];
		if row.iter().all(|&c| c != EMPTY) {
			full[y] = true;
		}
	}
	full
}

pub fn count_full_lines(full: &[bool; BIG_BOARD_HEIGHT]) -> u32 {
	full.iter().filter(|&&f| f).count() as u32
}

pub fn clear_lines(board: &mut BoardBig, full: &[bool; BIG_BOARD_HEIGHT]) {
	let mut write = 0usize;
	for y in 0..BIG_VISIBLE_ROWS {
		if full[y] {
			continue;
		}
		if write != y {
			board.rows[write] = board.rows[y];
		}
		write += 1;
	}
	for y in write..BIG_VISIBLE_ROWS {
		board.rows[y] = [EMPTY; BIG_BOARD_WIDTH];
	}
}
