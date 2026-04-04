//! Boot attract mode: title overlay and taglines on top of the demo playfield.

use macroquad::prelude::*;

use crate::theme::ArcadeFont;

/// Rotating lines under the title (seconds per line).
pub const TAGLINE_ROTATE_SEC: f32 = 4.5;

const TAGLINES: [&str; 5] = [
	"20G GRAVITY — LOCK DELAY — GRADE",
	"AUTOPLAY DEMO — SAME RULES AS PLAY",
	"WASD MOVE  ·  J/K/L ROTATE",
	"SECTIONS 100–900 + GM CHASE",
	"PRESS ANY KEY TO PLAY",
];

/// Exit attract when a key is newly pressed; skips F3/F11/F8 and Alt+Enter (fullscreen).
pub fn attract_key_requests_exit(
	prev: &std::collections::HashSet<KeyCode>,
	down: &std::collections::HashSet<KeyCode>,
) -> bool {
	for &k in down {
		if !prev.contains(&k) {
			if matches!(k, KeyCode::F3 | KeyCode::F11 | KeyCode::F8) {
				continue;
			}
			if k == KeyCode::Enter
				&& (is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt))
			{
				continue;
			}
			return true;
		}
	}
	false
}

/// Logo, tagline, scan of “PRESS ANY KEY”, top vignette for readability.
pub fn draw_attract_overlay(font: &ArcadeFont, time_sec: f32, design_w: f32, design_h: f32) {
	let vignette_h = 168.0_f32;
	draw_rectangle(
		0.0,
		0.0,
		design_w,
		vignette_h,
		Color::from_rgba(0, 0, 0, 115),
	);

	let pulse = (time_sec * 1.1).sin() * 0.5 + 0.5;
	let title_y = design_h * 0.06 + pulse * 4.0;
	let title = "VIBE CODED TGM";
	let title_sz = 22.0 + pulse * 1.5;
	let tw = font.measure(title, title_sz).width;
	font.draw(
		title,
		(design_w - tw) * 0.5,
		title_y,
		title_sz,
		crate::theme::TITLE_LINE,
	);

	let tag_i = ((time_sec / TAGLINE_ROTATE_SEC) as usize) % TAGLINES.len();
	let tag = TAGLINES[tag_i];
	let tag_sz = 12.0;
	let tgw = font.measure(tag, tag_sz).width;
	let tag_alpha = ((time_sec / TAGLINE_ROTATE_SEC).fract() * std::f32::consts::TAU)
		.sin()
		.mul_add(0.5, 0.5);
	let tag_col = Color::new(
		crate::theme::TEXT_MUTED.r,
		crate::theme::TEXT_MUTED.g,
		crate::theme::TEXT_MUTED.b,
		0.55 + 0.45 * tag_alpha,
	);
	font.draw(tag, (design_w - tgw) * 0.5, title_y + 34.0, tag_sz, tag_col);

	let hint = "PRESS ANY KEY";
	let hint_sz = 14.0;
	let blink = ((time_sec * 2.8).sin() * 0.5 + 0.5) * 0.35 + 0.45;
	let hw = font.measure(hint, hint_sz).width;
	font.draw(
		hint,
		(design_w - hw) * 0.5,
		design_h * 0.88,
		hint_sz,
		Color::new(1.0, 1.0, 1.0, blink),
	);
	let sub = "MOUSE CLICK";
	let sub_sz = 10.0;
	let sw = font.measure(sub, sub_sz).width;
	font.draw(
		sub,
		(design_w - sw) * 0.5,
		design_h * 0.88 + 20.0,
		sub_sz,
		Color::new(
			crate::theme::TEXT_HELP.r,
			crate::theme::TEXT_HELP.g,
			crate::theme::TEXT_HELP.b,
			0.85,
		),
	);
}
