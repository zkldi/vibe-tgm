//! Distinct procedural backgrounds (vector-only): each gameplay section uses a different scene, not
//! a palette swap.

use std::f32::consts::SQRT_2;

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

/// Which backdrop to draw: nine gameplay sections (100-level bands), title, replay list.
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
	Title,
	ReplayList,
}

impl ClockworkBackground {
	pub fn from_level(level: u16) -> Self {
		match (level / 100).min(8) {
			0 => Self::Section0,
			1 => Self::Section1,
			2 => Self::Section2,
			3 => Self::Section3,
			4 => Self::Section4,
			5 => Self::Section5,
			6 => Self::Section6,
			7 => Self::Section7,
			_ => Self::Section8,
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
			Self::Section5 => "Section 5 (500-599) — Truss",
			Self::Section6 => "Section 6 (600-699) — Matrix",
			Self::Section7 => "Section 7 (700-799) — Void",
			Self::Section8 => "Section 8 (800+) — Finale",
			Self::Title => "Title",
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
			Self::Section7 => 1.4,
			Self::Section8 => 1.48,
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
			if (seed + t * 0.3).sin() > 0.2 {
				if i + 1 < gx {
					draw_line(
						x + step * 0.5,
						y + step * 0.5,
						x + step * 1.5,
						y + step * 0.5,
						1.2,
						trace,
					);
				}
			}
			if (seed * 1.3 + t * 0.2).cos() > 0.15 {
				if j + 1 < gy {
					draw_line(
						x + step * 0.5,
						y + step * 0.5,
						x + step * 0.5,
						y + step * 1.5,
						1.2,
						trace,
					);
				}
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
			let a = (40 + layer * 12).min(220) as u8;
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

// --- Section 5: structural truss + hazard stripes ---

fn draw_section_truss(w: f32, h: f32, t: f32) {
	let top = Color::from_rgba(12, 10, 8, 255);
	let bottom = Color::from_rgba(28, 26, 22, 255);
	draw_vertical_gradient(top, bottom, w, h);

	// Muted structural grid so gameplay stays readable.
	let beam = Color::from_rgba(72, 70, 66, 95);
	let hazard = Color::from_rgba(95, 82, 38, 52);
	// Vertical columns
	for i in 0..5 {
		let x = w * (0.1 + i as f32 * 0.2);
		draw_line(x, 0.0, x, h, 2.8, beam);
		draw_line(x + 2.0, 0.0, x + 2.0, h, 1.4, darken(beam, 0.3));
	}
	// Fixed pitch so horizontal beams tile predictably (no moiré beat with diagonals).
	let step_y = 22.0_f32;
	let n_h = ((h / step_y).ceil() as usize).min(100).max(1);
	for i in 0..n_h {
		let y = i as f32 * step_y;
		if y > h {
			break;
		}
		draw_line(0.0, y, w, y, 2.2, beam);
	}
	// Diagonal hazard: parallel 45° stripes, perpendicular spacing `d_perp`; scroll wraps
	// seamlessly.
	let stripe = 8.0;
	let d_perp = 26.0;
	let dx = d_perp * SQRT_2;
	let scroll = t * 22.0;
	let phase = if scroll.is_finite() {
		scroll.rem_euclid(dx)
	} else {
		0.0
	};
	let mut x0 = -h - dx;
	let x_end = w + h + dx;
	while x0 < x_end {
		let x = x0 + phase;
		draw_line(x, 0.0, x + h, h, stripe, hazard);
		x0 += dx;
	}
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

// --- Section 8: golden radial finale ---

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

// --- Title: aurora bands (no gears) ---

fn draw_bg_title(w: f32, h: f32, t: f32) {
	for i in 0..32 {
		let y0 = h * i as f32 / 32.0;
		let y1 = h * (i + 1) as f32 / 32.0;
		let u = i as f32 / 31.0;
		let shift = (t * 0.4 + u * 3.0).sin() * 0.08;
		let c1 = Color::from_rgba(8, 12, 40, 255);
		let c2 = Color::from_rgba(20, 16, 55, 255);
		let c3 = Color::from_rgba(14, 22, 48, 255);
		let mix = lerp_color(lerp_color(c1, c2, u + shift), c3, (t + u).sin() * 0.5 + 0.5);
		draw_rectangle(0.0, y0, w, y1 - y0 + 0.5, mix);
	}
	for i in 0..5 {
		let yy = h * (0.2 + i as f32 * 0.12) + (t * 15.0 + i as f32).sin() * 6.0;
		draw_rectangle(
			0.0,
			yy,
			w,
			18.0,
			Color::from_rgba(80, 60, 160, (15 + i * 5) as u8),
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
pub fn draw_clockwork_background(w: f32, h: f32, phase: f32, bg: ClockworkBackground) {
	let (w, h) = sanitize_wh(w, h);
	let t = if phase.is_finite() { phase } else { 0.0 };

	match bg {
		ClockworkBackground::Section0 => draw_section_gears(w, h, t),
		ClockworkBackground::Section1 => draw_section_pipes(w, h, t),
		ClockworkBackground::Section2 => draw_section_pcb(w, h, t),
		ClockworkBackground::Section3 => draw_section_hex(w, h, t),
		ClockworkBackground::Section4 => draw_section_furnace(w, h, t),
		ClockworkBackground::Section5 => draw_section_truss(w, h, t),
		ClockworkBackground::Section6 => draw_section_matrix(w, h, t),
		ClockworkBackground::Section7 => draw_section_void(w, h, t),
		ClockworkBackground::Section8 => draw_section_finale(w, h, t),
		ClockworkBackground::Title => draw_bg_title(w, h, t),
		ClockworkBackground::ReplayList => draw_bg_replay(w, h, t),
	}
	// Slight warm lift so backdrops read brighter without re-tuning every gradient.
	draw_rectangle(
		0.0,
		0.0,
		w,
		h,
		Color::from_rgba(255, 252, 248, BG_SCREEN_LIFT_ALPHA),
	);
}
