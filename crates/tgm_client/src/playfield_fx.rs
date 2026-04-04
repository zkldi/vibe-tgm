//! Client-only VFX: line clear, lock flash, game-over rust/shake — driven by [`tgm_core::Game`]
//! state.

use macroquad::prelude::*;
use tgm_core::{
	BOARD_WIDTH, EMPTY, Game, Input, Phase, PieceState, VISIBLE_ROWS, find_full_lines,
	input_unpack, piece_cells,
};

/// Frames of death animation (shake + rust + border pulse).
pub const DEATH_FRAMES_MAX: u32 = 100;
const LOCK_FLASH_FRAMES: u32 = 8;
const LOCK_FLASH_SHORT: u32 = 2;

const WALL_SMOOTH_RATE: f32 = 14.0;

/// Max visual trail length (board rows) for a sonic drop — keeps long drops readable.
const SONIC_SLAM_MAX_ROWS: f32 = 6.0;
/// Exponential decay per second for the slam offset (`exp(-rate * dt)`).
const SONIC_SLAM_DECAY: f32 = 240.0;

/// Instant vertical offset for HUD (W/S), in pixels. No smoothing.
pub const HUD_VERTICAL_JOLT: f32 = 3.0;

/// Smoothed A/D for playfield chrome colour only (no geometry).
#[derive(Clone, Debug)]
pub struct WallInputFeel {
	pub smooth_x: f32,
}

impl Default for WallInputFeel {
	fn default() -> Self {
		Self { smooth_x: 0.0 }
	}
}

impl WallInputFeel {
	pub fn tick_horizontal(&mut self, dt: f32, target_x: f32) {
		let k = 1.0 - (-dt * WALL_SMOOTH_RATE).exp();
		self.smooth_x += (target_x - self.smooth_x) * k;
	}

	/// 0..1 from |A/D| (chrome intensity).
	pub fn horizontal_activity(&self) -> f32 {
		self.smooth_x.abs().clamp(0.0, 1.0)
	}
}

/// -1 / 0 / +1 from live A/D.
pub fn horizontal_target_from_keys() -> f32 {
	let mut x = 0.0f32;
	if is_key_down(KeyCode::A) {
		x -= 1.0;
	}
	if is_key_down(KeyCode::D) {
		x += 1.0;
	}
	x.clamp(-1.0, 1.0)
}

pub fn horizontal_target_from_replay_byte(last_byte: Option<u8>) -> f32 {
	let Some(b) = last_byte else {
		return 0.0;
	};
	let Some(inp) = input_unpack(b) else {
		return 0.0;
	};
	let mut x = 0.0f32;
	if inp.left {
		x -= 1.0;
	}
	if inp.right {
		x += 1.0;
	}
	x.clamp(-1.0, 1.0)
}

/// W up / S down — instant, no decay (for HUD placement).
pub fn hud_vertical_jolt_from_keys() -> f32 {
	let mut v = 0.0f32;
	if is_key_down(KeyCode::W) {
		v -= HUD_VERTICAL_JOLT;
	}
	if is_key_down(KeyCode::S) {
		v += HUD_VERTICAL_JOLT;
	}
	v
}

pub fn hud_vertical_jolt_from_replay_byte(last_byte: Option<u8>) -> f32 {
	let Some(b) = last_byte else {
		return 0.0;
	};
	let Some(inp) = input_unpack(b) else {
		return 0.0;
	};
	let mut v = 0.0f32;
	if inp.sonic {
		v -= HUD_VERTICAL_JOLT;
	}
	if inp.down {
		v += HUD_VERTICAL_JOLT;
	}
	v
}

#[derive(Clone, Debug)]
pub struct PlayfieldFx {
	lock_flash_cells: Vec<(i32, i32)>,
	lock_flash_timer: u32,
	/// 0 = not dead yet; 1..= [`DEATH_FRAMES_MAX`] after game over.
	pub death_frames: u32,
	pub wall_input: WallInputFeel,
	/// Visual-only: piece is drawn offset along gravity so it appears to slam into place after
	/// sonic.
	sonic_slam_cells: f32,
	sonic_slam_reverse: bool,
}

impl Default for PlayfieldFx {
	fn default() -> Self {
		Self {
			lock_flash_cells: Vec::new(),
			lock_flash_timer: 0,
			death_frames: 0,
			wall_input: WallInputFeel::default(),
			sonic_slam_cells: 0.0,
			sonic_slam_reverse: false,
		}
	}
}

impl PlayfieldFx {
	pub fn reset(&mut self) {
		self.lock_flash_cells.clear();
		self.lock_flash_timer = 0;
		self.death_frames = 0;
		self.wall_input.smooth_x = 0.0;
		self.sonic_slam_cells = 0.0;
	}

	/// 0..1 strength for fullscreen post blur (tracks slam offset).
	/// Scales down in mid–high level bands where gravity makes sonic hits max out every drop.
	pub fn sonic_slam_blur(&self, level: u16) -> f32 {
		let t = (self.sonic_slam_cells / SONIC_SLAM_MAX_ROWS).clamp(0.0, 1.0);
		let base = t * t;
		let scale = sonic_slam_blur_level_scale(level);
		(base * scale).min(1.0)
	}

	/// Extra screen-space Y for active minos after a sonic drop (decays in a few frames).
	pub fn sonic_slam_screen_y(&self, cell: f32) -> f32 {
		if self.sonic_slam_cells <= 0.0 {
			return 0.0;
		}
		let o = self.sonic_slam_cells * cell;
		if self.sonic_slam_reverse { o } else { -o }
	}

	/// Call after each simulation `step` with the piece state **before** that step.
	pub fn after_step(&mut self, game: &Game, piece_before: Option<PieceState>, input: Input) {
		if let (Some(pb), Some(pa)) = (piece_before, game.piece) {
			if input.sonic {
				let rows = (pb.y - pa.y).max(0);
				if rows > 0 {
					let add = (rows as f32).min(SONIC_SLAM_MAX_ROWS);
					self.sonic_slam_cells = self.sonic_slam_cells.max(add);
					self.sonic_slam_reverse = false;
				}
			}
		}

		if piece_before.is_some() && game.piece.is_none() && !game.game_over {
			self.lock_flash_timer = if game.phase == Phase::LineClear {
				LOCK_FLASH_SHORT
			} else {
				LOCK_FLASH_FRAMES
			};
			self.lock_flash_cells.clear();
			let p = piece_before.expect("checked");
			for (dx, dy) in piece_cells(p.kind, p.rot).cells {
				self.lock_flash_cells
					.push((p.x + dx as i32, p.y + dy as i32));
			}
		}
	}

	/// Decrement lock-flash at **start** of frame (before `step` + [`Self::after_step`]).
	pub fn tick_lock_flash(&mut self) {
		if self.lock_flash_timer > 0 {
			self.lock_flash_timer -= 1;
		}
	}

	/// Decay sonic slam offset; call each frame with `dt` (e.g. after lock flash tick).
	pub fn tick_sonic_slam(&mut self, dt: f32) {
		if self.sonic_slam_cells > 0.0 {
			self.sonic_slam_cells *= (-dt * SONIC_SLAM_DECAY).exp();
			if self.sonic_slam_cells < 0.008 {
				self.sonic_slam_cells = 0.0;
			}
		}
	}

	/// Clear slam offset (e.g. after bulk replay simulation with no per-frame decay).
	pub fn clear_sonic_slam(&mut self) {
		self.sonic_slam_cells = 0.0;
	}

	/// Advance death animation at **end** of frame (after simulation).
	pub fn tick_death_frame(&mut self, game: &Game) {
		if game.game_over {
			self.death_frames = (self.death_frames + 1).min(DEATH_FRAMES_MAX);
		}
	}

	pub fn death_shake(&self) -> (f32, f32) {
		if self.death_frames == 0 || self.death_frames >= DEATH_FRAMES_MAX {
			return (0.0, 0.0);
		}
		let t = self.death_frames as f32;
		let damp = 1.0 - (self.death_frames as f32 / DEATH_FRAMES_MAX as f32);
		let a = 3.8 * damp;
		((t * 1.7).sin() * a, (t * 2.1).cos() * a * 0.85)
	}

	/// 0 = none, 1 = full rust on stack.
	pub fn death_rust_amount(&self) -> f32 {
		if self.death_frames == 0 {
			return 0.0;
		}
		(self.death_frames.min(DEATH_FRAMES_MAX) as f32 / DEATH_FRAMES_MAX as f32).min(1.0)
	}

	pub fn apply_stack_color(&self, base: Color, bx: i32, by: i32, c: u8) -> Color {
		if c == EMPTY {
			return base;
		}
		let mut col = base;
		let rust = self.death_rust_amount();
		if rust > 0.0 {
			col = rust_tint(col, rust);
		}
		if self.lock_flash_timer > 0 {
			for &(lx, ly) in &self.lock_flash_cells {
				if lx == bx && ly == by {
					let pulse = (get_time() as f32 * 24.0).sin() * 0.5 + 0.5;
					return Color::new(
						(col.r + 0.22 * pulse).min(1.0),
						(col.g + 0.20 * pulse).min(1.0),
						(col.b + 0.17 * pulse).min(1.0),
						(col.a + 0.07 * pulse).min(1.0),
					);
				}
			}
		}
		col
	}

	pub fn draw_line_clear_normal(
		&self,
		game: &Game,
		ox: f32,
		cell: f32,
		board_screen_y: impl Fn(i32) -> f32,
	) {
		if game.phase != Phase::LineClear {
			return;
		}
		let full = find_full_lines(&game.board);
		let dur = game.options.line_clear_frames().max(1);
		let t = (dur - game.line_clear_timer) as f32 / dur as f32;
		self.draw_line_clear_rows(
			&full[..VISIBLE_ROWS],
			t,
			ox,
			cell,
			BOARD_WIDTH,
			board_screen_y,
		);
	}

	fn draw_line_clear_rows(
		&self,
		full: &[bool],
		t: f32,
		ox: f32,
		cell: f32,
		board_width: usize,
		board_screen_y: impl Fn(i32) -> f32,
	) {
		let flash = if t < 0.35 {
			let e = 1.0 - (t / 0.35);
			e * e
		} else {
			(1.0 - (t - 0.35) / 0.65) * 0.35
		};
		let fw = cell * board_width as f32;

		for (y, is_full) in full.iter().enumerate() {
			if !*is_full {
				continue;
			}
			let py = board_screen_y(y as i32);
			let row_h = cell;
			let flash_col = Color::from_rgba(255, 255, 255, (flash * 220.0) as u8);
			draw_rectangle(ox, py, fw, row_h, flash_col);
			let cyan = Color::from_rgba(180, 240, 255, (flash * 90.0) as u8);
			draw_rectangle(ox, py + row_h * 0.35, fw, row_h * 0.3, cyan);

			if t > 0.35 {
				let p2 = ((t - 0.35) / 0.65).clamp(0.0, 1.0);
				let spark_a = (p2 * 200.0) as u32;
				if spark_a > 8 {
					for i in 0..14 {
						let seed = (y * 131 + i * 17) as f32;
						let ox2 = (seed.sin() * 0.5 + 0.5) * fw;
						let spread = p2 * cell * 2.2;
						let dx = (seed * 3.7).cos() * spread;
						let dy = ((seed + i as f32) * 2.1).sin() * spread * 0.4;
						let sx = ox + ox2 + dx;
						let sy = py + cell * 0.5 + dy;
						let s = (3.0 + (i as f32 * 0.35)).min(7.0);
						let al = ((1.0 - p2) * spark_a as f32) as u8;
						draw_rectangle(sx, sy, s, s * 0.6, Color::from_rgba(220, 230, 245, al));
					}
				}
			}
		}
	}
}

fn sonic_slam_blur_level_scale(level: u16) -> f32 {
	match level {
		0..280 => 1.0,
		280..500 => {
			let t = (level - 280) as f32 / 220.0;
			(1.0 - 0.60 * t).clamp(0.35, 1.0)
		}
		_ => 0.40,
	}
}

fn rust_tint(base: Color, t: f32) -> Color {
	let rust = Color::from_rgba(110, 72, 48, 255);
	let gray = Color::new(
		base.r * 0.55 + base.g * 0.3 + base.b * 0.15,
		base.r * 0.55 + base.g * 0.3 + base.b * 0.15,
		base.r * 0.55 + base.g * 0.3 + base.b * 0.15,
		base.a,
	);
	Color::new(
		lerp(base.r, rust.r * 0.7 + gray.r * 0.3, t),
		lerp(base.g, rust.g * 0.7 + gray.g * 0.3, t),
		lerp(base.b, rust.b * 0.7 + gray.b * 0.3, t),
		base.a,
	)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}
