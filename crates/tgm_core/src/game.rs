use crate::board::{Board, EMPTY, clear_lines, count_full_lines, find_full_lines};
use crate::constants::{BOARD_HEIGHT, DAS_FRAMES, DAS_REPEAT_FRAMES, LOCK_DELAY_FRAMES};
use crate::grade::Grade;
use crate::gravity::effective_gravity;
use crate::level::level_after_piece_spawn;
use crate::options::GameOptions;
use crate::piece::{PieceKind, RotIndex, rotate_ccw, rotate_cw, spawn_origin};
use crate::randomizer::TgmRandomizer;
use crate::rotation::{try_rotate_ccw, try_rotate_cw};
use crate::score::{add_score, bravo_factor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
	Falling,
	LineClear,
	Are,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
	pub left: bool,
	pub right: bool,
	pub down: bool,
	pub sonic: bool,
	pub rot_cw: bool,
	pub rot_ccw: bool,
}

/// Pack [`Input`] for replay storage: bits 0..6 are `left, right, down, sonic, rot_cw, rot_ccw`;
/// bits 6–7 zero.
pub fn input_pack(i: Input) -> u8 {
	let mut b = 0u8;
	if i.left {
		b |= 1;
	}
	if i.right {
		b |= 1 << 1;
	}
	if i.down {
		b |= 1 << 2;
	}
	if i.sonic {
		b |= 1 << 3;
	}
	if i.rot_cw {
		b |= 1 << 4;
	}
	if i.rot_ccw {
		b |= 1 << 5;
	}
	b
}

/// Inverse of [`input_pack`]. Fails if reserved bits are set.
pub fn input_unpack(b: u8) -> Option<Input> {
	if b & 0xC0 != 0 {
		return None;
	}
	Some(Input {
		left: b & 1 != 0,
		right: b & (1 << 1) != 0,
		down: b & (1 << 2) != 0,
		sonic: b & (1 << 3) != 0,
		rot_cw: b & (1 << 4) != 0,
		rot_ccw: b & (1 << 5) != 0,
	})
}

#[derive(Clone, Copy, Debug)]
pub struct PieceState {
	pub kind: PieceKind,
	pub rot: RotIndex,
	pub x: i32,
	pub y: i32,
}

#[derive(Clone)]
pub struct Game {
	pub options: GameOptions,
	pub board: Board,
	pub phase: Phase,
	pub piece: Option<PieceState>,
	pub next_kind: PieceKind,
	pub level: u16,
	pub score: u64,
	pub combo: u32,
	pub rng: TgmRandomizer,
	pub frame: u64,
	pub line_clear_timer: u32,
	pub are_timer: u32,
	pub lock_delay: u32,
	pub gravity_accum: u32,
	pub das_left: u32,
	pub das_right: u32,
	pub soft_frames_this_piece: u32,
	pub game_over: bool,
	pub cleared: bool,
	pub frame_at_level_300: Option<u64>,
	pub frame_at_level_500: Option<u64>,
	pub frame_at_level_999: Option<u64>,
	pub score_at_level_300: Option<u64>,
	pub score_at_level_500: Option<u64>,
	pub gm_qualified: bool,
	pending_lines: Option<[bool; BOARD_HEIGHT]>,
}

impl Game {
	pub fn new(seed: u64) -> Self {
		Self::with_options(seed, GameOptions::default())
	}

	pub fn with_options(seed: u64, options: GameOptions) -> Self {
		let mut rng = TgmRandomizer::new(seed);
		let next = rng.next_piece();
		let mut g = Self {
			options,
			board: Board::new(),
			phase: Phase::Are,
			piece: None,
			next_kind: next,
			level: 0,
			score: 0,
			combo: 1,
			rng,
			frame: 0,
			line_clear_timer: 0,
			are_timer: options.are_frames(),
			lock_delay: LOCK_DELAY_FRAMES,
			gravity_accum: 0,
			das_left: 0,
			das_right: 0,
			soft_frames_this_piece: 0,
			game_over: false,
			cleared: false,
			frame_at_level_300: None,
			frame_at_level_500: None,
			frame_at_level_999: None,
			score_at_level_300: None,
			score_at_level_500: None,
			gm_qualified: false,
			pending_lines: None,
		};
		g.record_level_milestone();
		g
	}

	pub fn eligible_for_hiscore(&self) -> bool {
		true
	}

	pub fn grade(&self) -> Grade {
		Grade::from_score(self.score)
	}

	pub fn grade_label(&self) -> &'static str {
		if self.cleared && self.gm_qualified {
			return "GM";
		}
		self.grade().display()
	}

	fn record_level_milestone(&mut self) {
		if self.level >= 300 && self.frame_at_level_300.is_none() {
			self.frame_at_level_300 = Some(self.frame);
			self.score_at_level_300 = Some(self.score);
		}
		if self.level >= 500 && self.frame_at_level_500.is_none() {
			self.frame_at_level_500 = Some(self.frame);
			self.score_at_level_500 = Some(self.score);
		}
		if self.level >= 999 && self.frame_at_level_999.is_none() {
			self.frame_at_level_999 = Some(self.frame);
		}
		self.update_gm();
	}

	fn update_gm(&mut self) {
		if self.level < 999 || self.score < 126_000 {
			self.gm_qualified = false;
			return;
		}
		let t300 = match self.frame_at_level_300 {
			Some(t) => t,
			None => {
				self.gm_qualified = false;
				return;
			}
		};
		let t500 = match self.frame_at_level_500 {
			Some(t) => t,
			None => {
				self.gm_qualified = false;
				return;
			}
		};
		let t999 = match self.frame_at_level_999 {
			Some(t) => t,
			None => {
				self.gm_qualified = false;
				return;
			}
		};
		let gate300 = (4 * 60 + 15) * 60;
		let gate500 = (7 * 60 + 30) * 60;
		let gate999 = (13 * 60 + 30) * 60;
		let g300 = self.score_at_level_300.is_some_and(|s| s >= 12_000);
		let g500 = self.score_at_level_500.is_some_and(|s| s >= 40_000);
		self.gm_qualified = t300 < gate300 && t500 < gate500 && t999 < gate999 && g300 && g500;
	}

	pub fn step(&mut self, input: Input) {
		if self.game_over || self.cleared {
			return;
		}
		self.frame += 1;

		match self.phase {
			Phase::LineClear => self.step_line_clear(input),
			Phase::Are => self.step_are(input),
			Phase::Falling => self.step_falling(input),
		}
	}

	fn step_line_clear(&mut self, input: Input) {
		if self.line_clear_timer > 0 {
			self.advance_das_horizontal(input);
			self.line_clear_timer -= 1;
			return;
		}
		if let Some(full) = self.pending_lines.take() {
			clear_lines(&mut self.board, &full);
		}
		self.are_timer = self.options.are_frames();
		self.phase = Phase::Are;
	}

	fn step_are(&mut self, input: Input) {
		if self.are_timer > 0 {
			self.advance_das_horizontal(input);
			self.are_timer -= 1;
			return;
		}
		self.spawn_piece(input);
		if self.game_over {
			return;
		}
		self.phase = Phase::Falling;
	}

	fn spawn_piece(&mut self, input: Input) {
		let kind = self.next_kind;
		self.next_kind = self.rng.next_piece();
		let irs_requested = input.rot_cw || input.rot_ccw;
		let mut rot = if input.rot_cw {
			rotate_cw(kind, 0)
		} else if input.rot_ccw {
			rotate_ccw(kind, 0)
		} else {
			0
		};
		let (mut px, mut py) = self.spawn_position(kind, rot);

		if irs_requested && rot != 0 && self.board.collides(px, py, kind, rot) {
			rot = 0;
			(px, py) = self.spawn_position(kind, rot);
		}

		let spawned = PieceState {
			kind,
			rot,
			x: px,
			y: py,
		};
		if self.board.collides(px, py, kind, rot) {
			self.game_over = true;
			// Keep the blocking piece so clients can render how the game ended.
			self.piece = Some(spawned);
			return;
		}

		self.piece = Some(spawned);
		self.gravity_accum = 0;
		self.soft_frames_this_piece = 0;
		self.lock_delay = LOCK_DELAY_FRAMES;

		if let Some(lv) = level_after_piece_spawn(self.level) {
			self.level = lv;
			self.record_level_milestone();
		}
	}

	/// Final spawn position after origin + 20G placement.
	fn spawn_position(&self, kind: PieceKind, rot: RotIndex) -> (i32, i32) {
		let (ox, oy) = spawn_origin(kind, rot);
		let mut py = oy;
		let px = ox;
		let g = effective_gravity(self.level);
		if g >= 5120 {
			py = self.board.drop_to_bottom(px, py, kind, rot);
			py = self.board.rise_to_valid(px, py, kind, rot);
		}
		(px, py)
	}

	fn advance_das_horizontal(&mut self, input: Input) -> (bool, bool) {
		let mut move_left = false;
		let mut move_right = false;
		if input.left {
			self.das_right = 0;
			self.das_left = self.das_left.saturating_add(1);
			move_left = self.das_left == 1
				|| (self.das_left >= DAS_FRAMES
					&& (self.das_left - DAS_FRAMES).is_multiple_of(DAS_REPEAT_FRAMES));
		} else {
			self.das_left = 0;
		}
		if input.right {
			self.das_left = 0;
			self.das_right = self.das_right.saturating_add(1);
			move_right = self.das_right == 1
				|| (self.das_right >= DAS_FRAMES
					&& (self.das_right - DAS_FRAMES).is_multiple_of(DAS_REPEAT_FRAMES));
		} else {
			self.das_right = 0;
		}
		(move_left, move_right)
	}

	fn step_falling(&mut self, input: Input) {
		let Some(mut p) = self.piece else {
			self.phase = Phase::Are;
			self.are_timer = self.options.are_frames();
			return;
		};

		let g = effective_gravity(self.level);

		if input.rot_ccw {
			if let Some((nx, ny, nr)) = try_rotate_ccw(&self.board, p.x, p.y, p.kind, p.rot) {
				p.x = nx;
				p.y = ny;
				p.rot = nr;
			}
		} else if input.rot_cw {
			if let Some((nx, ny, nr)) = try_rotate_cw(&self.board, p.x, p.y, p.kind, p.rot) {
				p.x = nx;
				p.y = ny;
				p.rot = nr;
			}
		}

		let (move_left, move_right) = self.advance_das_horizontal(input);
		if move_left && !self.board.collides(p.x - 1, p.y, p.kind, p.rot) {
			p.x -= 1;
		}
		if move_right && !self.board.collides(p.x + 1, p.y, p.kind, p.rot) {
			p.x += 1;
		}

		if input.sonic {
			let ny = self.board.drop_to_bottom(p.x, p.y, p.kind, p.rot);
			if ny != p.y {
				p.y = ny;
				self.lock_delay = LOCK_DELAY_FRAMES;
			}
		}

		if input.down {
			self.soft_frames_this_piece = self.soft_frames_this_piece.saturating_add(1);
		}

		if g >= 5120 {
			let old_y = p.y;
			p.y = self.board.drop_to_bottom(p.x, p.y, p.kind, p.rot);
			p.y = self.board.rise_to_valid(p.x, p.y, p.kind, p.rot);
			if p.y < old_y {
				self.lock_delay = LOCK_DELAY_FRAMES;
			}
		} else if input.down {
			if !self.board.collides(p.x, p.y - 1, p.kind, p.rot) {
				p.y -= 1;
				self.lock_delay = LOCK_DELAY_FRAMES;
			}
		} else {
			self.gravity_accum = self.gravity_accum.saturating_add(g as u32);
			while self.gravity_accum >= 256 {
				self.gravity_accum -= 256;
				if !self.board.collides(p.x, p.y - 1, p.kind, p.rot) {
					p.y -= 1;
					self.lock_delay = LOCK_DELAY_FRAMES;
				} else {
					break;
				}
			}
		}

		let grounded = self.board.collides(p.x, p.y - 1, p.kind, p.rot);
		if grounded {
			if input.down {
				self.lock_piece(p);
				return;
			}
			if self.lock_delay > 0 {
				self.lock_delay -= 1;
			}
			if self.lock_delay == 0 {
				self.lock_piece(p);
				return;
			}
		}

		self.piece = Some(p);
	}

	fn lock_piece(&mut self, p: PieceState) {
		let color = p.kind as u8 + 1;
		self.board.lock_piece(p.x, p.y, p.kind, p.rot, color);
		self.piece = None;

		let full = find_full_lines(&self.board);
		let n = count_full_lines(&full);
		let level_before = self.level;

		if n > 0 {
			let mut tmp = self.board.clone();
			clear_lines(&mut tmp, &full);
			let empty_after = tmp.rows.iter().all(|row| row.iter().all(|&c| c == EMPTY));

			self.score = add_score(
				self.score,
				level_before,
				n,
				self.soft_frames_this_piece,
				&mut self.combo,
				bravo_factor(empty_after),
			);

			self.pending_lines = Some(full);
			self.line_clear_timer = self.options.line_clear_frames();
			self.phase = Phase::LineClear;

			for _ in 0..n {
				if self.level >= 999 {
					break;
				}
				self.level += 1;
				self.record_level_milestone();
			}
		} else {
			self.score = add_score(
				self.score,
				level_before,
				0,
				self.soft_frames_this_piece,
				&mut self.combo,
				1,
			);
			self.are_timer = self.options.are_frames();
			self.phase = Phase::Are;
		}

		if self.level >= 999 {
			self.cleared = true;
		}
	}
}

#[cfg(test)]
mod input_pack_tests {
	use super::{input_pack, input_unpack};

	#[test]
	fn pack_unpack_round_trip_all_6bit() {
		for b in 0u8..64 {
			let i = input_unpack(b).expect("valid 6-bit");
			assert_eq!(input_pack(i), b);
		}
	}

	#[test]
	fn unpack_rejects_reserved_bits() {
		assert!(input_unpack(0x40).is_none());
		assert!(input_unpack(0x80).is_none());
		assert!(input_unpack(0xFF).is_none());
	}
}
