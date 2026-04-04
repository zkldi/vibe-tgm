//! HUD layout, chrome, and stress visuals. Optional `y_jolt` shifts panels vertically (W/S).

use macroquad::prelude::*;
use tgm_core::{
	Game, GameOptions, PieceKind, effective_gravity, line_clear_only_for_increment, piece_cells,
	piece_cells_big,
};

/// Short pulse when score tier improves (see [`GradeUpAnim`]).
const GRADE_UP_PANEL_FLASH: Color = Color::from_rgba(255, 230, 120, 255);

/// Pop + color flash on the grade readout (`t` ∈ 0..1).
#[derive(Clone, Copy, Debug)]
pub struct GradeUpAnim {
	t: f32,
}

impl GradeUpAnim {
	pub const DURATION_SEC: f32 = 0.9;

	pub fn new() -> Self {
		Self { t: 0.0 }
	}

	pub fn tick(&mut self, dt: f32) {
		self.t += dt / Self::DURATION_SEC;
	}

	pub fn finished(&self) -> bool {
		self.t >= 1.0
	}

	/// Normalized time for easing (0 = start, 1 = end).
	pub fn t01(self) -> f32 {
		self.t.clamp(0.0, 1.0)
	}
}

use crate::theme::{
	ArcadeFont, GRADE_VALUE, HUD_LABEL, PANEL_BG, PLAYFIELD_FRAME_PAD, TEXT_HELP, cell_color,
	darken, draw_cell_beveled, draw_hud_panel, lighten,
};

pub const HUD_W: f32 = 224.0;

/// Height of the NEXT panel (label + mini preview) only.
pub const NEXT_ZONE_H: f32 = 64.0;
/// Empty gap between the bottom of the NEXT panel and the playfield top.
pub const NEXT_PLAYFIELD_GAP: f32 = 8.0;
/// Space reserved below the playfield frame for the TGM-style timer.
pub const TIMER_ZONE_H: f32 = 48.0;

const MARGIN: f32 = 24.0;
/// Baseline inset from HUD panel top (`draw_text_ex` y is baseline; leave room for ascent).
const PANEL_TEXT_TOP: f32 = 24.0;
/// Baseline offset from the NEXT band top for the “NEXT” label (keeps label high so preview fits).
const NEXT_STRIP_LABEL_BASELINE: f32 = 12.0;
/// Gap between outer playfield chrome and timer baseline.
const TIMER_BELOW_CHROME_GAP: f32 = 11.0;

const ROT_DECAY: f32 = 14.0;
const ROT_BUMP: f32 = 0.42;
const ROT_BUMP_REPLAY: f32 = 0.33;
/// Slow breathing when level is gated behind line clears (~3.2 s period).
const GATE_PERIOD_SEC: f32 = 3.2;
const LEVEL_GATE_AMBER: Color = Color::from_rgba(255, 218, 165, 255);

/// Decaying rotation impulses (JKL / replay) for HUD stress accents.
#[derive(Clone, Copy, Default)]
pub struct HudRotFeel {
	pub cw: f32,
	pub ccw: f32,
}

impl HudRotFeel {
	/// `bump_*` are edge-style (live: key pressed this frame; replay: rotation in stepped input).
	/// `replay_scale` uses a smaller bump so one-frame replay pulses do not overshoot.
	pub fn tick(&mut self, dt: f32, bump_cw: bool, bump_ccw: bool, replay_scale: bool) {
		let k = (-dt * ROT_DECAY).exp();
		self.cw *= k;
		self.ccw *= k;
		let b = if replay_scale {
			ROT_BUMP_REPLAY
		} else {
			ROT_BUMP
		};
		if bump_cw {
			self.cw = (self.cw + b).min(1.0);
		}
		if bump_ccw {
			self.ccw = (self.ccw + b).min(1.0);
		}
	}
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
	let t = t.clamp(0.0, 1.0);
	Color::new(
		a.r + (b.r - a.r) * t,
		a.g + (b.g - a.g) * t,
		a.b + (b.b - a.b) * t,
		a.a + (b.a - a.a) * t,
	)
}

fn hud_stress(game: &Game, opts: &GameOptions, rot: &HudRotFeel, hud_time: f32) -> f32 {
	let g = effective_gravity(game.level, opts);
	let g_norm = (g as f32 / 5120.0).min(1.0);
	let grav = game.gravity_accum as f32 / 256.0;
	let mut s = grav * 0.42 + g_norm * 0.38;
	s += rot.cw * 0.12 + rot.ccw * 0.12;
	if line_clear_only_for_increment(game.level) {
		let breath01 = gate_stress_breath01(hud_time);
		s += breath01 * 0.07;
	}
	s.min(1.0)
}

fn gate_stress_breath01(hud_time: f32) -> f32 {
	let g = (hud_time * (std::f32::consts::TAU / GATE_PERIOD_SEC)).sin();
	g * 0.5 + 0.5
}

fn draw_row(
	font: &ArcadeFont,
	label: &str,
	value: &str,
	label_x: f32,
	value_right: f32,
	y: f32,
	label_sz: f32,
	value_sz: f32,
	value_color: Color,
) {
	font.draw(label, label_x, y, label_sz, HUD_LABEL);
	let vw = font.measure(value, value_sz).width;
	font.draw(value, value_right - vw, y, value_sz, value_color);
}

fn draw_modes_line(font: &ArcadeFont, x: f32, y: f32, opts: &GameOptions) {
	let mut parts = Vec::new();
	if opts.force_20g {
		parts.push("20G");
	}
	if opts.big {
		parts.push("BIG");
	}
	if opts.reverse {
		parts.push("REV");
	}
	if opts.tls_always {
		parts.push("TLS");
	}
	if opts.monochrome {
		parts.push("MONO");
	}
	if opts.uki {
		parts.push("UKI");
	}
	if parts.is_empty() {
		font.draw("NORMAL", x, y, 10.0, TEXT_HELP);
	} else {
		font.draw(&parts.join(" · "), x, y, 10.0, TEXT_HELP);
	}
}

fn draw_mini_piece(x: f32, y: f32, kind: PieceKind, mono: bool) {
	let def = piece_cells(kind, 0);
	let s = 11.0;
	let col = cell_color(kind as u8 + 1, mono);
	for (dx, dy) in def.cells {
		let px = x + dx as f32 * s;
		let py = y + (3.0 - dy as f32) * s;
		draw_cell_beveled(px, py, s, col);
	}
}

fn draw_mini_piece_big(x: f32, y: f32, kind: PieceKind, mono: bool) {
	let def = piece_cells_big(kind, 0);
	let s = 14.0;
	let col = cell_color(kind as u8 + 1, mono);
	for (dx, dy) in def.cells {
		let px = x + dx as f32 * s * 0.5;
		let py = y + (3.0 - dy as f32) * s * 0.5;
		draw_cell_beveled(px, py, s * 0.5, col);
	}
}

/// Max Y extent of [`draw_mini_piece`] / [`draw_mini_piece_big`] (anchor `y` = top row).
fn next_preview_span_y(big: bool) -> f32 {
	if big { 3.0 * 14.0 * 0.5 } else { 3.0 * 11.0 }
}

fn draw_gravity_pressure_bar(
	x: f32,
	y: f32,
	w: f32,
	game: &Game,
	opts: &GameOptions,
	gate: bool,
	hud_time: f32,
) {
	let frac = (game.gravity_accum as f32 / 256.0).clamp(0.0, 1.0);
	let g = effective_gravity(game.level, opts);
	let hot = (g as f32 / 5120.0).min(1.0);
	let base = darken(PANEL_BG, 0.25);
	let fill = lighten(
		Color::from_rgba(80, 120, 160, 255),
		0.15 + hot * 0.35 + frac * 0.15,
	);
	let h = 2.0;
	draw_rectangle(x, y, w, h, base);
	let wf = w * frac;
	let wf_draw = if gate {
		(wf * (1.0 + 0.018 * (hud_time * 5.7).sin())).min(w)
	} else {
		wf
	};
	draw_rectangle(x, y, wf_draw, h, fill);
}

/// `mm:ss:cs` at 60 Hz (centiseconds derived from the frame index within each second).
fn format_time_tgm(frame: u64) -> String {
	let fc = frame;
	let mm = fc / 3600;
	let ss = (fc % 3600) / 60;
	let cs = ((fc % 60) * 100 / 60).min(99);
	format!("{:02}:{:02}:{:02}", mm, ss, cs)
}

/// NEXT label + preview centered on the playfield, in the band above `field_top_y`.
/// Call **after** drawing the playfield so the mini-piece draws above the stack (z-order).
pub fn draw_next_strip(
	font: &ArcadeFont,
	game: &Game,
	opts: &GameOptions,
	rot_feel: &HudRotFeel,
	hud_time: f32,
	ox: f32,
	field_w: f32,
	band_top_y: f32,
	field_top_y: f32,
	big: bool,
) {
	let stress = hud_stress(game, opts, rot_feel, hud_time);
	let next_stress = (stress * 0.85).min(1.0);
	let panel_bottom_y = band_top_y + NEXT_ZONE_H;
	draw_hud_panel(ox, band_top_y, field_w, NEXT_ZONE_H, next_stress, false);
	let pad = 10.0;
	let inner = ox + pad;
	let label_baseline = band_top_y + NEXT_STRIP_LABEL_BASELINE;
	let label = if big { "NEXT (BIG)" } else { "NEXT" };
	let label_sz = 11.0;
	let lm = font.measure(label, label_sz);
	// Macroquad: text occupies y in [baseline - offset_y, baseline - offset_y + height].
	let label_bottom = label_baseline - lm.offset_y + lm.height;
	let gap_below_label = 4.0;
	let gap_above_field = 3.0;
	let span_y = next_preview_span_y(big);
	let py_min = label_bottom + gap_below_label;
	let py_max =
		(panel_bottom_y - gap_above_field - span_y).min(field_top_y - gap_above_field - span_y);
	let py = py_min.min(py_max);
	font.draw(label, inner, label_baseline, label_sz, HUD_LABEL);
	let preview_w = if big { 28.0 } else { 44.0 };
	let px = ox + (field_w - preview_w) * 0.5;
	if big {
		draw_mini_piece_big(px, py, game.next_kind, opts.monochrome);
	} else {
		draw_mini_piece(px, py, game.next_kind, opts.monochrome);
	}
}

/// Large timer below the playfield chrome, centered on the field width.
/// `field_inner_bottom_y` is the bottom edge of the cell rect (same `y + h` passed to
/// [`crate::theme::draw_playfield_frame`]). Macroquad’s `draw_text` `y` is the **baseline**;
/// the bitmap sits in `Rect::new(x, y - offset_y, w, h)`, so we add `measure.offset_y`
/// so the top of the glyph box clears the chrome.
pub fn draw_timer_below_field(
	font: &ArcadeFont,
	game: &Game,
	ox: f32,
	field_w: f32,
	field_inner_bottom_y: f32,
) {
	let t = format_time_tgm(game.frame);
	let sz = 18.0;
	let m = font.measure(&t, sz);
	let tw = m.width;
	let chrome_bottom = field_inner_bottom_y + PLAYFIELD_FRAME_PAD;
	// Top of text = baseline - offset_y; want that >= chrome_bottom + gap  ⇒  baseline = chrome +
	// gap + offset_y
	let baseline_y = chrome_bottom + TIMER_BELOW_CHROME_GAP + m.offset_y;
	font.draw(&t, ox + (field_w - tw) * 0.5, baseline_y, sz, WHITE);
}

/// Right-side rail (TGM1 “Free Play” / replay title). `rail_x` is the left edge of the panel.
pub fn draw_right_rail(
	font: &ArcadeFont,
	rail_x: f32,
	hud_time: f32,
	primary: &str,
	secondary: Option<&str>,
	panel_y: f32,
) {
	let stress = 0.16 + 0.05 * (hud_time * 0.85).sin();
	let primary_sz = 15.0;
	let m = font.measure(primary, primary_sz);
	let panel_h = if secondary.is_some() { 92.0 } else { 64.0 };
	draw_hud_panel(rail_x, panel_y, HUD_W, panel_h, stress, true);
	font.draw(
		primary,
		rail_x + (HUD_W - m.width) * 0.5,
		panel_y + 24.0,
		primary_sz,
		HUD_LABEL,
	);
	if let Some(s) = secondary {
		let sm = font.measure(s, 11.0);
		font.draw(
			s,
			rail_x + (HUD_W - sm.width) * 0.5,
			panel_y + 50.0,
			11.0,
			TEXT_HELP,
		);
	}
}

pub fn draw_hud(
	font: &ArcadeFont,
	game: &Game,
	opts: &GameOptions,
	rot_feel: &HudRotFeel,
	hud_time: f32,
	y_jolt: f32,
	grade_up_t01: Option<f32>,
) {
	draw_hud_at(
		font,
		game,
		opts,
		rot_feel,
		hud_time,
		MARGIN,
		false,
		y_jolt,
		grade_up_t01,
	);
}

pub fn draw_hud_big(
	font: &ArcadeFont,
	game: &Game,
	opts: &GameOptions,
	rot_feel: &HudRotFeel,
	hud_time: f32,
	y_jolt: f32,
	grade_up_t01: Option<f32>,
) {
	draw_hud_at(
		font,
		game,
		opts,
		rot_feel,
		hud_time,
		MARGIN,
		true,
		y_jolt,
		grade_up_t01,
	);
}

fn draw_hud_at(
	font: &ArcadeFont,
	game: &Game,
	opts: &GameOptions,
	rot_feel: &HudRotFeel,
	hud_time: f32,
	hx: f32,
	_big: bool,
	y_jolt: f32,
	grade_up_t01: Option<f32>,
) {
	let stress = hud_stress(game, opts, rot_feel, hud_time);
	let level_gate = line_clear_only_for_increment(game.level);
	let breath01 = if level_gate {
		gate_stress_breath01(hud_time)
	} else {
		0.0
	};
	let level_color = if level_gate {
		lerp_color(WHITE, LEVEL_GATE_AMBER, 0.04 + breath01 * 0.07)
	} else {
		WHITE
	};
	let pad = 10.0;
	let inner = hx + pad;
	let value_right = hx + HUD_W - pad;
	let mut y = MARGIN + y_jolt;

	let stats_h = 96.0;
	draw_hud_panel(hx, y, HUD_W, stats_h, stress, true);

	let mut ty = y + PANEL_TEXT_TOP;
	let val_sz = 14.0;
	let label_sz = 11.0;
	const STATS_ROW_STEP: f32 = 24.0;

	draw_row(
		font,
		"LEVEL",
		&format!("{}", game.level),
		inner,
		value_right,
		ty,
		label_sz,
		val_sz,
		level_color,
	);
	ty += STATS_ROW_STEP;
	draw_row(
		font,
		"SCORE",
		&format!("{}", game.score),
		inner,
		value_right,
		ty,
		label_sz,
		val_sz,
		WHITE,
	);
	ty += STATS_ROW_STEP;

	let hi_ok = game.eligible_for_hiscore();
	let hi_val = if hi_ok { "OK" } else { "OFF" };
	let hi_color = Color::from_rgba(140, 200, 150, 255);
	draw_row(
		font,
		"HISCORE",
		hi_val,
		inner,
		value_right,
		ty,
		label_sz,
		11.0,
		hi_color,
	);

	// Match panel inset used by [`crate::theme::draw_hud_panel`] bottom rule (`x + 4`).
	let bar_inset = 4.0f32;
	let bar_y = y + stats_h - 6.0;
	draw_gravity_pressure_bar(
		hx + bar_inset,
		bar_y,
		HUD_W - bar_inset * 2.0,
		game,
		opts,
		level_gate,
		hud_time,
	);

	y += stats_h + 8.0;

	// Center "GRADE" + value as a block in the panel (macroquad: y is baseline; use
	// offset_y/height).
	let gl = game.grade_label();
	let lm = font.measure("GRADE", 11.0);
	let gap = 8.0f32;
	let t_up = grade_up_t01.unwrap_or(0.0);
	let pop = if grade_up_t01.is_some() {
		0.28 * (t_up * std::f32::consts::PI).sin().powf(1.15)
	} else {
		0.0
	};
	let val_sz = 24.0 * (1.0 + pop);
	let vm = font.measure(gl, val_sz);
	let grade_body_h = lm.height + gap + vm.height;
	let grade_h = (grade_body_h + 32.0).max(84.0);
	let grade_stress = (stress * 0.92 + pop * 0.55).min(1.0);
	draw_hud_panel(hx, y, HUD_W, grade_h, grade_stress, true);
	if let Some(t) = grade_up_t01 {
		let flash_a = ((1.0 - t).powf(1.6) * 0.22).clamp(0.0, 1.0);
		let mut c = GRADE_UP_PANEL_FLASH;
		c.a *= flash_a;
		draw_rectangle(hx, y, HUD_W, grade_h, c);
	}
	let content_w = HUD_W - 2.0 * pad;
	let label_x = inner + (content_w - lm.width) * 0.5;
	let gx = inner + (content_w - vm.width) * 0.5;
	let b1 = y + grade_h * 0.5 + lm.offset_y - (lm.height + gap + vm.height) * 0.5;
	let b2 = b1 - lm.offset_y + lm.height + gap + vm.offset_y;
	let grade_draw_color = if grade_up_t01.is_some() {
		let spark = (1.0 - t_up).powf(1.8);
		lerp_color(GRADE_VALUE, lighten(WHITE, 0.38), spark * 0.92)
	} else {
		GRADE_VALUE
	};
	font.draw("GRADE", label_x, b1, 11.0, HUD_LABEL);
	font.draw(gl, gx, b2, val_sz, grade_draw_color);
	y += grade_h + 8.0;

	draw_modes_line(font, inner, y, opts);
}
