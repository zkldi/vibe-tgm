//! Fullscreen post-process: **idle is a clean pass-through**; CRT-style punch (warp,
//! chroma, scanlines, tint) ramps from [`ScreenJuice`] and death stack rust. Sonic slam
//! blur is masked to the four active minos only.

use macroquad::prelude::*;
use tgm_core::{Game, Phase, PieceState, count_full_lines, find_full_lines};

/// Decaying screen-space “juice” from gameplay. Death strength is supplied separately
/// each frame from [`crate::playfield_fx::PlayfieldFx::death_rust_amount`].
#[derive(Clone, Debug)]
pub struct ScreenJuice {
	/// Line clear burst (multi-line clears add more in [`Self::trigger_line_clear`]).
	pub line_clear: f32,
	/// Lock with no line clear.
	pub lock_punch: f32,
	/// Grade increase celebration.
	pub grade_pulse: f32,
	/// 100-level section transition (timer split readout); drives fullscreen shader punch.
	pub section_split: f32,
}

impl Default for ScreenJuice {
	fn default() -> Self {
		Self {
			line_clear: 0.0,
			lock_punch: 0.0,
			grade_pulse: 0.0,
			section_split: 0.0,
		}
	}
}

impl ScreenJuice {
	pub fn reset(&mut self) {
		*self = Self::default();
	}

	pub fn tick(&mut self, dt: f32) {
		let d = dt.max(0.0);
		self.line_clear *= (-d * 11.0).exp();
		self.lock_punch *= (-d * 22.0).exp();
		self.grade_pulse *= (-d * 3.8).exp();
		self.section_split *= (-d * 5.4).exp();
	}

	pub fn trigger_line_clear(&mut self, lines: u32) {
		if lines == 0 {
			return;
		}
		let add = 0.18 + lines as f32 * 0.17;
		self.line_clear = (self.line_clear + add).min(1.0);
	}

	pub fn trigger_lock(&mut self) {
		self.lock_punch = self.lock_punch.max(0.26);
	}

	pub fn trigger_grade(&mut self) {
		self.grade_pulse = self.grade_pulse.max(0.62);
	}

	pub fn trigger_section_split(&mut self) {
		self.section_split = self.section_split.max(0.94);
	}
}

/// After `game.step`, if a piece just locked, bump juice for line clear or ARE lock.
pub fn feed_screen_juice_after_step(
	game: &Game,
	piece_before: Option<PieceState>,
	juice: &mut ScreenJuice,
) {
	if piece_before.is_none() || game.piece.is_some() || game.game_over {
		return;
	}
	match game.phase {
		Phase::LineClear => {
			let n = count_full_lines(&find_full_lines(&game.board));
			if n > 0 {
				juice.trigger_line_clear(n);
			}
		}
		Phase::Are => juice.trigger_lock(),
		Phase::Falling => {}
	}
}

/// Vertex shader compatible with macroquad’s textured quad batching.
const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

const FRAGMENT: &str = r#"#version 100
precision mediump float;

varying lowp vec4 color;
varying lowp vec2 uv;

uniform sampler2D Texture;
uniform vec4 _Time;
uniform vec2 u_screen_size;
uniform vec4 u_juice;
uniform float u_section_split;
uniform float u_sonic_blur;
uniform vec4 u_mino0;
uniform vec4 u_mino1;
uniform vec4 u_mino2;
uniform vec4 u_mino3;

float in_mino(vec2 u, vec4 r) {
    vec2 mn = r.xy;
    vec2 mx = r.zw;
    return step(mn.x, u.x) * step(u.x, mx.x) * step(mn.y, u.y) * step(u.y, mx.y);
}

vec3 sonic_tex(vec2 u) {
    vec3 sharp = texture2D(Texture, u).rgb * color.rgb;
    if (u_sonic_blur < 0.001) {
        return sharp;
    }
    float m = in_mino(u, u_mino0);
    m = max(m, in_mino(u, u_mino1));
    m = max(m, in_mino(u, u_mino2));
    m = max(m, in_mino(u, u_mino3));
    if (m < 0.01) {
        return sharp;
    }
    float dv = (u_sonic_blur * 9.5) / u_screen_size.y;
    vec3 b = sharp * 0.22;
    b += texture2D(Texture, u + vec2(0.0, dv * 1.0)).rgb * color.rgb * 0.18;
    b += texture2D(Texture, u + vec2(0.0, dv * -1.0)).rgb * color.rgb * 0.18;
    b += texture2D(Texture, u + vec2(0.0, dv * 2.0)).rgb * color.rgb * 0.13;
    b += texture2D(Texture, u + vec2(0.0, dv * -2.0)).rgb * color.rgb * 0.13;
    b += texture2D(Texture, u + vec2(0.0, dv * 3.0)).rgb * color.rgb * 0.085;
    b += texture2D(Texture, u + vec2(0.0, dv * -3.0)).rgb * color.rgb * 0.085;
    b += texture2D(Texture, u + vec2(0.0, dv * 4.0)).rgb * color.rgb * 0.05;
    b += texture2D(Texture, u + vec2(0.0, dv * -4.0)).rgb * color.rgb * 0.05;
    b += texture2D(Texture, u + vec2(0.0, dv * 5.0)).rgb * color.rgb * 0.03;
    b += texture2D(Texture, u + vec2(0.0, dv * -5.0)).rgb * color.rgb * 0.03;
    return mix(sharp, b, m * u_sonic_blur * 0.82);
}

vec2 curve_uv(vec2 uv) {
    uv = uv * 2.0 - 1.0;
    vec2 offset = abs(uv.yx) / vec2(22.0, 16.0);
    uv = uv + uv * offset * offset * 0.48;
    uv = uv * 0.5 + 0.5;
    return uv;
}

void main() {
    float line = u_juice.x;
    float lock = u_juice.y;
    float grade = u_juice.z;
    float death = u_juice.w;
    float section = u_section_split;
    float pulse = max(max(line, lock), max(grade, death * 0.95));
    pulse = max(pulse, section * 0.96);

    // `section` can be small but still visible; do not fast-path while it decays.
    if (pulse < 0.002 && section < 0.0004) {
        gl_FragColor = vec4(sonic_tex(uv), 1.0);
        return;
    }

    vec2 dir = (uv - 0.5) * 2.0;
    float edge = clamp(length(dir), 0.0, 1.0);

    float warp = line * 0.42 + death * 0.65 + grade * 0.12 + lock * 0.18 + section * 0.38;
    vec2 w = mix(uv, curve_uv(uv), clamp(warp, 0.0, 1.0));

    if (w.x < 0.0 || w.x > 1.0 || w.y < 0.0 || w.y > 1.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    float ca = edge * (line * 0.0055 + lock * 0.0014 + grade * 0.0022 + death * 0.0035 + section * 0.0095);
    vec2 cao = normalize(dir + vec2(0.0001)) * ca;
    vec3 tr = sonic_tex(w + cao);
    vec3 tg = sonic_tex(w);
    vec3 tb = sonic_tex(w - cao);
    vec3 col = vec3(tr.r, tg.g, tb.b);

    col += vec3(0.09, 0.1, 0.12) * line;
    col += vec3(0.14, 0.1, 0.22) * section;
    float irid = _Time.x * 6.2831853 * 0.35 + edge * 8.0;
    col.r += 0.12 * section * sin(irid);
    col.g += 0.1 * section * sin(irid + 2.094);
    col.b += 0.14 * section * sin(irid + 4.189);
    col *= 1.0 + 0.1 * line * col;

    col *= mix(vec3(1.0), vec3(1.06, 0.98, 0.88), grade * 0.55);

    float scan_mix = clamp(line * 0.85 + lock * 0.35 + death * 0.15 + section * 0.55, 0.0, 1.0);
    float lines = u_screen_size.y;
    float drift = 0.0004 * _Time.x;
    float scan = 0.94 + 0.06 * cos(3.14159265 * (w.y + drift) * lines);
    col = mix(col, col * scan, scan_mix);

    float mask_mix = clamp(line * 0.7 + grade * 0.25 + section * 0.4, 0.0, 1.0);
    float mask = 0.94 + 0.06 * cos(3.14159265 * w.x * u_screen_size.x * 0.65);
    col = mix(col, col * mask, mask_mix);

    float vig = w.x * w.y * (1.0 - w.x) * (1.0 - w.y);
    vig = clamp(pow(16.0 * vig, 0.28), 0.0, 1.0);
    float vig_push = death * 0.55 + grade * 0.24 + section * 0.42;
    col *= mix(1.0, vig, clamp(vig_push, 0.0, 1.0));

    col *= mix(vec3(1.0), vec3(0.62, 0.35, 0.32), death * 0.72);

    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
"#;

pub struct ScreenPostFx {
	material: Material,
}

impl ScreenPostFx {
	pub fn new() -> Result<Self, macroquad::Error> {
		let material = load_material(
			ShaderSource::Glsl {
				vertex: VERTEX,
				fragment: FRAGMENT,
			},
			MaterialParams {
				uniforms: vec![
					UniformDesc::new("u_screen_size", UniformType::Float2),
					UniformDesc::new("u_juice", UniformType::Float4),
					UniformDesc::new("u_section_split", UniformType::Float1),
					UniformDesc::new("u_sonic_blur", UniformType::Float1),
					UniformDesc::new("u_mino0", UniformType::Float4),
					UniformDesc::new("u_mino1", UniformType::Float4),
					UniformDesc::new("u_mino2", UniformType::Float4),
					UniformDesc::new("u_mino3", UniformType::Float4),
				],
				..Default::default()
			},
		)?;
		Ok(Self { material })
	}

	/// Draw the offscreen scene texture into the window letterbox with post-processing.
	pub fn draw_composite(
		&self,
		texture: &Texture2D,
		ox: f32,
		oy: f32,
		vw: f32,
		vh: f32,
		design_w: f32,
		design_h: f32,
		juice: &ScreenJuice,
		death_strength: f32,
		sonic_blur: f32,
		piece_mino_uvs: &[f32; 16],
	) {
		let d = death_strength.clamp(0.0, 1.0);
		self.material.set_uniform(
			"u_juice",
			vec4(juice.line_clear, juice.lock_punch, juice.grade_pulse, d),
		);
		self.material
			.set_uniform("u_section_split", juice.section_split.clamp(0.0, 1.0));
		self.material
			.set_uniform("u_screen_size", vec2(design_w, design_h));
		self.material
			.set_uniform("u_sonic_blur", sonic_blur.clamp(0.0, 1.0));
		let m = piece_mino_uvs;
		self.material
			.set_uniform("u_mino0", vec4(m[0], m[1], m[2], m[3]));
		self.material
			.set_uniform("u_mino1", vec4(m[4], m[5], m[6], m[7]));
		self.material
			.set_uniform("u_mino2", vec4(m[8], m[9], m[10], m[11]));
		self.material
			.set_uniform("u_mino3", vec4(m[12], m[13], m[14], m[15]));
		gl_use_material(&self.material);
		draw_texture_ex(
			texture,
			ox,
			oy,
			WHITE,
			DrawTextureParams {
				dest_size: Some(vec2(vw, vh)),
				flip_y: true,
				..Default::default()
			},
		);
		gl_use_default_material();
	}
}
