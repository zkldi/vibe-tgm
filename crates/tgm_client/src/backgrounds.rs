//! Distinct procedural backgrounds (mostly vector-only): each gameplay section uses a different
//! scene, not a palette swap. Section 5 adds text for a stock-site parody watermark.

use macroquad::prelude::*;

/// Full-screen pass after each procedural background: sections are authored dark; this is the
/// single brightness knob (there is no separate engine “dimmer” — the RT is blitted at full white).
const BG_SCREEN_LIFT_ALPHA: u8 = 42;

/// Clamp design-space dimensions: avoids NaN/∞ and `while` loops that never advance if step → 0.
fn sanitize_wh(w: f32, h: f32) -> (f32, f32) {
	let w = if w.is_finite() {
		w.clamp(1.0, 8192.0)
	} else {
		640.0
	};
	let h = if h.is_finite() {
		h.clamp(1.0, 8192.0)
	} else {
		480.0
	};
	(w, h)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
	Color::new(
		a.r + (b.r - a.r) * t,
		a.g + (b.g - a.g) * t,
		a.b + (b.b - a.b) * t,
		a.a + (b.a - a.a) * t,
	)
}

fn lighten(c: Color, amt: f32) -> Color {
	Color::new(
		(c.r + amt).min(1.0),
		(c.g + amt).min(1.0),
		(c.b + amt).min(1.0),
		c.a,
	)
}

fn darken(c: Color, amt: f32) -> Color {
	Color::new(
		(c.r * (1.0 - amt)).max(0.0),
		(c.g * (1.0 - amt)).max(0.0),
		(c.b * (1.0 - amt)).max(0.0),
		c.a,
	)
}

fn draw_vertical_gradient(top: Color, bottom: Color, w: f32, h: f32) {
	let rows = 28_u32;
	for i in 0..rows {
		let y0 = h * i as f32 / rows as f32;
		let y1 = h * (i + 1) as f32 / rows as f32;
		let u = if rows <= 1 {
			0.0
		} else {
			i as f32 / (rows - 1) as f32
		};
		draw_rectangle(0.0, y0, w, y1 - y0 + 0.5, lerp_color(top, bottom, u));
	}
}

/// Which backdrop to draw: gameplay sections (100-level bands; 800–899 and 900+ are distinct),
/// title, replay list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockworkBackground {
	Section0,
	Section1,
	Section2,
	Section3,
	Section4,
	Section5,
	Section6,
	Section7,
	Section8,
	Section9,
	Title,
	ReplayList,
}

impl ClockworkBackground {
	pub fn from_level(level: u16) -> Self {
		match level / 100 {
			0 => Self::Section0,
			1 => Self::Section1,
			2 => Self::Section2,
			3 => Self::Section3,
			4 => Self::Section4,
			5 => Self::Section5,
			6 => Self::Section6,
			7 => Self::Section7,
			8 => Self::Section8,
			_ => Self::Section9,
		}
	}

	/// Every backdrop variant, in a stable order (for the BG animation test screen).
	pub const ALL: &'static [ClockworkBackground] = &[
		Self::Section0,
		Self::Section1,
		Self::Section2,
		Self::Section3,
		Self::Section4,
		Self::Section5,
		Self::Section6,
		Self::Section7,
		Self::Section8,
		Self::Section9,
		Self::Title,
		Self::ReplayList,
	];

	pub fn label(self) -> &'static str {
		match self {
			Self::Section0 => "Section 0 (0-99) — Gears",
			Self::Section1 => "Section 1 (100-199) — Pipes",
			Self::Section2 => "Section 2 (200-299) — PCB",
			Self::Section3 => "Section 3 (300-399) — Hex",
			Self::Section4 => "Section 4 (400-499) — Furnace",
			Self::Section5 => "Section 5 (500-599) — Stock photo",
			Self::Section6 => "Section 6 (600-699) — Matrix",
			Self::Section7 => "Section 7 (700-799) — Void",
			Self::Section8 => "Section 8 (800-899) — Tissue + DNA",
			Self::Section9 => "Section 9 (900-999) — Finale",
			Self::Title => "Title — Luminous wall",
			Self::ReplayList => "Replay list",
		}
	}

	/// Base animation rate for this backdrop (multiply by wall-clock `dt`, then stack multiplier).
	pub fn section_speed(self) -> f32 {
		match self {
			Self::Section0 => 1.0,
			Self::Section1 => 1.08,
			Self::Section2 => 1.12,
			Self::Section3 => 1.15,
			Self::Section4 => 1.22,
			Self::Section5 => 1.28,
			Self::Section6 => 1.35,
			Self::Section7 => 0.1,
			Self::Section8 => 1.28,
			Self::Section9 => 0.58,
			Self::Title => 0.75,
			Self::ReplayList => 0.62,
		}
	}
}

// --- Section 0: rotating clockwork gears (classic mechanical) ---

fn draw_section_gears(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(6, 5, 8, 255);
	let bottom = Color::from_rgba(18, 14, 12, 255);
	draw_vertical_gradient(top, bottom, w, h);
	draw_rectangle(0.0, 0.0, w, h * 0.55, Color::from_rgba(0, 0, 0, 70));
	draw_rectangle(0.0, h * 0.45, w, h * 0.55, Color::from_rgba(20, 12, 8, 45));

	fn gear(cx: f32, cy: f32, r: f32, teeth: i32, rot: f32, thickness: f32, col: Color) {
		draw_circle_lines(cx, cy, r, thickness, col);
		let inner = r * 0.58;
		draw_circle_lines(cx, cy, inner, thickness * 0.65, darken(col, 0.15));
		let step = std::f32::consts::TAU / teeth as f32;
		for i in 0..teeth {
			let a = rot + i as f32 * step + step * 0.5;
			let r1 = r * 0.92;
			let r2 = r * 1.14;
			draw_line(
				cx + a.cos() * r1,
				cy + a.sin() * r1,
				cx + a.cos() * r2,
				cy + a.sin() * r2,
				thickness * 0.85,
				col,
			);
		}
	}

	let brass = Color::from_rgba(95, 78, 58, 200);
	let silver = Color::from_rgba(120, 122, 130, 160);
	let dim = Color::from_rgba(55, 52, 58, 110);
	gear(w * 0.12, h * 0.2, h * 0.42, 16, t * 0.09, 1.8, dim);
	gear(w * 0.88, h * 0.75, h * 0.38, 14, -t * 0.11, 1.6, dim);
	gear(w * 0.72, h * 0.18, h * 0.22, 12, -t * 0.22, 1.4, silver);
	gear(w * 0.08, h * 0.82, h * 0.2, 10, t * 0.28, 1.2, brass);
	gear(
		w * 0.45,
		h * 0.5,
		h * 0.14,
		8,
		t * 0.55,
		1.0,
		lighten(brass, 0.08),
	);
	gear(w * 0.28, h * 0.38, h * 0.1, 7, -t * 0.62, 0.85, silver);
	gear(w * 0.62, h * 0.62, h * 0.09, 6, t * 0.7, 0.75, brass);
	draw_circle_lines(
		w * 0.5,
		h * 0.12,
		h * 0.06,
		0.6,
		Color::from_rgba(140, 135, 125, 90),
	);
}

// --- Section 1: vertical pipes + flanges (plumbing / foundry) ---

fn draw_section_pipes(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(18, 10, 8, 255);
	let bottom = Color::from_rgba(36, 20, 12, 255);
	draw_vertical_gradient(top, bottom, w, h);
	draw_rectangle(0.0, 0.0, w, h, Color::from_rgba(0, 0, 0, 40));

	let pipe_w = w / 7.0;
	let pipe_col = Color::from_rgba(90, 82, 78, 220);
	let flange = Color::from_rgba(130, 118, 108, 200);
	let glow = Color::from_rgba(200, 120, 60, 35);

	for col in 0..7 {
		let x0 = 12.0 + col as f32 * pipe_w;
		let pw = pipe_w * 0.42;
		let mut y = -20.0;
		for _ in 0..200 {
			if y >= h + 40.0 {
				break;
			}
			let phase = (y * 0.02 + t * 1.4 + col as f32 * 0.7).sin() * 0.5 + 0.5;
			let seg_h = 38.0 + phase * 14.0;
			draw_rectangle(x0, y, pw, seg_h, pipe_col);
			draw_rectangle_lines(x0, y, pw, seg_h, 1.2, darken(pipe_col, 0.25));
			draw_line(x0 - 2.0, y, x0 + pw + 2.0, y, 2.0, flange);
			draw_line(x0 - 2.0, y + seg_h, x0 + pw + 2.0, y + seg_h, 2.0, flange);
			y += (seg_h + 6.0).max(1.0);
		}
	}
	draw_rectangle(0.0, h * 0.35, w, h * 0.4, glow);
}

// --- Section 2: PCB traces + chip silhouettes ---

fn draw_section_pcb(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(8, 22, 14, 255);
	let bottom = Color::from_rgba(4, 12, 8, 255);
	draw_vertical_gradient(top, bottom, w, h);

	let trace = Color::from_rgba(40, 95, 55, 180);
	let via = Color::from_rgba(70, 140, 85, 200);
	let chip = Color::from_rgba(15, 18, 16, 230);
	let step = 22.0;
	let gx = (w / step).ceil() as i32;
	let gy = (h / step).ceil() as i32;

	for j in 0..gy {
		for i in 0..gx {
			let x = i as f32 * step;
			let y = j as f32 * step;
			let seed = (i * 17 + j * 31) as f32;
			if (seed + t * 0.3).sin() > 0.2 && i + 1 < gx {
				draw_line(
					x + step * 0.5,
					y + step * 0.5,
					x + step * 1.5,
					y + step * 0.5,
					1.2,
					trace,
				);
			}
			if (seed * 1.3 + t * 0.2).cos() > 0.15 && j + 1 < gy {
				draw_line(
					x + step * 0.5,
					y + step * 0.5,
					x + step * 0.5,
					y + step * 1.5,
					1.2,
					trace,
				);
			}
			draw_circle(x + step * 0.5, y + step * 0.5, 2.0, via);
		}
	}
	// Chip packages
	for k in 0..8 {
		let cx = w * (0.12 + (k as f32 * 0.11) % 0.76);
		let cy = h * (0.15 + ((k * 7) as f32 * 0.09) % 0.65);
		let rw = 28.0 + (k as f32 * 3.0) % 18.0;
		let rh = 18.0 + (k as f32 * 2.0) % 12.0;
		draw_rectangle(cx, cy, rw, rh, chip);
		draw_rectangle_lines(cx, cy, rw, rh, 1.0, Color::from_rgba(50, 55, 52, 255));
		let pulse = ((t * 2.0 + k as f32).sin() * 0.5 + 0.5) * 40.0;
		draw_rectangle(
			cx + 4.0,
			cy + 3.0,
			rw - 8.0,
			3.0,
			Color::from_rgba(80, 120, 90, pulse as u8),
		);
	}
}

// --- Section 3: hex lattice ---

fn draw_section_hex(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(6, 8, 22, 255);
	let bottom = Color::from_rgba(12, 14, 38, 255);
	draw_vertical_gradient(top, bottom, w, h);

	let col = Color::from_rgba(80, 95, 160, 120);
	let r = 18.0;
	let horiz = r * 1.732; // sqrt(3) * r
	let vert = r * 1.5;
	let mut row = 0_i32;
	let mut y = -r;
	for _row in 0..80 {
		if y >= h + r {
			break;
		}
		let offset = if row % 2 == 0 { 0.0 } else { horiz * 0.5 };
		let mut x = -horiz + offset;
		for _col in 0..64 {
			if x >= w + horiz {
				break;
			}
			let cx = x;
			let cy = y;
			for k in 0..6 {
				let a1 = std::f32::consts::TAU / 6.0 * k as f32 + t * 0.08;
				let a2 = std::f32::consts::TAU / 6.0 * (k + 1) as f32 + t * 0.08;
				draw_line(
					cx + a1.cos() * r,
					cy + a1.sin() * r,
					cx + a2.cos() * r,
					cy + a2.sin() * r,
					1.0,
					col,
				);
			}
			x += horiz;
		}
		y += vert;
		row += 1;
	}
}

// --- Section 4: heat / furnace (wavy glow bands) ---

fn draw_section_furnace(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(35, 12, 4, 255);
	let bottom = Color::from_rgba(12, 4, 2, 255);
	draw_vertical_gradient(top, bottom, w, h);

	for layer in 0..12 {
		let yb = h * (0.08 + layer as f32 * 0.075);
		let amp = 10.0 + layer as f32 * 2.0;
		let mut px = 0.0;
		let mut py = yb + (px * 0.02 + t * 1.5 + layer as f32).sin() * amp;
		let mut x = 4.0;
		while x <= w {
			let ny = yb + (x * 0.015 + t * 1.5 + layer as f32 * 0.4).sin() * amp;
			let a = (40 + layer * 12).min(220);
			draw_line(
				px,
				py,
				x,
				ny,
				2.2,
				Color::from_rgba(255, 140 + layer * 8, 40, a),
			);
			px = x;
			py = ny;
			x += 8.0;
		}
	}
	for k in 0..6 {
		let cx = w * (0.15 + (k as f32 * 0.17) % 0.7);
		let cy = h * (0.3 + (k as f32 * 0.11) % 0.5);
		let pulse = ((t * 3.0 + k as f32).sin() * 0.5 + 0.5) * 55.0;
		draw_circle(
			cx,
			cy,
			25.0 + k as f32 * 4.0,
			Color::from_rgba(255, 180, 60, pulse as u8),
		);
	}
}

// --- Section 5: stock-photo site parody (Shutterstock-style chrome + watermark) ---

/// Brand-adjacent orange (readable on white; not an official asset).
const SS_ORANGE: Color = Color::from_rgba(238, 48, 44, 255);

fn draw_shutterstock_aperture(cx: f32, cy: f32, r: f32, line: f32, col: Color, rot: f32) {
	draw_circle_lines(cx, cy, r, line, col);
	let blades = 6;
	let c = rot.cos();
	let s = rot.sin();
	let rp = |lx: f32, ly: f32| (cx + lx * c - ly * s, cy + lx * s + ly * c);
	for i in 0..blades {
		let a = std::f32::consts::TAU / blades as f32 * i as f32 - 0.12;
		let a2 = a + std::f32::consts::TAU / blades as f32 * 0.42;
		let (x0, y0) = rp(a.cos() * r * 0.25, a.sin() * r * 0.25);
		let (x1, y1) = rp(a2.cos() * r * 0.92, a2.sin() * r * 0.92);
		draw_line(x0, y0, x1, y1, line * 1.1, col);
	}
	draw_circle_lines(cx, cy, r * 0.22, line * 0.8, col);
}

fn draw_section_truss(w: f32, h: f32, t: f32) {
	// Generic “hero stock photo” wash: cool highlight → warm floor (site preview look).
	let sky = Color::from_rgba(210, 228, 245, 255);
	let horizon = Color::from_rgba(248, 242, 230, 255);
	let floor = Color::from_rgba(255, 236, 214, 255);
	// Breathing horizon (slow “camera” sway).
	let split = 0.58 + 0.018 * (t * 0.28).sin();
	let h_grad = h * split.clamp(0.48, 0.68);
	draw_vertical_gradient(sky, horizon, w, h_grad);
	draw_rectangle(0.0, h_grad, w, h - h_grad, floor);
	// Soft “lens” blobs — drift + pulse (cheap stock lighting cliché).
	for (i, &(fx, fy, rr, a)) in [
		(0.22_f32, 0.35_f32, 0.42_f32, 40_u8),
		(0.78_f32, 0.28_f32, 0.32_f32, 35_u8),
		(0.55_f32, 0.72_f32, 0.38_f32, 28_u8),
	]
	.iter()
	.enumerate()
	{
		let pulse = 0.85 + 0.15 * (t * 0.9 + i as f32 * 1.3).sin();
		let dx = (t * 0.41 + i as f32 * 2.1).sin() * 14.0;
		let dy = (t * 0.33 - i as f32 * 1.4).cos() * 11.0;
		draw_circle(
			w * fx + dx,
			h * fy + dy,
			h * rr * pulse,
			Color::from_rgba(255, 255, 255, a),
		);
	}
	// Thumbnail grid — idle scroll + per-tile shimmer (muted “search results”).
	let grid_top = h * 0.12;
	let grid_h = h * 0.78;
	let cols = 6_u32;
	let rows = 4_u32;
	let row_pitch = grid_h / rows as f32 * 0.82;
	let scroll = (t * 9.0).rem_euclid(row_pitch.max(1.0));
	for j in 0..rows {
		for i in 0..cols {
			let seed = (i * 17 + j * 31) as f32;
			let gx = w * (i as f32 + 0.04) / cols as f32
				+ (t * 11.0 + seed).sin() * 1.8;
			let gy = grid_top + grid_h * (j as f32 + 0.06) / rows as f32 - scroll
				+ (t * 7.0 + seed * 0.5).cos() * 2.0;
			let gw = w / cols as f32 * 0.88;
			let gh = grid_h / rows as f32 * 0.82;
			let tint = 0.55
				+ 0.45 * (seed * 0.7 + t * 0.35).sin() * 0.5 + 0.5;
			let r = (180.0 + tint * 55.0) as u8;
			let g = (200.0 + (1.0 - tint) * 40.0) as u8;
			let b = (210.0 + seed.sin() * 25.0) as u8;
			draw_rectangle(gx, gy, gw, gh, Color::from_rgba(r, g, b, 115));
			draw_rectangle_lines(gx, gy, gw, gh, 1.0, Color::from_rgba(255, 255, 255, 45));
		}
	}

	// Top site chrome + Shutterstock-style wordmark + aperture mark (user-requested logo).
	let bar_h = (h * 0.085).max(36.0).min(72.0);
	draw_rectangle(0.0, 0.0, w, bar_h, Color::from_rgba(255, 255, 255, 255));
	draw_line(0.0, bar_h, w, bar_h, 1.5, Color::from_rgba(230, 232, 235, 255));
	let icon_r = bar_h * 0.32;
	let icx = bar_h * 0.55;
	let icy = bar_h * 0.5;
	let icon_spin = (t * 0.42).sin() * 0.18;
	draw_shutterstock_aperture(icx, icy, icon_r, 2.0, SS_ORANGE, icon_spin);
	let word_x = icx + icon_r + 14.0;
	let word_y = bar_h * 0.72;
	let word_pulse = 0.92 + 0.08 * (t * 1.6).sin();
	let orange_live = Color::new(
		SS_ORANGE.r * word_pulse,
		SS_ORANGE.g * word_pulse,
		SS_ORANGE.b * word_pulse,
		1.0,
	);
	draw_text("shutterstock", word_x, word_y, 26.0, orange_live);
	draw_text(".com", word_x + 200.0, word_y, 14.0, Color::from_rgba(140, 145, 152, 255));
	// Fake search field on the right (starts past wordmark so layouts don’t overlap).
	let sx = (word_x + 205.0).max(w * 0.52);
	let sw = (w - sx - 18.0).max(32.0);
	let sy = bar_h * 0.22;
	let sh = bar_h * 0.56;
	draw_rectangle(sx, sy, sw, sh, Color::from_rgba(245, 246, 248, 255));
	draw_rectangle_lines(sx, sy, sw, sh, 1.0, Color::from_rgba(210, 215, 222, 255));
	draw_text(
		"stock photos, vectors, video…",
		sx + 10.0,
		bar_h * 0.62,
		14.0,
		Color::from_rgba(120, 128, 138, 255),
	);
	// Specular sweep + blinking caret (site UI energy).
	let sweep_x = sx + 12.0 + (t * 38.0).rem_euclid((sw - 28.0).max(8.0));
	let sweep_a = (55.0 + (t * 6.0).sin() * 22.0).clamp(28.0, 95.0).round() as u8;
	draw_rectangle(
		sweep_x,
		sy + 4.0,
		10.0,
		sh - 8.0,
		Color::from_rgba(255, 255, 255, sweep_a),
	);
	let caret_on = (t * 2.8).rem_euclid(1.0) < 0.52;
	if caret_on {
		draw_line(
			sx + sw - 14.0,
			sy + 6.0,
			sx + sw - 14.0,
			sy + sh - 6.0,
			1.4,
			Color::from_rgba(238, 48, 44, 220),
		);
	}

	// Diagonal watermark — readable but not dominating (soft outline + fill).
	let wm = "shutterstock";
	let wm_alpha = (52.0 + 14.0 * (t * 1.9).sin()).clamp(38.0, 72.0).round() as u8;
	let wm_col = Color::from_rgba(255, 255, 255, wm_alpha);
	let outline_a = (wm_alpha as f32 * 0.45).min(120.0).round() as u8;
	let outline = Color::from_rgba(28, 30, 38, outline_a);
	let wm_size = 34_u16;
	let pitch_x = 295.0_f32;
	let pitch_y = 100.0_f32;
	let drift_x = (t * 22.0).rem_euclid(pitch_x);
	let drift_y = (t * 7.5).sin() * 10.0;
	let wm_rot = -0.52 + 0.055 * (t * 0.85).sin();
	let mut row = 0_i32;
	let mut wy = -pitch_y * 2.0 + drift_y;
	while wy < h + pitch_y * 2.0 {
		let stagger = if row % 2 == 0 { 0.0 } else { pitch_x * 0.5 };
		let row_sway = (t * 0.65 + row as f32 * 0.4).sin() * 6.0;
		let mut wx = -pitch_x * 2.0 + drift_x + stagger + row_sway;
		while wx < w + pitch_x * 2.0 {
			let ox = 2.0_f32;
			let oy = 2.0_f32;
			let params = TextParams {
				font_size: wm_size,
				rotation: wm_rot,
				..Default::default()
			};
			// Light faux outline (corners only — fewer passes than “max obvious” mode).
			for (dx, dy) in [(ox, oy), (-ox, oy), (ox, -oy), (-ox, -oy)] {
				draw_text_ex(
					wm,
					wx + dx,
					wy + dy,
					TextParams {
						color: outline,
						..params
					},
				);
			}
			draw_text_ex(
				wm,
				wx,
				wy,
				TextParams {
					color: wm_col,
					..params
				},
			);
			wx += pitch_x;
		}
		wy += pitch_y;
		row += 1;
	}

	// Bottom-right tiny logo — slow rotation (stock sites love corner marks).
	let corner = 12.0;
	let cx = w - corner - 26.0;
	let cy = h - corner - 26.0;
	draw_shutterstock_aperture(cx, cy, 18.0, 1.4, SS_ORANGE, t * 0.19);
	draw_text(
		"shutterstock",
		w - 168.0,
		h - corner - 12.0,
		12.0,
		Color::from_rgba(
			238,
			48,
			44,
			(180.0 + (t * 3.0).sin() * 40.0).clamp(0.0, 255.0).round() as u8,
		),
	);

	// Vignette to keep playfield readable.
	draw_rectangle(0.0, 0.0, w * 0.12, h, Color::from_rgba(0, 0, 0, 22));
	draw_rectangle(w * 0.88, 0.0, w * 0.12, h, Color::from_rgba(0, 0, 0, 22));
	draw_rectangle(0.0, 0.0, w, h * 0.06, Color::from_rgba(0, 0, 0, 18));
	draw_rectangle(0.0, h * 0.94, w, h * 0.06, Color::from_rgba(0, 0, 0, 28));
}

// --- Section 6: matrix-style falling columns ---

fn draw_section_matrix(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(0, 8, 4, 255);
	let bottom = Color::from_rgba(0, 18, 10, 255);
	draw_vertical_gradient(top, bottom, w, h);
	draw_rectangle(0.0, 0.0, w, h, Color::from_rgba(0, 25, 12, 80));

	let cols = 48;
	for i in 0..cols {
		let x = w * (i as f32 + 0.5) / cols as f32;
		let speed = 40.0 + (i % 7) as f32 * 25.0;
		let head = (t * speed + i as f32 * 13.0) % (h + 120.0) - 60.0;
		let seg_len = 12.0 + (i % 5) as f32 * 8.0;
		let mut y = head;
		let g = (120 + (i * 3) % 100) as u8;
		let dy = (seg_len + 5.0).max(1.0);
		for _ in 0..120 {
			if y >= h + 40.0 {
				break;
			}
			draw_line(x, y, x, y + seg_len, 1.6, Color::from_rgba(40, g, 70, 220));
			draw_circle(
				x,
				y + seg_len * 0.5,
				1.5,
				Color::from_rgba(180, 255, 200, 180),
			);
			y += dy;
		}
	}
}

// --- Section 7: starfield + nebula drift ---

fn draw_section_void(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(2, 4, 18, 255);
	let bottom = Color::from_rgba(4, 6, 28, 255);
	draw_vertical_gradient(top, bottom, w, h);

	let neb = Color::from_rgba(40, 50, 120, 25);
	draw_circle(w * 0.35 + (t * 8.0).sin() * 20.0, h * 0.35, h * 0.35, neb);
	draw_circle(
		w * 0.7 + (t * 6.0).cos() * 15.0,
		h * 0.6,
		h * 0.25,
		Color::from_rgba(60, 30, 80, 20),
	);

	for i in 0..140 {
		let sx = (i * 73 % 1000) as f32 / 1000.0 * w;
		let sy = (i * 41 % 1000) as f32 / 1000.0 * h;
		let drift_x = (t * (0.3 + (i % 5) as f32 * 0.1) + i as f32 * 0.1).sin() * 3.0;
		let drift_y = (t * 0.25 + i as f32 * 0.08).cos() * 2.0;
		let br = 0.3 + (i % 7) as f32 * 0.35;
		let a = (80 + (i * 17) % 120) as u8;
		draw_circle(
			sx + drift_x,
			sy + drift_y,
			br,
			Color::from_rgba(220, 230, 255, a),
		);
	}
}

// --- Section 8: tissue field + cinematic DNA-style helices (800–899) ---

/// 2D projection of a double helix: two backbones (phase-opposed) + rungs, with drift and spin.
fn draw_dna_helix_2d(
	_w: f32,
	_h: f32,
	t: f32,
	x0: f32,
	y0: f32,
	x1: f32,
	y1: f32,
	turns: f32,
	radius: f32,
	phase: f32,
	spin: f32,
	seg: usize,
	line_w: f32,
	base_alpha: u8,
	blurred: bool,
) {
	let seg = seg.max(8);
	let n = seg - 1;
	let pulse = 0.72 + 0.28 * (t * 2.4 + phase * 0.7).sin().abs();
	let drift_x = (t * spin * 0.18 + phase).sin() * (if blurred { 22.0 } else { 14.0 });
	let drift_y = (t * spin * 0.15 + phase * 1.3).cos() * (if blurred { 16.0 } else { 11.0 });
	let ax = x1 - x0;
	let ay = y1 - y0;
	let mut px_prev_a = 0.0_f32;
	let mut py_prev_a = 0.0_f32;
	let mut px_prev_b = 0.0_f32;
	let mut py_prev_b = 0.0_f32;
	let mut first = true;

	let center = |si: f32| {
		let cx = x0 + ax * si + drift_x * (si - 0.5) * 2.0;
		let cy = y0 + ay * si + drift_y * (si - 0.3);
		(cx, cy)
	};

	for i in 0..=n {
		let s = i as f32 / n as f32;
		let (cx, cy) = center(s);
		let (tx, ty) = if i < n {
			let (cxn, cyn) = center((i + 1) as f32 / n as f32);
			(cxn - cx, cyn - cy)
		} else if i > 0 {
			let (cxp, cyp) = center((i - 1) as f32 / n as f32);
			(cx - cxp, cy - cyp)
		} else {
			(ax, ay)
		};
		let tlen = (tx * tx + ty * ty).sqrt().max(0.001);
		let nx = -ty / tlen;
		let ny = tx / tlen;
		let ang = s * turns * std::f32::consts::TAU + t * spin + phase;
		let breathe = radius * (1.0 + 0.1 * (t * 3.2 + s * 24.0).sin());
		let r = breathe * 0.52;
		let c = ang.cos();
		let px_a = cx + nx * r * c;
		let py_a = cy + ny * r * c;
		let px_b = cx - nx * r * c;
		let py_b = cy - ny * r * c;

		let a_line = ((base_alpha as f32) * pulse * if blurred { 0.45 } else { 1.0 }) as u8;
		let a_bead = ((base_alpha as f32 + 40.0) * pulse * if blurred { 0.4 } else { 1.0 }) as u8;
		let cyan = Color::from_rgba(30, 230, 255, a_line);
		let cyan_bright = Color::from_rgba(160, 250, 255, a_bead);
		let rung = Color::from_rgba(60, 200, 240, (a_line as i32 * 3 / 4).max(0) as u8);

		if !first {
			draw_line(px_prev_a, py_prev_a, px_a, py_a, line_w, cyan);
			draw_line(px_prev_b, py_prev_b, px_b, py_b, line_w, cyan);
		}
		if i % 2 == 0 {
			draw_line(px_a, py_a, px_b, py_b, line_w * 0.65, rung);
		}
		if i % 3 == 0 && !blurred {
			let br = 2.0 + (s * 7.0 + t).sin() * 0.6;
			draw_circle(px_a, py_a, br, cyan_bright);
			draw_circle(px_b, py_b, br, cyan_bright);
		}
		px_prev_a = px_a;
		py_prev_a = py_a;
		px_prev_b = px_b;
		py_prev_b = py_b;
		first = false;
	}
	// Soft halo for depth (fake bokeh on blurred helices)
	if blurred {
		let mid_x = x0 + ax * 0.5;
		let mid_y = y0 + ay * 0.5;
		let hh = (y1 - y0).abs().max(x1 - x0).abs();
		draw_circle(
			mid_x,
			mid_y,
			radius * 6.0 + hh * 0.08,
			Color::from_rgba(20, 100, 160, 18),
		);
	}
}

fn draw_section_tissue(w: f32, h: f32, t: f32) {
	// Dark lab / microscope void so cyan helices read.
	let top = Color::from_rgba(4, 8, 22, 255);
	let bottom = Color::from_rgba(14, 28, 52, 255);
	draw_vertical_gradient(top, bottom, w, h);
	// Volumetric-ish fan from a focal point (slow rotation + pulse).
	let focus_x = w * 0.48 + (t * 0.35).sin() * 24.0;
	let focus_y = h * 0.38 + (t * 0.28).cos() * 18.0;
	let ray_pulse = 10.0 + 8.0 * (t * 1.9).sin();
	for r in 0..28 {
		let ang = -0.55 + r as f32 * 0.09 + t * 0.11;
		let len = w.max(h) * 1.35;
		draw_line(
			focus_x,
			focus_y,
			focus_x + ang.cos() * len,
			focus_y + ang.sin() * len,
			2.2,
			Color::from_rgba(20, 90, 140, (ray_pulse as u8).min(22)),
		);
	}

	// Background helix: large, slow, defocused.
	draw_dna_helix_2d(
		w,
		h,
		t,
		w * 1.02,
		-h * 0.08,
		-w * 0.12,
		h * 0.72,
		5.5,
		h * 0.11,
		2.1,
		0.42,
		40,
		2.8,
		55,
		true,
	);
	draw_dna_helix_2d(
		w,
		h,
		t + 1.7,
		w * 0.88,
		h * 0.15,
		-w * 0.02,
		h * 0.98,
		4.0,
		h * 0.07,
		4.5,
		0.55,
		32,
		2.0,
		40,
		true,
	);

	// Soft clinical highlight (ties to tissue slide).
	draw_circle(
		-w * 0.06,
		-h * 0.04,
		h * 0.38,
		Color::from_rgba(200, 220, 245, 28),
	);

	// Parallax “cells” — faster drift + stronger pulse so the field feels alive.
	let layers: [(f32, f32, u8, f32); 3] = [
		(13.0, 1.35, 55, 1.0),
		(19.0, 0.95, 68, 1.45),
		(27.0, 0.65, 78, 1.9),
	];
	for (step, speed, base_a, r_scale) in layers {
		let mut y = -step;
		let mut row = 0_u32;
		while y < h + step {
			let x_off = if row % 2 == 1 { step * 0.5 } else { 0.0 };
			let mut x = -step + x_off;
			while x < w + step {
				let flow = t * speed;
				let jitter = (x * 0.024 + y * 0.019 + flow * 0.62).sin() * 6.0;
				let jitter2 = (x * 0.016 - y * 0.021 + flow * 0.52).cos() * 5.0;
				let px = x + jitter;
				let py = y + jitter2;
				let pulse = ((flow * 1.1 + x * 0.012 + y * 0.014).sin() * 0.5 + 0.5) * 55.0;
				let a = ((base_a as f32 + pulse) as u8).min(200);
				let r = r_scale * (0.82 + (x * 0.04 + y * 0.033 + flow * 0.4).sin() * 0.22);
				draw_circle(px, py, r, Color::from_rgba(70, 150, 210, a));
				x += step;
			}
			y += step * 0.866;
			row += 1;
		}
	}

	// Golden-brown membrane mesh (top-left).
	const GW: usize = 10;
	const GH: usize = 8;
	for pass in 0..2 {
		let t_off = t * 1.15 + pass as f32 * 1.7;
		let alpha_line = if pass == 0 { 105_u8 } else { 65 };
		let alpha_node = if pass == 0 { 160_u8 } else { 100 };
		let mut pts = [[(0.0_f32, 0.0_f32); GW]; GH];
		for j in 0..GH {
			for i in 0..GW {
				let ox = (t_off * 0.52 + i as f32 * 0.73).sin() * 6.0
					+ (t_off * 0.31 + j as f32 * 0.5).cos() * 3.5;
				let oy = (t_off * 0.48 + j as f32 * 0.68).cos() * 5.5
					+ (t_off * 0.33 + i as f32 * 0.44).sin() * 3.0;
				let px = w * (0.02 + i as f32 / (GW - 1) as f32 * 0.48) + ox + pass as f32 * 6.0;
				let py = h * (0.02 + j as f32 / (GH - 1) as f32 * 0.46) + oy + pass as f32 * 4.0;
				pts[j][i] = (px, py);
			}
		}
		let brown = Color::from_rgba(155, 92, 38, alpha_line);
		let brown_soft = Color::from_rgba(175, 115, 55, alpha_line / 2);
		for j in 0..GH {
			for i in 0..GW {
				let (a, b) = pts[j][i];
				if i + 1 < GW {
					let (c, d) = pts[j][i + 1];
					draw_line(a, b, c, d, 1.4, brown);
				}
				if j + 1 < GH {
					let (c, d) = pts[j + 1][i];
					draw_line(a, b, c, d, 1.4, brown);
				}
				if i + 1 < GW && j + 1 < GH {
					let (c, d) = pts[j + 1][i + 1];
					if (i + j + pass as usize).is_multiple_of(2) {
						draw_line(a, b, c, d, 1.0, brown_soft);
					}
				}
				draw_circle(a, b, 2.2, Color::from_rgba(175, 110, 48, alpha_node));
			}
		}
	}

	// Foreground helix: sharp, dominant diagonal (endpoint past bottom-right so the strand
	// doesn’t visibly stop short of the corner; mirrors the start overshoot past top-left).
	let hx0 = -w * 0.06;
	let hy0 = -h * 0.04;
	let hx1 = w * 1.06;
	let hy1 = h * 1.04;
	let ax = hx1 - hx0;
	let ay = hy1 - hy0;
	let path_len = (ax * ax + ay * ay).sqrt();
	// Match previous (0.92, 0.92) endpoint twist density.
	let path_len_was = {
		let ax0 = w * 0.92 - hx0;
		let ay0 = h * 0.92 - hy0;
		(ax0 * ax0 + ay0 * ay0).sqrt()
	};
	let turns = 7.0 * (path_len / path_len_was);
	draw_dna_helix_2d(
		w,
		h,
		t,
		hx0,
		hy0,
		hx1,
		hy1,
		turns,
		h * 0.065,
		0.0,
		0.95,
		64,
		2.2,
		165,
		false,
	);
}

// --- Section 9: golden radial finale (900–999) ---

fn draw_section_finale(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(8, 6, 2, 255);
	let bottom = Color::from_rgba(35, 28, 8, 255);
	draw_vertical_gradient(top, bottom, w, h);
	draw_rectangle(0.0, 0.0, w, h, Color::from_rgba(60, 45, 0, 35));

	let cx = w * 0.5;
	let cy = h * 0.42;
	let rays = 48;
	for i in 0..rays {
		let ang = std::f32::consts::TAU / rays as f32 * i as f32 + t * 0.35;
		let len = h * 0.95;
		let gold = Color::from_rgba(255, 210, 80, (40 + (i % 5) * 8) as u8);
		draw_line(
			cx,
			cy,
			cx + ang.cos() * len,
			cy + ang.sin() * len,
			2.5,
			gold,
		);
	}
	for ring in 1..6 {
		let rr = ring as f32 * 38.0 + (t * 20.0).sin() * 4.0;
		draw_circle_lines(
			cx,
			cy,
			rr,
			1.2,
			Color::from_rgba(255, 220, 120, 100 - ring * 12),
		);
	}
	draw_circle(cx, cy, 45.0, Color::from_rgba(255, 240, 180, 50));
	draw_circle_lines(cx, cy, 28.0, 2.0, Color::from_rgba(255, 255, 220, 200));
}

// --- Title: orange-lit wall, wainscot, brick texture, subtle tech overlay ---

fn draw_bg_title(w: f32, h: f32, t: f32) {
	let wain_top = h * 0.68;
	let top = Color::from_rgba(110, 32, 6, 255);
	let mid = Color::from_rgba(255, 135, 40, 255);
	draw_vertical_gradient(top, mid, w, wain_top);
	draw_rectangle(
		0.0,
		wain_top,
		w,
		h - wain_top + 1.0,
		Color::from_rgba(5, 3, 5, 255),
	);
	draw_rectangle(0.0, wain_top, w, 2.0, Color::from_rgba(255, 155, 65, 60));

	let bloom = 0.82 + 0.18 * (t * 1.1).sin();
	draw_circle(
		w * 0.9,
		-h * 0.1,
		h * 0.92,
		Color::from_rgba(255, 205, 115, (88.0 * bloom) as u8),
	);
	draw_circle(
		w * 0.52,
		h * 0.1,
		h * 0.5,
		Color::from_rgba(255, 150, 55, (35.0 * bloom) as u8),
	);

	for row in 0..14 {
		let yy = 12.0 + row as f32 * (wain_top - 18.0) / 14.0;
		let shift = (row % 3) as f32 * 7.0;
		draw_line(0.0, yy, w, yy, 1.0, Color::from_rgba(55, 20, 8, 24));
		let mut x = -shift + (t * 18.0).rem_euclid(24.0);
		while x < w + 24.0 {
			draw_line(x, yy, x, yy + 11.0, 1.0, Color::from_rgba(42, 14, 5, 20));
			x += 24.0;
		}
	}

	for g in 0..18 {
		let gx = g as f32 / 17.0;
		let yy = 20.0 + gx * (wain_top - 40.0);
		let sc = 0.35 + 0.65 * gx;
		draw_line(
			w * 0.08,
			yy,
			w * 0.92,
			yy,
			1.0,
			Color::from_rgba(0, 210, 255, (10.0 * sc) as u8),
		);
	}
	for g in 0..12 {
		let xx = w * (0.1 + g as f32 / 11.0 * 0.8);
		draw_line(
			xx,
			14.0,
			xx,
			wain_top - 10.0,
			1.0,
			Color::from_rgba(255, 195, 95, 9),
		);
	}

	let scan_y = (t * 42.0).rem_euclid(wain_top + 40.0) - 20.0;
	draw_rectangle(0.0, scan_y, w, 12.0, Color::from_rgba(255, 245, 210, 16));

	for m in 0..36 {
		let bx = ((m * 7919) % 1000) as f32 / 1000.0 * w;
		let by = ((m * 5347) % 1000) as f32 / 1000.0 * wain_top * 0.95;
		let dx = (t * 0.5 + m as f32 * 0.6).sin() * 10.0;
		let dy = (t * 0.4 + m as f32 * 0.45).cos() * 8.0;
		let a = (20 + (m * 13) % 55) as u8;
		draw_circle(
			bx + dx,
			by + dy,
			0.9 + (m % 4) as f32 * 0.45,
			Color::from_rgba(255, 220, 160, a),
		);
	}
}

// --- Replay list: film / data tape ---

fn draw_bg_replay(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(4, 16, 22, 255);
	let bottom = Color::from_rgba(6, 28, 32, 255);
	draw_vertical_gradient(top, bottom, w, h);

	let track = Color::from_rgba(30, 70, 78, 200);
	for band in 0..6 {
		let yy = h * (0.12 + band as f32 * 0.14);
		draw_rectangle(0.0, yy, w, 22.0, track);
		draw_rectangle_lines(0.0, yy, w, 22.0, 1.0, Color::from_rgba(50, 110, 120, 255));
		let mut x = (t * 35.0 + band as f32 * 17.0) % 40.0 - 40.0;
		for _ in 0..200 {
			if x >= w + 40.0 {
				break;
			}
			draw_line(
				x,
				yy + 4.0,
				x,
				yy + 18.0,
				2.5,
				Color::from_rgba(20, 45, 50, 255),
			);
			x += 14.0;
		}
	}
	draw_circle_lines(
		w * 0.08,
		h * 0.5,
		h * 0.18,
		3.0,
		Color::from_rgba(70, 120, 130, 150),
	);
	draw_circle_lines(
		w * 0.92,
		h * 0.5,
		h * 0.18,
		3.0,
		Color::from_rgba(70, 120, 130, 150),
	);
	for k in 0..20 {
		let a = k as f32 * 0.31 + t * 0.5;
		draw_line(
			w * 0.08 + a.cos() * h * 0.16,
			h * 0.5 + a.sin() * h * 0.16,
			w * 0.08 + (a + 0.15).cos() * h * 0.16,
			h * 0.5 + (a + 0.15).sin() * h * 0.16,
			2.0,
			Color::from_rgba(90, 150, 160, 100),
		);
	}
}

/// `phase` is integrated animation time: advance each frame with
/// `phase += dt * bg.section_speed() * stack_speed_mult` so changing the stack multiplier does not
/// jump the phase (unlike multiplying wall time by a varying factor).
/// `beat_pulse` is 0..1 from BGM sync — nudges phase and brightens the backdrop on each beat.
pub fn draw_clockwork_background(
	w: f32,
	h: f32,
	phase: f32,
	bg: ClockworkBackground,
	beat_pulse: f32,
) {
	let (w, h) = sanitize_wh(w, h);
	let bp = beat_pulse.clamp(0.0, 1.0);
	let t0 = if phase.is_finite() { phase } else { 0.0 };
	// Tiny phase nudge on the beat (keep subtle — large values read as jitter).
	let t = t0 + bp * 0.09;

	match bg {
		ClockworkBackground::Section0 => draw_section_gears(w, h, t),
		ClockworkBackground::Section1 => draw_section_pipes(w, h, t),
		ClockworkBackground::Section2 => draw_section_pcb(w, h, t),
		ClockworkBackground::Section3 => draw_section_hex(w, h, t),
		ClockworkBackground::Section4 => draw_section_furnace(w, h, t),
		ClockworkBackground::Section5 => draw_section_truss(w, h, t),
		ClockworkBackground::Section6 => draw_section_matrix(w, h, t),
		ClockworkBackground::Section7 => draw_section_void(w, h, t),
		ClockworkBackground::Section8 => draw_section_tissue(w, h, t),
		ClockworkBackground::Section9 => draw_section_finale(w, h, t),
		ClockworkBackground::Title => draw_bg_title(w, h, t),
		ClockworkBackground::ReplayList => draw_bg_replay(w, h, t),
	}
	// Slight warm lift; beat adds a few alpha steps only.
	let lift = (BG_SCREEN_LIFT_ALPHA as f32 + bp * 10.0).min(58.0).round() as u8;
	draw_rectangle(
		0.0,
		0.0,
		w,
		h,
		Color::from_rgba(255, 252, 248, lift),
	);
	let flash = (bp * 12.0).min(20.0).round() as u8;
	if flash > 0 {
		draw_rectangle(
			0.0,
			0.0,
			w,
			h,
			Color::from_rgba(235, 248, 255, flash),
		);
	}
}
