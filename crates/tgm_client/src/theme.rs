//! TGM-inspired visuals: dark industrial field, Sega mino colors, TGM3-style clockwork BG + chrome
//! frame.

use macroquad::prelude::*;

use crate::playfield_fx::DEATH_FRAMES_MAX;

/// Outer letterbox (matches dark scene).
pub const LETTERBOX: Color = Color::from_rgba(0, 0, 0, 255);

pub const TITLE_LINE: Color = Color::from_rgba(255, 220, 64, 255);
pub const CLEAR_TEXT: Color = Color::from_rgba(255, 240, 80, 255);
/// Section labels (NEXT, GRADE, MODE, etc.).
pub const HUD_LABEL: Color = Color::from_rgba(170, 170, 185, 255);
/// Grade number (arcade yellow on black).
pub const GRADE_VALUE: Color = Color::from_rgba(255, 210, 64, 255);
pub const TEXT_MUTED: Color = Color::from_rgba(130, 130, 145, 255);
pub const TEXT_HELP: Color = Color::from_rgba(100, 100, 115, 255);

pub const PANEL_BG: Color = Color::from_rgba(12, 12, 18, 250);
pub const PANEL_BORDER: Color = Color::from_rgba(90, 90, 102, 255);

pub use crate::backgrounds::{ClockworkBackground, draw_clockwork_background};

/// Outer inset of [`draw_playfield_frame`] lines beyond the inner cell rect (`y` / `h`).
pub const PLAYFIELD_FRAME_PAD: f32 = 5.0;

/// Per-edge pixel offsets (unused when all zero — kept for [`rect_outline_nudged`]).
#[derive(Clone, Copy, Default)]
pub struct PlayfieldWallNudge {
	pub left: f32,
	pub right: f32,
	pub top: f32,
	pub bottom: f32,
}

fn rect_outline_nudged(
	x: f32,
	y: f32,
	w: f32,
	h: f32,
	nudge: PlayfieldWallNudge,
	scale: f32,
	thickness: f32,
	color: Color,
) {
	let l = nudge.left * scale;
	let r = nudge.right * scale;
	let t = nudge.top * scale;
	let b = nudge.bottom * scale;
	let x0 = x - l;
	let y0 = y - t;
	let x1 = x + w + r;
	let y1 = y + h + b;
	draw_line(x0, y0, x1, y0, thickness, color);
	draw_line(x1, y0, x1, y1, thickness, color);
	draw_line(x1, y1, x0, y1, thickness, color);
	draw_line(x0, y1, x0, y0, thickness, color);
}

/// A/D only: no geometry — colour shifts with signed horizontal input (`signed` in -1..1).
fn chrome_horizontal_tint(base: Color, activity: f32, signed: f32) -> Color {
	let t = activity.clamp(0.0, 1.0);
	let s = signed.clamp(-1.0, 1.0);
	let s2 = t * t;
	let push_left = (-s).max(0.0);
	let push_right = s.max(0.0);
	Color::new(
		(base.r + 0.10 * s2 + 0.06 * t + 0.05 * push_right).min(1.0),
		(base.g + 0.12 * s2 + 0.08 * t + 0.07 * push_right).min(1.0),
		(base.b + 0.14 * s2 + 0.10 * t + 0.10 * push_left).min(1.0),
		base.a,
	)
}

/// Beveled chrome frame; `death_frames` tints red and pulses during game-over sequence.
/// `horizontal_activity` / `horizontal_signed` come from smoothed A/D (colour only).
pub fn draw_playfield_frame(
	x: f32,
	y: f32,
	w: f32,
	h: f32,
	death_frames: u32,
	horizontal_activity: f32,
	horizontal_signed: f32,
) {
	let death_t = death_frames.min(DEATH_FRAMES_MAX) as f32 / DEATH_FRAMES_MAX as f32;
	let pulse = if death_frames > 0 && death_frames < DEATH_FRAMES_MAX {
		(death_frames as f32 * 0.38).sin() * 0.5 + 0.5
	} else {
		0.0
	};
	let red_mix = death_t * 0.55 * (0.35 + pulse * 0.65);

	let outer_hi = Color::from_rgba(210, 212, 222, 255);
	let outer_lo = Color::from_rgba(88, 90, 102, 255);
	let inner_hi = Color::from_rgba(245, 246, 252, 255);
	let inner_lo = Color::from_rgba(150, 152, 168, 255);

	let mix = |c: Color| -> Color {
		Color::new(
			(c.r * (1.0 - red_mix) + red_mix).min(1.0),
			c.g * (1.0 - red_mix * 0.55),
			c.b * (1.0 - red_mix * 0.55),
			c.a,
		)
	};

	let h_act = horizontal_activity.clamp(0.0, 1.0);
	let h_s = horizontal_signed;
	let col = |c: Color| chrome_horizontal_tint(mix(c), h_act, h_s);

	let pad = PLAYFIELD_FRAME_PAD;
	let no_nudge = PlayfieldWallNudge::default();
	const S0: f32 = 1.0;
	const S1: f32 = 0.88;
	const S2: f32 = 0.62;
	const S3: f32 = 0.4;

	rect_outline_nudged(
		x - pad,
		y - pad,
		w + pad * 2.0,
		h + pad * 2.0,
		no_nudge,
		S0,
		2.5,
		col(outer_lo),
	);
	rect_outline_nudged(
		x - pad + 2.0,
		y - pad + 2.0,
		w + pad * 2.0 - 4.0,
		h + pad * 2.0 - 4.0,
		no_nudge,
		S1,
		1.8,
		col(outer_hi),
	);
	rect_outline_nudged(
		x - 1.0,
		y - 1.0,
		w + 2.0,
		h + 2.0,
		no_nudge,
		S2,
		1.2,
		col(inner_lo),
	);
	rect_outline_nudged(
		x + 1.0,
		y + 1.0,
		w - 2.0,
		h - 2.0,
		no_nudge,
		S3,
		1.0,
		col(inner_hi),
	);

	let bolt = |bx: f32, by: f32| {
		let hi = chrome_horizontal_tint(Color::from_rgba(160, 162, 175, 255), h_act, h_s);
		let lo = chrome_horizontal_tint(Color::from_rgba(60, 62, 72, 255), h_act, h_s);
		draw_circle_lines(bx, by, 2.5, 0.8, mix(hi));
		draw_circle_lines(bx, by, 1.0, 0.5, mix(lo));
	};
	let o = pad + 3.0;
	bolt(x - o, y - o);
	bolt(x + w + o, y - o);
	bolt(x - o, y + h + o);
	bolt(x + w + o, y + h + o);
}

/// Flat panel with one neutral border.
pub fn draw_panel(x: f32, y: f32, w: f32, h: f32) {
	draw_rectangle(x, y, w, h, PANEL_BG);
	draw_rectangle_lines(x, y, w, h, 1.0, PANEL_BORDER);
}

/// Instrument-style panel: double chrome + corner bolts, lighter than the playfield well.
/// `stress` in 0..1 nudges inner highlight (gravity / load).
pub fn draw_hud_panel(x: f32, y: f32, w: f32, h: f32, stress: f32, divider_bottom: bool) {
	let stress = stress.clamp(0.0, 1.0);
	let outer_hi = lighten(PANEL_BORDER, 0.55 + stress * 0.08);
	let outer_lo = darken(PANEL_BORDER, 0.12);
	let inner_hi = lighten(PANEL_BORDER, 0.95 + stress * 0.1);
	let inner_lo = darken(PANEL_BORDER, 0.02);

	let pad = 3.0;
	draw_rectangle(x, y, w, h, PANEL_BG);

	draw_rectangle_lines(
		x - pad,
		y - pad,
		w + pad * 2.0,
		h + pad * 2.0,
		2.0,
		outer_lo,
	);
	draw_rectangle_lines(
		x - pad + 1.5,
		y - pad + 1.5,
		w + pad * 2.0 - 3.0,
		h + pad * 2.0 - 3.0,
		1.4,
		outer_hi,
	);
	draw_rectangle_lines(x - 0.5, y - 0.5, w + 1.0, h + 1.0, 1.0, inner_lo);
	let inset = 1.0 + stress * 0.4;
	draw_rectangle_lines(
		x + inset,
		y + inset,
		w - inset * 2.0,
		h - inset * 2.0,
		0.9,
		inner_hi,
	);

	let bolt = |bx: f32, by: f32| {
		draw_circle_lines(bx, by, 2.0, 0.7, lighten(PANEL_BORDER, 0.35));
		draw_circle_lines(bx, by, 0.9, 0.45, darken(PANEL_BORDER, 0.25));
	};
	let o = pad + 2.0;
	bolt(x - o, y - o);
	bolt(x + w + o, y - o);
	bolt(x - o, y + h + o);
	bolt(x + w + o, y + h + o);

	if divider_bottom {
		let rule = Color::from_rgba(55, 55, 68, 200);
		draw_rectangle(x + 4.0, y + h - 1.0, w - 8.0, 1.0, rule);
	}
}

pub fn lighten(base: Color, amt: f32) -> Color {
	Color::new(
		(base.r + amt).min(1.0),
		(base.g + amt).min(1.0),
		(base.b + amt).min(1.0),
		base.a,
	)
}

pub fn darken(base: Color, amt: f32) -> Color {
	Color::new(
		(base.r * (1.0 - amt)).max(0.0),
		(base.g * (1.0 - amt)).max(0.0),
		(base.b * (1.0 - amt)).max(0.0),
		base.a,
	)
}

/// Gem-like minos: dark rim, glossy center (TGM3-ish).
pub fn draw_cell_beveled(x: f32, y: f32, cell: f32, base: Color) {
	let gap = 1.5;
	let w = cell - gap;
	let h = cell - gap;
	if w <= 0.0 || h <= 0.0 {
		return;
	}
	let rim = Color::new(base.r * 0.22, base.g * 0.22, base.b * 0.24, 0.92);
	draw_rectangle_lines(x - 0.5, y - 0.5, cell, cell, 1.25, rim);

	let bevel = (cell * 0.1).clamp(1.5, 4.0);
	let core = lighten(base, 0.06);
	draw_rectangle(x, y, w, h, core);
	let hi = lighten(base, 0.22);
	let hi_side = lighten(base, 0.14);
	let sh = darken(base, 0.38);
	let sh_side = darken(base, 0.26);
	let band = bevel.min(h * 0.42);
	draw_rectangle(x, y, w, band, hi);
	draw_rectangle(x, y, bevel.min(w * 0.42), h, hi_side);
	draw_rectangle(x, y + h - band, w, band, sh);
	draw_rectangle(
		x + w - bevel.min(w * 0.42),
		y,
		bevel.min(w * 0.42),
		h,
		sh_side,
	);

	let cx = x + w * 0.35;
	let cy = y + h * 0.32;
	let gloss = Color::new(
		(hi.r * 0.55 + 1.0 * 0.45).min(1.0),
		(hi.g * 0.55 + 1.0 * 0.45).min(1.0),
		(hi.b * 0.55 + 1.0 * 0.45).min(1.0),
		0.22,
	);
	draw_circle(cx, cy, cell * 0.14, gloss);
}

/// Piece / cell index color (`EMPTY` = dark well). Sega-style TGM1 palette.
pub fn cell_color(c: u8, mono: bool) -> Color {
	if mono {
		let v = 60 + (c as u32 * 18).min(160);
		return Color::from_rgba(v as u8, v as u8, v as u8, 255);
	}
	match c {
		1 => Color::from_rgba(0, 240, 240, 255), // I
		2 => Color::from_rgba(200, 0, 240, 255), // T (classic magenta)
		3 => Color::from_rgba(240, 160, 0, 255), // L
		4 => Color::from_rgba(0, 0, 240, 255),   // J
		5 => Color::from_rgba(0, 240, 0, 255),   // S
		6 => Color::from_rgba(240, 0, 0, 255),   // Z
		7 => Color::from_rgba(240, 240, 0, 255), // O
		_ => Color::from_rgba(18, 18, 24, 255),  // empty well
	}
}

pub fn dim_stack_cell(base: Color) -> Color {
	Color::new(base.r * 0.88, base.g * 0.88, base.b * 0.88, base.a * 0.98)
}

/// Darkening layer over the empty well (TGM1: no per-cell “background mino” grid).
///
/// Semi-transparent so the procedural background shows through; opaque fills would hide it entirely.
pub fn well_fill_color(mono: bool) -> Color {
	if mono {
		Color::from_rgba(20, 20, 22, 108)
	} else {
		Color::from_rgba(8, 9, 16, 112)
	}
}

/// Loaded pixel font for UI (Press Start 2P, OFL). Use after first `next_frame` if needed.
pub struct ArcadeFont(pub Font);

impl ArcadeFont {
	pub fn try_load() -> Result<Self, macroquad::Error> {
		let mut f =
			load_ttf_font_from_bytes(include_bytes!("../assets/fonts/PressStart2P-Regular.ttf"))?;
		f.set_filter(FilterMode::Nearest);
		for sz in [12_u16, 14, 16, 18, 20, 22, 24, 28, 32] {
			f.populate_font_cache(&Font::ascii_character_list(), sz);
		}
		Ok(ArcadeFont(f))
	}

	pub fn draw(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) -> TextDimensions {
		draw_text_ex(
			text,
			x,
			y,
			TextParams {
				font: Some(&self.0),
				font_size: font_size as u16,
				color,
				..Default::default()
			},
		)
	}

	pub fn measure(&self, text: &str, font_size: f32) -> TextDimensions {
		measure_text(text, Some(&self.0), font_size as u16, 1.0)
	}
}
