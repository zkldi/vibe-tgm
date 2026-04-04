//! TGM1-style client: WASD + J/K/L (L = CCW duplicate), F3 debug.

mod backgrounds;
mod hud;
mod persist;
mod playfield_fx;
mod postprocess;
mod theme;
mod title_codes;

use std::collections::{HashSet, VecDeque};

use ::rand::Rng;
use chrono::{DateTime, Local, Utc};
use macroquad::prelude::*;
use postprocess::{ScreenJuice, feed_screen_juice_after_step};
use persist::{
	ClientSettings, HighScoreEntry, HighScoresFile, ReplayFile, ReplayListEntry, ReplaySummary,
	load_client_settings, load_highscores, load_replay, load_replay_list_entries, merge_highscore,
	now_ms, replays_dir, save_client_settings, save_replay,
};
use playfield_fx::{
	DEATH_FRAMES_MAX, PlayfieldFx, horizontal_target_from_keys, horizontal_target_from_replay_byte,
	hud_vertical_jolt_from_keys, hud_vertical_jolt_from_replay_byte,
};
use tgm_core::{
	BIG_BOARD_HEIGHT, BIG_BOARD_WIDTH, BIG_VISIBLE_ROWS, BOARD_HEIGHT, BOARD_WIDTH, Board, EMPTY,
	Game, GameOptions, Grade, Input, Phase, PieceKind, TLS_MAX_LEVEL, VISIBLE_ROWS,
	autoplay_plan_inputs, clear_lines, count_full_lines, find_full_lines, input_pack, input_unpack,
	piece_cells, piece_cells_big, rotate_ccw, rotate_cw,
};
use theme::{
	ArcadeFont, ClockworkBackground, LETTERBOX, PANEL_BORDER, cell_color, dim_stack_cell,
	draw_cell_beveled, draw_clockwork_background, draw_playfield_frame, well_fill_color,
};
use title_codes::{TITLE_BUFFER_CAP, TitleToken, decode_options};

use crate::hud::{GradeUpAnim, HudRotFeel};

const CELL: f32 = 22.0;
const BIG_CELL: f32 = 44.0;
const MARGIN: f32 = 24.0;

/// Playfield width (normal and big mode both use 10×22 / 5×44 = 220px).
const FIELD_COL_W: f32 = BOARD_WIDTH as f32 * CELL;
/// Gap between side rails and the centered playfield column.
const HUD_GAP: f32 = 24.0;
/// Left edge of the playfield column (before death shake), after the left HUD rail.
const FIELD_OX_BASE: f32 = MARGIN + hud::HUD_W + HUD_GAP;
/// Left edge of the right HUD rail (“Free Play” / replay).
const RIGHT_RAIL_X: f32 = FIELD_OX_BASE + FIELD_COL_W + HUD_GAP;

/// Logical canvas: `[margin][left HUD][gap][field][gap][right HUD][margin]`.
const DESIGN_WIDTH: f32 =
	MARGIN + hud::HUD_W + HUD_GAP + FIELD_COL_W + HUD_GAP + hud::HUD_W + MARGIN;
/// Top edge of the playfield interior / frame (below NEXT strip + gap).
const FIELD_TOP: f32 = MARGIN + hud::NEXT_ZONE_H + hud::NEXT_PLAYFIELD_GAP;
const DESIGN_HEIGHT: f32 = FIELD_TOP + VISIBLE_ROWS as f32 * CELL + hud::TIMER_ZONE_H + MARGIN;

/// Vertical placement for the right-rail panel (roughly centered on the stack area).
fn right_rail_panel_y() -> f32 {
	FIELD_TOP + VISIBLE_ROWS as f32 * CELL * 0.5 - 46.0
}

const WINDOW_PRESETS: [(u32, u32); 4] = [(1280, 720), (1600, 900), (1920, 1080), (1024, 768)];

const STEP_SEC: f64 = 1.0 / 60.0;

/// Replay scrub bar (design space); click / drag to seek.
const REPLAY_BAR_H: f32 = 22.0;
const REPLAY_BAR_Y: f32 = DESIGN_HEIGHT - 56.0;

/// BG test screen: slider for stack proximity (drives 1×–3× animation speed).
const BG_TEST_SLIDER_Y: f32 = 118.0;
const BG_TEST_SLIDER_H: f32 = 22.0;

/// Half of a fade-to-black transition (fade out or fade in). Full cross is `2 *` this.
const TRANS_HALF_SEC: f32 = 0.13;

const TITLE_MENU_ITEMS: [&str; 6] = [
	"Normal",
	"Autoplay",
	"High Scores",
	"Replays",
	"Settings",
	"Exit",
];

const SETTINGS_MENU_ITEMS: [&str; 2] = ["Background test", "Back"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientState {
	Title,
	Playing,
	PostGame,
	ReplayList,
	ReplayPlayback,
	HighScores,
	Settings,
	/// Cycle through [`ClockworkBackground::ALL`] (dev / preview).
	BgAnimTest,
}

#[derive(Clone, Copy)]
enum ScreenTransition {
	None,
	FadeOut { next: ClientState, t: f32 },
	FadeIn { t: f32 },
}

fn screen_transition_active(t: &ScreenTransition) -> bool {
	!matches!(t, ScreenTransition::None)
}

fn screen_transition_begin(trans: &mut ScreenTransition, next: ClientState) {
	if matches!(*trans, ScreenTransition::None) {
		*trans = ScreenTransition::FadeOut { next, t: 0.0 };
	}
}

/// Smoothstep 0..1 for overlay alpha.
fn trans_fade_ease(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

fn draw_screen_fade_overlay(t: &ScreenTransition) {
	let (a, w, h) = match t {
		ScreenTransition::FadeOut { t, .. } => (trans_fade_ease(*t), DESIGN_WIDTH, DESIGN_HEIGHT),
		ScreenTransition::FadeIn { t } => (1.0 - trans_fade_ease(*t), DESIGN_WIDTH, DESIGN_HEIGHT),
		ScreenTransition::None => return,
	};
	if a <= 0.001 {
		return;
	}
	let u8a = (a * 255.0).round() as u8;
	draw_rectangle(0.0, 0.0, w, h, Color::from_rgba(0, 0, 0, u8a));
}

fn screen_transition_after_fade_out(
	old: ClientState,
	new: ClientState,
	title_keys_down_prev: &mut HashSet<KeyCode>,
	settings_keys_prev: &mut HashSet<KeyCode>,
	game: &mut Option<Game>,
	replay_record: &mut Vec<u8>,
	title_buffer: &mut Vec<TitleToken>,
	replay_watch: &mut Option<ReplayWatch>,
	replay_list_entries: &mut Vec<ReplayListEntry>,
	replay_list_scroll: &mut usize,
	replay_list_prev: &mut HashSet<KeyCode>,
	replay_mouse_drag: &mut bool,
) {
	match (old, new) {
		(ClientState::PostGame, ClientState::Title) => {
			*game = None;
			replay_record.clear();
			title_buffer.clear();
			*title_keys_down_prev = get_keys_down();
		}
		(ClientState::ReplayList, ClientState::Title) => {
			title_keys_down_prev.clear();
		}
		(ClientState::ReplayPlayback, ClientState::ReplayList) => {
			*replay_mouse_drag = false;
			*replay_watch = None;
			*replay_list_entries = load_replay_list_entries();
			*replay_list_scroll = 0;
			replay_list_prev.clear();
		}
		(ClientState::HighScores, ClientState::Title) => {
			title_keys_down_prev.clear();
		}
		(ClientState::Settings, ClientState::Title) => {
			title_keys_down_prev.clear();
		}
		(ClientState::BgAnimTest, ClientState::Settings) => {
			*settings_keys_prev = get_keys_down();
		}
		_ => {}
	}
}

/// Active replay playback (separate from [`ClientState`] so the main loop can swap state without
/// borrow conflicts).
struct ReplayWatch {
	seed: u64,
	game: Game,
	inputs: Vec<u8>,
	/// First input index where each hundred (100..=900) or 999 is reached.
	hundred_marks: Vec<(usize, u16)>,
	idx: usize,
	step_accum: f64,
	paused: bool,
}

/// Rebuild [`Game`] after applying the first `target_idx` replay inputs (for scrubbing).
fn replay_simulate_to(
	seed: u64,
	options: GameOptions,
	inputs: &[u8],
	target_idx: usize,
	playfield_fx: &mut PlayfieldFx,
) -> Game {
	playfield_fx.reset();
	let mut g = Game::with_options(seed, options);
	let n = target_idx.min(inputs.len());
	for i in 0..n {
		let piece_before = g.piece;
		let inp = input_unpack(inputs[i]).expect("replay byte");
		g.step(inp);
		playfield_fx.after_step(&g, piece_before, inp);
	}
	// Bulk sim has no per-frame decay; avoid a stale slam offset after timeline scrub.
	playfield_fx.clear_sonic_slam();
	g
}

/// Simulate the full replay (no VFX) to record level at each input index.
fn replay_level_timeline(seed: u64, options: GameOptions, inputs: &[u8]) -> Vec<u16> {
	let mut g = Game::with_options(seed, options);
	let mut levels = Vec::with_capacity(inputs.len() + 1);
	levels.push(g.level);
	for &b in inputs {
		let inp = input_unpack(b).expect("replay byte");
		g.step(inp);
		levels.push(g.level);
	}
	levels
}

fn level_hundred_marks(levels: &[u16]) -> Vec<(usize, u16)> {
	let mut out = Vec::new();
	for h in (100..=900).step_by(100).map(|x| x as u16) {
		for i in 1..levels.len() {
			if levels[i - 1] < h && levels[i] >= h {
				out.push((i, h));
				break;
			}
		}
	}
	for i in 1..levels.len() {
		if levels[i - 1] < 999 && levels[i] >= 999 {
			out.push((i, 999));
			break;
		}
	}
	out
}

fn letterbox_in_screen(sw: f32, sh: f32) -> (f32, f32, f32, f32) {
	let scale = (sw / DESIGN_WIDTH).min(sh / DESIGN_HEIGHT);
	let vw = DESIGN_WIDTH * scale;
	let vh = DESIGN_HEIGHT * scale;
	let ox = (sw - vw) * 0.5;
	let oy = (sh - vh) * 0.5;
	(ox, oy, vw, vh)
}

/// Window pixels → design-space coords (matches letterboxed `flip_y` texture draw).
///
/// `v` increases top→bottom of the letterbox; design `y` increases top→bottom (`y=0` at top).
fn screen_to_design_coords(mx: f32, my: f32, sw: f32, sh: f32) -> Option<(f32, f32)> {
	let (ox, oy, vw, vh) = letterbox_in_screen(sw, sh);
	if mx < ox || mx > ox + vw || my < oy || my > oy + vh {
		return None;
	}
	let u = (mx - ox) / vw;
	let v = (my - oy) / vh;
	let dx = u * DESIGN_WIDTH;
	let dy = v * DESIGN_HEIGHT;
	Some((dx, dy))
}

fn replay_bar_geom() -> (f32, f32, f32, f32) {
	let x = MARGIN;
	let w = DESIGN_WIDTH - 2.0 * MARGIN;
	(x, REPLAY_BAR_Y, w, REPLAY_BAR_H)
}

fn bg_test_stack_slider_geom() -> (f32, f32, f32, f32) {
	let x = MARGIN;
	let w = DESIGN_WIDTH - 2.0 * MARGIN;
	(x, BG_TEST_SLIDER_Y, w, BG_TEST_SLIDER_H)
}

fn point_in_bg_test_stack_slider(px: f32, py: f32) -> bool {
	let (x, y, w, h) = bg_test_stack_slider_geom();
	px >= x && px <= x + w && py >= y - 10.0 && py <= y + h + 4.0
}

fn point_in_replay_bar(px: f32, py: f32) -> bool {
	let (x, y, w, h) = replay_bar_geom();
	// Include tick labels above the track for easier grabbing.
	px >= x && px <= x + w && py >= y - 14.0 && py <= y + h
}

fn replay_seek_to(
	rw: &mut ReplayWatch,
	new_idx: usize,
	playfield_fx: &mut PlayfieldFx,
	screen_juice: &mut ScreenJuice,
	grade_last: &mut Grade,
	grade_up_fx: &mut Option<GradeUpAnim>,
) {
	let new_idx = new_idx.min(rw.inputs.len());
	if new_idx == rw.idx {
		return;
	}
	let seed = rw.seed;
	let opts = rw.game.options;
	rw.game = replay_simulate_to(seed, opts, &rw.inputs, new_idx, playfield_fx);
	rw.idx = new_idx;
	rw.step_accum = 0.0;
	*grade_last = rw.game.grade();
	*grade_up_fx = None;
	screen_juice.reset();
}

fn draw_replay_timeline(font: &ArcadeFont, rw: &ReplayWatch) {
	let (bx, by, bw, bh) = replay_bar_geom();
	let n = rw.inputs.len().max(1);
	let frac = (rw.idx as f32 / n as f32).clamp(0.0, 1.0);

	let track = Color::from_rgba(18, 18, 26, 255);
	let fill = Color::from_rgba(55, 70, 95, 255);
	let border = theme::PANEL_BORDER;
	let tick = Color::from_rgba(255, 200, 90, 255);
	let playhead = Color::from_rgba(255, 255, 255, 255);

	draw_rectangle(bx, by, bw, bh, track);
	draw_rectangle(bx, by, bw * frac, bh, fill);
	draw_rectangle_lines(bx, by, bw, bh, 1.5, border);

	for &(frame_i, level) in &rw.hundred_marks {
		let t = frame_i as f32 / n as f32;
		let x = bx + t * bw;
		draw_line(x, by + bh - 1.0, x, by - 6.0, 1.2, tick);
		let label = format!("{}", level);
		let tw = font.measure(&label, 8.0).width;
		font.draw(
			&label,
			(x - tw * 0.5).max(bx + 2.0).min(bx + bw - tw - 2.0),
			by - 12.0,
			8.0,
			tick,
		);
	}

	let px = bx + frac * bw;
	draw_line(px, by, px, by + bh, 2.0, playhead);
}

fn window_conf() -> Conf {
	let s = load_client_settings();
	Conf {
		window_title: "TGM1 (Rust)".to_string(),
		window_width: s.window_width as i32,
		window_height: s.window_height as i32,
		fullscreen: s.fullscreen,
		..Default::default()
	}
}

#[macroquad::main(window_conf)]
async fn main() {
	next_frame().await;
	let font = ArcadeFont::try_load().expect("load embedded Press Start 2P font");

	// RNG seed for the current run; set when a game starts and used when saving a replay.
	let mut game_seed: u64 = 0;
	let mut client_state = ClientState::Title;
	let mut screen_transition = ScreenTransition::None;
	let mut title_buffer: Vec<TitleToken> = Vec::new();
	let mut title_menu_idx: usize = 0;
	let mut settings_menu_idx: usize = 0;
	let mut settings_keys_prev: HashSet<KeyCode> = HashSet::new();
	let mut game: Option<Game> = None;
	let mut play_options = GameOptions::default();
	let mut playing_autoplay = false;
	let mut autoplay_bot = AutoplayBot::default();

	let mut debug_overlay = false;
	let mut step_accum: f64 = 0.0;
	let mut pending_rot_cw: u8 = 0;
	let mut pending_rot_ccw: u8 = 0;
	// Previous frame's `get_keys_down()` for rising-edge detection (`is_key_pressed` can miss keys
	// because `keys_pressed` is cleared in macroquad's `end_frame()`).
	let mut title_keys_down_prev: HashSet<KeyCode> = HashSet::new();
	let mut replay_record: Vec<u8> = Vec::new();
	let mut replay_list_scroll: usize = 0;
	let mut replay_list_entries: Vec<ReplayListEntry> = Vec::new();
	let mut replay_list_prev: HashSet<KeyCode> = HashSet::new();
	let mut bg_anim_idx: usize = 0;
	let mut bg_anim_keys_prev: HashSet<KeyCode> = HashSet::new();
	// Stack proximity 0..=1 for BG test (same factor as gameplay `bg_danger_proximity`).
	let mut bg_anim_test_stack: f32 = 0.0;
	let mut bg_anim_slider_drag: bool = false;
	let mut replay_watch: Option<ReplayWatch> = None;
	let mut replay_mouse_drag = false;
	let mut playfield_fx = PlayfieldFx::default();
	// Integrated BG phase: advance with `dt` so stack-height multiplier changes do not jump phase.
	let mut bg_anim_phase: f32 = 0.0;
	let mut hud_rot_feel = HudRotFeel::default();
	let mut grade_last: Grade = Grade::Nine;
	let mut grade_up_fx: Option<GradeUpAnim> = None;

	let initial_settings = load_client_settings();
	let mut last_persisted_size = (
		initial_settings.window_width,
		initial_settings.window_height,
	);
	let mut fullscreen = initial_settings.fullscreen;
	let mut window_preset_idx = WINDOW_PRESETS
		.iter()
		.position(|&(w, h)| w == last_persisted_size.0 && h == last_persisted_size.1)
		.unwrap_or(0);

	// Render design-sized scene to a texture, then letterbox to the window (see macroquad
	// `examples/letterbox.rs` — viewport letterboxing interacts badly with Y; `flip_y` fixes it).
	let design_rt = render_target(DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32);
	design_rt.texture.set_filter(FilterMode::Linear);
	let mut render_target_cam =
		Camera2D::from_display_rect(Rect::new(0., 0., DESIGN_WIDTH, DESIGN_HEIGHT));
	render_target_cam.render_target = Some(design_rt.clone());
	let screen_fx = postprocess::ScreenPostFx::new().expect("load screen postprocess shader");
	let mut screen_juice = ScreenJuice::default();

	loop {
		let alt_enter = (is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt))
			&& is_key_pressed(KeyCode::Enter);
		if is_key_pressed(KeyCode::F11) || alt_enter {
			if !fullscreen {
				last_persisted_size = (screen_width() as u32, screen_height() as u32);
			}
			fullscreen = !fullscreen;
			set_fullscreen(fullscreen);
			let _ = save_client_settings(&ClientSettings {
				window_width: last_persisted_size.0,
				window_height: last_persisted_size.1,
				fullscreen,
			});
		}
		if matches!(client_state, ClientState::Settings)
			&& !screen_transition_active(&screen_transition)
			&& is_key_pressed(KeyCode::F8)
		{
			window_preset_idx = (window_preset_idx + 1) % WINDOW_PRESETS.len();
			let (w, h) = WINDOW_PRESETS[window_preset_idx];
			if fullscreen {
				fullscreen = false;
				set_fullscreen(false);
			}
			request_new_screen_size(w as f32, h as f32);
			last_persisted_size = (w, h);
			let _ = save_client_settings(&ClientSettings {
				window_width: w,
				window_height: h,
				fullscreen: false,
			});
		}

		if is_key_pressed(KeyCode::F3) {
			debug_overlay = !debug_overlay;
		}

		playfield_fx.tick_lock_flash();
		let dt = get_frame_time();
		screen_juice.tick(dt);
		playfield_fx.tick_sonic_slam(dt);
		let mut replay_rot_input: Option<Input> = None;
		match &mut screen_transition {
			ScreenTransition::FadeOut { next, t } => {
				*t += dt / TRANS_HALF_SEC;
				if *t >= 1.0 {
					let old = client_state;
					let next_state = *next;
					screen_transition_after_fade_out(
						old,
						next_state,
						&mut title_keys_down_prev,
						&mut settings_keys_prev,
						&mut game,
						&mut replay_record,
						&mut title_buffer,
						&mut replay_watch,
						&mut replay_list_entries,
						&mut replay_list_scroll,
						&mut replay_list_prev,
						&mut replay_mouse_drag,
					);
					client_state = next_state;
					screen_transition = ScreenTransition::FadeIn { t: 0.0 };
				}
			}
			ScreenTransition::FadeIn { t } => {
				*t += dt / TRANS_HALF_SEC;
				if *t >= 1.0 {
					screen_transition = ScreenTransition::None;
				}
			}
			ScreenTransition::None => {}
		}

		if !screen_transition_active(&screen_transition) {
			match &mut client_state {
				ClientState::Title => {
					let down = get_keys_down();
					poll_title_input(&mut title_buffer, &title_keys_down_prev, &down);
					if key_just_pressed(&title_keys_down_prev, &down, KeyCode::Up) {
						title_menu_idx =
							(title_menu_idx + TITLE_MENU_ITEMS.len() - 1) % TITLE_MENU_ITEMS.len();
					} else if key_just_pressed(&title_keys_down_prev, &down, KeyCode::Down) {
						title_menu_idx = (title_menu_idx + 1) % TITLE_MENU_ITEMS.len();
					} else if key_just_pressed(&title_keys_down_prev, &down, KeyCode::Enter) {
						match title_menu_idx {
							0 | 1 => {
								play_options = decode_options(&title_buffer);
								play_options.autoplay = title_menu_idx == 1;
								game_seed = ::rand::thread_rng().gen();
								game = Some(Game::with_options(game_seed, play_options));
								playing_autoplay = title_menu_idx == 1;
								autoplay_bot.reset();
								hud_rot_feel = HudRotFeel::default();
								grade_last = Grade::Nine;
								grade_up_fx = None;
								playfield_fx.reset();
								screen_juice.reset();
								replay_record.clear();
								screen_transition_begin(
									&mut screen_transition,
									ClientState::Playing,
								);
								step_accum = 0.0;
								pending_rot_cw = 0;
								pending_rot_ccw = 0;
							}
							2 => {
								screen_transition_begin(
									&mut screen_transition,
									ClientState::HighScores,
								);
							}
							3 => {
								replay_list_entries = load_replay_list_entries();
								replay_list_scroll = 0;
								replay_list_prev = down.clone();
								screen_transition_begin(
									&mut screen_transition,
									ClientState::ReplayList,
								);
							}
							4 => {
								settings_menu_idx = 0;
								settings_keys_prev = down.clone();
								screen_transition_begin(
									&mut screen_transition,
									ClientState::Settings,
								);
							}
							5 => {
								macroquad::window::miniquad::window::quit();
							}
							_ => {}
						}
					}
					title_keys_down_prev = down;
				}
				ClientState::PostGame => {
					if is_key_pressed(KeyCode::Enter) {
						playing_autoplay = false;
						screen_transition_begin(&mut screen_transition, ClientState::Title);
					}
				}
				ClientState::Playing => {
					let g = game.as_mut().expect("playing");

					if !playing_autoplay {
						if !matches!(g.phase, Phase::Falling) {
							pending_rot_cw = 0;
							pending_rot_ccw = 0;
						} else {
							if is_key_pressed(KeyCode::K) {
								pending_rot_cw = pending_rot_cw.saturating_add(1).min(8);
							}
							if is_key_pressed(KeyCode::J) || is_key_pressed(KeyCode::L) {
								pending_rot_ccw = pending_rot_ccw.saturating_add(1).min(8);
							}
						}
					} else if !matches!(g.phase, Phase::Falling) {
						pending_rot_cw = 0;
						pending_rot_ccw = 0;
					}

					step_accum += get_frame_time() as f64;
					step_accum = step_accum.min(STEP_SEC * 5.0);
					if step_accum >= STEP_SEC {
						let piece_before = g.piece;
						let inp = if playing_autoplay {
							autoplay_bot.pick_input(g)
						} else {
							poll_input(g.phase, &mut pending_rot_cw, &mut pending_rot_ccw)
						};
						replay_record.push(input_pack(inp));
						g.step(inp);
						playfield_fx.after_step(g, piece_before, inp);
						feed_screen_juice_after_step(g, piece_before, &mut screen_juice);
						step_accum -= STEP_SEC;
						let gr = g.grade();
						if gr > grade_last {
							grade_up_fx = Some(GradeUpAnim::new());
							screen_juice.trigger_grade();
						}
						grade_last = gr;
					}

					if g.game_over || g.cleared {
						if g.eligible_for_hiscore() {
							let entry = HighScoreEntry {
								score: g.score,
								grade: g.grade_label().to_string(),
								level: g.level,
								cleared: g.cleared,
								gm: g.cleared && g.gm_qualified,
								saved_at_ms: now_ms(),
							};
							let _ = merge_highscore(load_highscores(), entry);
						}
						if !replay_record.is_empty() {
							let t = now_ms();
							let rf = ReplayFile::new(
								game_seed,
								g.options,
								replay_record.clone(),
								t,
								ReplaySummary {
									score: g.score,
									level: g.level,
									grade: g.grade_label().to_string(),
								},
							);
							let name = format!("replay_{}.json", t);
							let path = replays_dir().join(name);
							let _ = save_replay(&path, &rf);
						}
						client_state = ClientState::PostGame;
					}
				}
				ClientState::ReplayList => {
					let down = get_keys_down();
					if key_just_pressed(&replay_list_prev, &down, KeyCode::Escape) {
						screen_transition_begin(&mut screen_transition, ClientState::Title);
					} else if key_just_pressed(&replay_list_prev, &down, KeyCode::Up)
						|| key_just_pressed(&replay_list_prev, &down, KeyCode::W)
					{
						replay_list_scroll = replay_list_scroll.saturating_sub(1);
					} else if key_just_pressed(&replay_list_prev, &down, KeyCode::Down)
						|| key_just_pressed(&replay_list_prev, &down, KeyCode::S)
					{
						if !replay_list_entries.is_empty() {
							replay_list_scroll =
								(replay_list_scroll + 1).min(replay_list_entries.len() - 1);
						}
					} else if key_just_pressed(&replay_list_prev, &down, KeyCode::Enter)
						&& !replay_list_entries.is_empty()
					{
						let path = replay_list_entries[replay_list_scroll].path.clone();
						if let Ok(r) = load_replay(&path) {
							let g = Game::with_options(r.seed, r.options);
							grade_last = g.grade();
							grade_up_fx = None;
							playfield_fx.reset();
							screen_juice.reset();
							let levels = replay_level_timeline(r.seed, r.options, &r.inputs);
							let hundred_marks = level_hundred_marks(&levels);
							replay_mouse_drag = false;
							hud_rot_feel = HudRotFeel::default();
							replay_watch = Some(ReplayWatch {
								seed: r.seed,
								game: g,
								inputs: r.inputs,
								hundred_marks,
								idx: 0,
								step_accum: 0.0,
								paused: false,
							});
							screen_transition_begin(
								&mut screen_transition,
								ClientState::ReplayPlayback,
							);
						}
					}
					replay_list_prev = down;
				}
				ClientState::ReplayPlayback => {
					let sw = screen_width();
					let sh = screen_height();
					let mouse = mouse_position();
					if let Some(rw) = replay_watch.as_mut() {
						if let Some((dx, dy)) = screen_to_design_coords(mouse.0, mouse.1, sw, sh) {
							if is_mouse_button_pressed(MouseButton::Left)
								&& point_in_replay_bar(dx, dy)
							{
								replay_mouse_drag = true;
							}
						}
						if is_mouse_button_released(MouseButton::Left) {
							if replay_mouse_drag {
								rw.paused = false;
							}
							replay_mouse_drag = false;
						}
						if replay_mouse_drag && is_mouse_button_down(MouseButton::Left) {
							let (bx, _, bw, _) = replay_bar_geom();
							let (ox, _, vw, _) = letterbox_in_screen(sw, sh);
							let mx = mouse.0.clamp(ox, ox + vw);
							let ux = ((mx - ox) / vw) * DESIGN_WIDTH;
							let frac = ((ux - bx) / bw).clamp(0.0, 1.0);
							let new_idx = (frac * rw.inputs.len() as f32).round() as usize;
							let new_idx = new_idx.min(rw.inputs.len());
							rw.paused = true;
							replay_seek_to(
								rw,
								new_idx,
								&mut playfield_fx,
								&mut screen_juice,
								&mut grade_last,
								&mut grade_up_fx,
							);
						}

						if is_key_pressed(KeyCode::P) {
							rw.paused = !rw.paused;
						}

						let g = &mut rw.game;
						let inputs = &rw.inputs;
						let idx = &mut rw.idx;
						let r_accum = &mut rw.step_accum;
						let r_paused = &mut rw.paused;

						let mut step_once = !*r_paused;
						if *r_paused && is_key_pressed(KeyCode::Period) {
							step_once = true;
						}

						let replay_done = *idx >= inputs.len() || g.game_over || g.cleared;
						if !replay_done {
							if !*r_paused {
								*r_accum += get_frame_time() as f64;
								*r_accum = (*r_accum).min(STEP_SEC * 5.0);
								if step_once && *r_accum >= STEP_SEC && *idx < inputs.len() {
									let piece_before = g.piece;
									let inp = input_unpack(inputs[*idx]).expect("replay byte");
									g.step(inp);
									replay_rot_input = Some(inp);
									playfield_fx.after_step(g, piece_before, inp);
									feed_screen_juice_after_step(g, piece_before, &mut screen_juice);
									*idx += 1;
									*r_accum -= STEP_SEC;
									let gr = g.grade();
									if gr > grade_last {
										grade_up_fx = Some(GradeUpAnim::new());
										screen_juice.trigger_grade();
									}
									grade_last = gr;
								}
							} else if step_once && *idx < inputs.len() {
								let piece_before = g.piece;
								let inp = input_unpack(inputs[*idx]).expect("replay byte");
								g.step(inp);
								replay_rot_input = Some(inp);
								playfield_fx.after_step(g, piece_before, inp);
								feed_screen_juice_after_step(g, piece_before, &mut screen_juice);
								*idx += 1;
								let gr = g.grade();
								if gr > grade_last {
									grade_up_fx = Some(GradeUpAnim::new());
									screen_juice.trigger_grade();
								}
								grade_last = gr;
							}
						}

						if is_key_pressed(KeyCode::Escape) {
							screen_transition_begin(
								&mut screen_transition,
								ClientState::ReplayList,
							);
						}
					} else {
						client_state = ClientState::ReplayList;
					}
				}
				ClientState::HighScores => {
					if is_key_pressed(KeyCode::Escape) {
						screen_transition_begin(&mut screen_transition, ClientState::Title);
					}
				}
				ClientState::Settings => {
					let down = get_keys_down();
					if key_just_pressed(&settings_keys_prev, &down, KeyCode::Escape) {
						screen_transition_begin(&mut screen_transition, ClientState::Title);
					} else if key_just_pressed(&settings_keys_prev, &down, KeyCode::Up) {
						settings_menu_idx =
							(settings_menu_idx + SETTINGS_MENU_ITEMS.len() - 1)
								% SETTINGS_MENU_ITEMS.len();
					} else if key_just_pressed(&settings_keys_prev, &down, KeyCode::Down) {
						settings_menu_idx =
							(settings_menu_idx + 1) % SETTINGS_MENU_ITEMS.len();
					} else if key_just_pressed(&settings_keys_prev, &down, KeyCode::Enter) {
						match settings_menu_idx {
							0 => {
								bg_anim_idx = 0;
								bg_anim_test_stack = 0.0;
								bg_anim_slider_drag = false;
								bg_anim_keys_prev = down.clone();
								screen_transition_begin(
									&mut screen_transition,
									ClientState::BgAnimTest,
								);
							}
							1 => {
								screen_transition_begin(
									&mut screen_transition,
									ClientState::Title,
								);
							}
							_ => {}
						}
					}
					settings_keys_prev = down;
				}
				ClientState::BgAnimTest => {
					let down = get_keys_down();
					let n = ClockworkBackground::ALL.len().max(1);
					if key_just_pressed(&bg_anim_keys_prev, &down, KeyCode::Escape) {
						bg_anim_slider_drag = false;
						screen_transition_begin(&mut screen_transition, ClientState::Settings);
					} else if key_just_pressed(&bg_anim_keys_prev, &down, KeyCode::Left)
						|| key_just_pressed(&bg_anim_keys_prev, &down, KeyCode::A)
					{
						bg_anim_idx = (bg_anim_idx + n - 1) % n;
					} else if key_just_pressed(&bg_anim_keys_prev, &down, KeyCode::Right)
						|| key_just_pressed(&bg_anim_keys_prev, &down, KeyCode::D)
					{
						bg_anim_idx = (bg_anim_idx + 1) % n;
					}
					bg_anim_keys_prev = down;

					let sw = screen_width();
					let sh = screen_height();
					let mouse = mouse_position();
					if let Some((dx, dy)) = screen_to_design_coords(mouse.0, mouse.1, sw, sh) {
						if is_mouse_button_pressed(MouseButton::Left)
							&& point_in_bg_test_stack_slider(dx, dy)
						{
							bg_anim_slider_drag = true;
						}
					}
					if is_mouse_button_released(MouseButton::Left) {
						bg_anim_slider_drag = false;
					}
					if bg_anim_slider_drag && is_mouse_button_down(MouseButton::Left) {
						let (bx, _, bw, _) = bg_test_stack_slider_geom();
						let (ox, _, vw, _) = letterbox_in_screen(sw, sh);
						let mx = mouse.0.clamp(ox, ox + vw);
						let ux = ((mx - ox) / vw) * DESIGN_WIDTH;
						bg_anim_test_stack = ((ux - bx) / bw).clamp(0.0, 1.0);
					}
				}
			}
		}

		match &client_state {
			ClientState::Playing | ClientState::PostGame => {
				if game.is_some() {
					hud_rot_feel.tick(
						dt,
						is_key_pressed(KeyCode::K),
						is_key_pressed(KeyCode::J) || is_key_pressed(KeyCode::L),
						false,
					);
				} else {
					hud_rot_feel.tick(dt, false, false, false);
				}
			}
			ClientState::ReplayPlayback => {
				if let Some(inp) = replay_rot_input {
					hud_rot_feel.tick(dt, inp.rot_cw, inp.rot_ccw, true);
				} else {
					hud_rot_feel.tick(dt, false, false, false);
				}
			}
			_ => {
				hud_rot_feel.tick(dt, false, false, false);
			}
		}

		match &client_state {
			ClientState::Playing | ClientState::PostGame => {
				if game.is_some() {
					playfield_fx
						.wall_input
						.tick_horizontal(dt, horizontal_target_from_keys());
				} else {
					playfield_fx.wall_input.tick_horizontal(dt, 0.0);
				}
			}
			ClientState::ReplayPlayback => {
				if let Some(rw) = replay_watch.as_ref() {
					let last = rw.idx.checked_sub(1).map(|i| rw.inputs[i]);
					playfield_fx
						.wall_input
						.tick_horizontal(dt, horizontal_target_from_replay_byte(last));
				} else {
					playfield_fx.wall_input.tick_horizontal(dt, 0.0);
				}
			}
			_ => {
				playfield_fx.wall_input.tick_horizontal(dt, 0.0);
			}
		}

		if let Some(ref g) = game {
			playfield_fx.tick_death_frame(g);
		} else if let Some(rw) = replay_watch.as_ref() {
			playfield_fx.tick_death_frame(&rw.game);
		}

		let grade_fx_done = if let Some(ref mut fx) = grade_up_fx {
			fx.tick(dt);
			fx.finished()
		} else {
			false
		};
		if grade_fx_done {
			grade_up_fx = None;
		}

		set_camera(&render_target_cam);
		let clock_bg = match &client_state {
			ClientState::Title => ClockworkBackground::Title,
			ClientState::ReplayList => ClockworkBackground::ReplayList,
			ClientState::HighScores | ClientState::Settings => ClockworkBackground::Title,
			ClientState::Playing => game
				.as_ref()
				.map(|g| ClockworkBackground::from_level(g.level))
				.unwrap_or(ClockworkBackground::Title),
			ClientState::PostGame => game
				.as_ref()
				.map(|g| ClockworkBackground::from_level(g.level))
				.unwrap_or(ClockworkBackground::Title),
			ClientState::ReplayPlayback => replay_watch
				.as_ref()
				.map(|rw| ClockworkBackground::from_level(rw.game.level))
				.unwrap_or(ClockworkBackground::ReplayList),
			ClientState::BgAnimTest => {
				let n = ClockworkBackground::ALL.len();
				let idx = bg_anim_idx.min(n.saturating_sub(1));
				ClockworkBackground::ALL[idx]
			}
		};
		let bg_danger = match &client_state {
			ClientState::Playing | ClientState::PostGame => {
				game.as_ref().map(bg_danger_proximity).unwrap_or(0.0)
			}
			ClientState::ReplayPlayback => replay_watch
				.as_ref()
				.map(|rw| bg_danger_proximity(&rw.game))
				.unwrap_or(0.0),
			ClientState::BgAnimTest => bg_anim_test_stack,
			_ => 0.0,
		};
		let stack_speed_mult = 1.0 + bg_danger * 2.0;
		bg_anim_phase += dt * clock_bg.section_speed() * stack_speed_mult;
		draw_clockwork_background(DESIGN_WIDTH, DESIGN_HEIGHT, bg_anim_phase, clock_bg);

		match &client_state {
			ClientState::Title => {
				draw_title_screen(&font, title_menu_idx);
			}
			ClientState::PostGame => {
				if let Some(ref g) = game {
					let hud_time = get_time() as f32;
					let hud_jolt = hud_vertical_jolt_from_keys();
					let grade_up_t = grade_up_fx.as_ref().map(|a| a.t01());
					draw_gameplay_layer(
						&font,
						g,
						&play_options,
						&playfield_fx,
						&hud_rot_feel,
						hud_time,
						hud_jolt,
						g.options.big,
						grade_up_t,
					);
					hud::draw_right_rail(
						&font,
						RIGHT_RAIL_X,
						hud_time,
						"Free Play",
						None,
						right_rail_panel_y() + hud_jolt,
					);
					if debug_overlay {
						draw_debug(&font, g);
					}
					if g.game_over {
						draw_game_over_overlay(&font, g, &playfield_fx, hud_time);
					} else if g.cleared {
						font.draw(
							"LEVEL 999",
							MARGIN,
							DESIGN_HEIGHT * 0.42,
							28.0,
							theme::CLEAR_TEXT,
						);
						let gr = g.grade_label();
						font.draw(
							&format!("GRADE {gr}"),
							MARGIN,
							DESIGN_HEIGHT * 0.48,
							22.0,
							WHITE,
						);
						{
							let pr = "PRESS ENTER TO RETURN";
							let pw = font.measure(pr, 14.0).width;
							font.draw(
								pr,
								(DESIGN_WIDTH - pw) * 0.5,
								DESIGN_HEIGHT * 0.55,
								14.0,
								theme::TEXT_MUTED,
							);
						}
					}
				}
			}
			ClientState::Playing => {
				if let Some(ref g) = game {
					let hud_time = get_time() as f32;
					let hud_jolt = hud_vertical_jolt_from_keys();
					let grade_up_t = grade_up_fx.as_ref().map(|a| a.t01());
					draw_gameplay_layer(
						&font,
						g,
						&play_options,
						&playfield_fx,
						&hud_rot_feel,
						hud_time,
						hud_jolt,
						g.options.big,
						grade_up_t,
					);
					hud::draw_right_rail(
						&font,
						RIGHT_RAIL_X,
						hud_time,
						if playing_autoplay {
							"Autoplay"
						} else {
							"Free Play"
						},
						None,
						right_rail_panel_y() + hud_jolt,
					);
					if debug_overlay {
						draw_debug(&font, g);
					}
					if g.cleared {
						font.draw(
							"LEVEL 999",
							MARGIN,
							DESIGN_HEIGHT * 0.42,
							28.0,
							theme::CLEAR_TEXT,
						);
						let gr = g.grade_label();
						font.draw(
							&format!("GRADE {gr}"),
							MARGIN,
							DESIGN_HEIGHT * 0.48,
							22.0,
							WHITE,
						);
					}
				}
			}
			ClientState::ReplayList => {
				draw_replay_list(&font, replay_list_scroll, &replay_list_entries);
			}
			ClientState::ReplayPlayback => {
				if let Some(rw) = replay_watch.as_ref() {
					let g = &rw.game;
					let opts = g.options;
					let hud_time = get_time() as f32;
					let last_inp = rw.idx.checked_sub(1).map(|i| rw.inputs[i]);
					let hud_jolt = hud_vertical_jolt_from_replay_byte(last_inp);
					let grade_up_t = grade_up_fx.as_ref().map(|a| a.t01());
					draw_gameplay_layer(
						&font,
						g,
						&opts,
						&playfield_fx,
						&hud_rot_feel,
						hud_time,
						hud_jolt,
						opts.big,
						grade_up_t,
					);
					let prog = format!("frame {} / {}", rw.idx, rw.inputs.len());
					hud::draw_right_rail(
						&font,
						RIGHT_RAIL_X,
						hud_time,
						"REPLAY",
						Some(&prog),
						right_rail_panel_y() + hud_jolt,
					);
					if debug_overlay {
						draw_debug(&font, g);
					}
					font.draw(
						"P pause  . step  drag timeline",
						MARGIN,
						36.0,
						12.0,
						theme::TEXT_HELP,
					);
					if rw.paused {
						font.draw(
							"PAUSED",
							MARGIN,
							52.0,
							14.0,
							Color::from_rgba(200, 200, 100, 255),
						);
					}
					let done = rw.idx >= rw.inputs.len() || g.game_over || g.cleared;
					if done {
						font.draw(
							"END - ESC: LIST",
							MARGIN,
							DESIGN_HEIGHT * 0.45,
							18.0,
							theme::TEXT_MUTED,
						);
					}
					font.draw(
						"ESC: REPLAY LIST",
						MARGIN,
						DESIGN_HEIGHT - 38.0,
						12.0,
						theme::TEXT_HELP,
					);
					draw_replay_timeline(&font, rw);
				}
			}
			ClientState::HighScores => {
				draw_high_scores(&font, &load_highscores());
			}
			ClientState::Settings => {
				draw_settings_screen(
					&font,
					settings_menu_idx,
					last_persisted_size.0,
					last_persisted_size.1,
					fullscreen,
				);
			}
			ClientState::BgAnimTest => {
				draw_bg_anim_test(&font, bg_anim_idx, bg_anim_test_stack);
			}
		}

		draw_screen_fade_overlay(&screen_transition);

		set_default_camera();

		clear_background(LETTERBOX);
		let sw = screen_width();
		let sh = screen_height();
		let scale = (sw / DESIGN_WIDTH).min(sh / DESIGN_HEIGHT);
		let vw = DESIGN_WIDTH * scale;
		let vh = DESIGN_HEIGHT * scale;
		let ox = (sw - vw) * 0.5;
		let oy = (sh - vh) * 0.5;
		let death_strength = match client_state {
			ClientState::Playing | ClientState::PostGame | ClientState::ReplayPlayback => {
				playfield_fx.death_rust_amount()
			}
			_ => 0.0,
		};
		let (sonic_blur, piece_mino_uvs) = match &client_state {
			ClientState::Playing | ClientState::PostGame => {
				if let Some(g) = game.as_ref() {
					if g.piece.is_some() && playfield_fx.sonic_slam_blur(g.level) > 0.001 {
						(
							playfield_fx.sonic_slam_blur(g.level),
							active_piece_mino_uvs(g, &playfield_fx),
						)
					} else {
						(0.0, [0.0f32; 16])
					}
				} else {
					(0.0, [0.0f32; 16])
				}
			}
			ClientState::ReplayPlayback => {
				if let Some(rw) = replay_watch.as_ref() {
					let g = &rw.game;
					if g.piece.is_some() && playfield_fx.sonic_slam_blur(g.level) > 0.001 {
						(
							playfield_fx.sonic_slam_blur(g.level),
							active_piece_mino_uvs(g, &playfield_fx),
						)
					} else {
						(0.0, [0.0f32; 16])
					}
				} else {
					(0.0, [0.0f32; 16])
				}
			}
			_ => (0.0, [0.0f32; 16]),
		};
		screen_fx.draw_composite(
			&design_rt.texture,
			ox,
			oy,
			vw,
			vh,
			DESIGN_WIDTH,
			DESIGN_HEIGHT,
			&screen_juice,
			death_strength,
			sonic_blur,
			&piece_mino_uvs,
		);

		if !fullscreen {
			let sw = screen_width() as u32;
			let sh = screen_height() as u32;
			if (sw, sh) != last_persisted_size {
				last_persisted_size = (sw, sh);
				let _ = save_client_settings(&ClientSettings {
					window_width: sw,
					window_height: sh,
					fullscreen: false,
				});
			}
		}

		next_frame().await;
	}
}

fn format_replay_played_ms(ms: u64) -> String {
	if ms == 0 {
		return "—".to_string();
	}
	DateTime::<Utc>::from_timestamp_millis(ms as i64)
		.map(|dt| {
			dt.with_timezone(&Local)
				.format("%Y-%m-%d %H:%M")
				.to_string()
		})
		.unwrap_or_else(|| "—".to_string())
}

fn draw_replay_list(font: &ArcadeFont, scroll: usize, entries: &[ReplayListEntry]) {
	let title = "REPLAYS";
	let tw = font.measure(title, 28.0).width;
	let panel_h = DESIGN_HEIGHT - MARGIN * 2.0 - 20.0;
	theme::draw_panel(MARGIN, 32.0, DESIGN_WIDTH - MARGIN * 2.0, panel_h);
	font.draw(
		title,
		(DESIGN_WIDTH - tw) * 0.5,
		52.0,
		28.0,
		theme::TITLE_LINE,
	);
	font.draw(
		"UP/DOWN  ENTER  ESC",
		MARGIN + 12.0,
		DESIGN_HEIGHT - 40.0,
		14.0,
		theme::TEXT_HELP,
	);
	font.draw(
		"  PLAYED          GRADE  LEVEL    SCORE",
		MARGIN + 16.0,
		86.0,
		12.0,
		theme::TEXT_MUTED,
	);
	if entries.is_empty() {
		font.draw(
			"No replays yet - finish a run to record one",
			MARGIN + 16.0,
			120.0,
			14.0,
			theme::TEXT_MUTED,
		);
		return;
	}
	let start = scroll.saturating_sub(5);
	let y0 = 100.0_f32;
	for (i, (row, entry)) in entries.iter().enumerate().skip(start).take(14).enumerate() {
		let y = y0 + i as f32 * 26.0;
		let played = format_replay_played_ms(entry.display_ms);
		let (gr, lv_part, sc) = match &entry.summary {
			Some(s) => (
				s.grade.as_str(),
				format!("Lv {:>3}", s.level),
				format!("{:>10}", s.score),
			),
			None => ("—", "Lv   —".to_string(), format!("{:>10}", "—")),
		};
		let prefix = if row == scroll { "> " } else { "  " };
		let label = format!("{}{:16}  {:>4}  {:8}  {}", prefix, played, gr, lv_part, sc);
		let col = if row == scroll {
			WHITE
		} else {
			theme::TEXT_MUTED
		};
		font.draw(&label, MARGIN + 16.0, y, 14.0, col);
	}
}

fn draw_bg_anim_test(font: &ArcadeFont, idx: usize, stack_t: f32) {
	let n = ClockworkBackground::ALL.len();
	let idx = idx.min(n.saturating_sub(1));
	let bg = ClockworkBackground::ALL[idx];
	let label = bg.label();

	draw_rectangle(
		0.0,
		0.0,
		DESIGN_WIDTH,
		DESIGN_HEIGHT,
		Color::from_rgba(0, 0, 0, 120),
	);
	let title = "BACKGROUND ANIMATIONS";
	let tw = font.measure(title, 22.0).width;
	font.draw(
		title,
		(DESIGN_WIDTH - tw) * 0.5,
		36.0,
		22.0,
		theme::TITLE_LINE,
	);
	let sub = format!("{} / {}", idx + 1, n);
	let sw = font.measure(&sub, 14.0).width;
	font.draw(
		&sub,
		(DESIGN_WIDTH - sw) * 0.5,
		66.0,
		14.0,
		theme::TEXT_MUTED,
	);
	let lw = font.measure(label, 14.0).width;
	font.draw(label, (DESIGN_WIDTH - lw) * 0.5, 92.0, 14.0, WHITE);

	let stack_t = stack_t.clamp(0.0, 1.0);
	let mult = 1.0 + stack_t * 2.0;
	let (bx, by, bw, bh) = bg_test_stack_slider_geom();
	let track = Color::from_rgba(18, 18, 26, 255);
	let fill = Color::from_rgba(90, 120, 70, 255);
	let border = theme::PANEL_BORDER;
	let knob = Color::from_rgba(240, 240, 250, 255);
	font.draw(
		"Stack height (test) — animation speed",
		MARGIN,
		BG_TEST_SLIDER_Y - 18.0,
		12.0,
		theme::TEXT_MUTED,
	);
	draw_rectangle(bx, by, bw, bh, track);
	draw_rectangle(bx, by, bw * stack_t, bh, fill);
	draw_rectangle_lines(bx, by, bw, bh, 1.5, border);
	let kx = bx + stack_t * bw;
	draw_line(kx, by, kx, by + bh, 2.5, knob);
	let speed_lbl = format!("{mult:.2}× (1×–3×)");
	let slw = font.measure(&speed_lbl, 12.0).width;
	font.draw(
		&speed_lbl,
		bx + bw - slw,
		BG_TEST_SLIDER_Y - 18.0,
		12.0,
		theme::GRADE_VALUE,
	);

	font.draw(
		"LEFT / RIGHT   A / D   DRAG SLIDER   ESC: SETTINGS",
		MARGIN,
		DESIGN_HEIGHT - 36.0,
		12.0,
		theme::TEXT_HELP,
	);
}

fn draw_settings_screen(
	font: &ArcadeFont,
	menu_idx: usize,
	window_w: u32,
	window_h: u32,
	fullscreen: bool,
) {
	let panel_h = DESIGN_HEIGHT - MARGIN * 2.0 - 20.0;
	theme::draw_panel(MARGIN, 32.0, DESIGN_WIDTH - MARGIN * 2.0, panel_h);
	let title = "SETTINGS";
	let tw = font.measure(title, 28.0).width;
	font.draw(
		title,
		(DESIGN_WIDTH - tw) * 0.5,
		52.0,
		28.0,
		theme::TITLE_LINE,
	);

	let mode = if fullscreen {
		"Fullscreen"
	} else {
		"Windowed"
	};
	let res_line = format!("{window_w}×{window_h}  ({mode})");
	font.draw(&res_line, MARGIN + 16.0, 90.0, 14.0, WHITE);
	font.draw(
		"F8: WINDOW PRESET   F11 / ALT+ENTER: TOGGLE FULLSCREEN",
		MARGIN + 16.0,
		108.0,
		11.0,
		theme::TEXT_HELP,
	);

	let line_h = 24.0;
	let start_y = 142.0;
	let sz = 16.0;
	for (i, label) in SETTINGS_MENU_ITEMS.iter().enumerate() {
		let sel = i == menu_idx;
		let col = if sel { WHITE } else { theme::TEXT_MUTED };
		let prefix = if sel { ">" } else { " " };
		let line = format!("{prefix} {label}");
		let lw = font.measure(&line, sz).width;
		font.draw(&line, (DESIGN_WIDTH - lw) * 0.5, start_y + i as f32 * line_h, sz, col);
	}

	font.draw(
		"UP/DOWN  ENTER  ESC: TITLE",
		MARGIN + 12.0,
		DESIGN_HEIGHT - 40.0,
		14.0,
		theme::TEXT_HELP,
	);
}

fn draw_high_scores(font: &ArcadeFont, h: &HighScoresFile) {
	let title = "HIGH SCORES";
	let tw = font.measure(title, 28.0).width;
	let panel_h = DESIGN_HEIGHT - MARGIN * 2.0 - 20.0;
	theme::draw_panel(MARGIN, 32.0, DESIGN_WIDTH - MARGIN * 2.0, panel_h);
	font.draw(
		title,
		(DESIGN_WIDTH - tw) * 0.5,
		52.0,
		28.0,
		theme::TITLE_LINE,
	);
	font.draw(
		"ESC: TITLE",
		MARGIN + 12.0,
		DESIGN_HEIGHT - 28.0,
		14.0,
		theme::TEXT_HELP,
	);
	if h.entries.is_empty() {
		font.draw(
			"No scores yet - eligible runs (no hidden modes) are saved",
			MARGIN + 16.0,
			120.0,
			14.0,
			theme::TEXT_MUTED,
		);
		return;
	}
	let mut y = 100.0_f32;
	for (i, e) in h.entries.iter().enumerate() {
		let gm = if e.gm { " GM" } else { "" };
		let line = format!(
			"{:2}. {:>10}  {:4}  Lv {:3}{}",
			i + 1,
			e.score,
			e.grade,
			e.level,
			gm
		);
		font.draw(&line, MARGIN + 16.0, y, 14.0, WHITE);
		y += 24.0;
	}
}

fn key_just_pressed(prev: &HashSet<KeyCode>, down: &HashSet<KeyCode>, k: KeyCode) -> bool {
	down.contains(&k) && !prev.contains(&k)
}

fn poll_title_input(buf: &mut Vec<TitleToken>, prev: &HashSet<KeyCode>, down: &HashSet<KeyCode>) {
	let mut push = |t: TitleToken| {
		if buf.len() >= TITLE_BUFFER_CAP {
			buf.remove(0);
		}
		buf.push(t);
	};
	let edge = |k: KeyCode| key_just_pressed(prev, down, k);

	if edge(KeyCode::W) {
		push(TitleToken::U);
	}
	if edge(KeyCode::S) {
		push(TitleToken::D);
	}
	if edge(KeyCode::A) || edge(KeyCode::Left) {
		push(TitleToken::L);
	}
	if edge(KeyCode::D) || edge(KeyCode::Right) {
		push(TitleToken::R);
	}
	if edge(KeyCode::J) {
		push(TitleToken::A);
	}
	if edge(KeyCode::K) {
		push(TitleToken::B);
	}
	if edge(KeyCode::L) {
		push(TitleToken::C);
	}
}

#[derive(Default)]
struct AutoplayBot {
	queue: VecDeque<Input>,
	greedy: AutoplayGreedy,
}

/// One-step greedy toward a heuristic target when BFS planning is unavailable (big/reverse) or
/// fails.
#[derive(Default)]
struct AutoplayGreedy {
	had_piece: bool,
	target_x: i32,
	target_rot: u8,
}

impl AutoplayGreedy {
	fn reset(&mut self) {
		self.had_piece = false;
	}

	fn pick_input(&mut self, g: &Game) -> Input {
		let Some(p) = g.piece else {
			self.had_piece = false;
			return Input::default();
		};

		let new_piece = !self.had_piece;
		self.had_piece = true;
		if new_piece {
			if g.options.big || g.options.reverse {
				self.target_x = 3;
				self.target_rot = 0;
			} else {
				let (tx, tr) = best_placement(&g.board, p.kind);
				self.target_x = tx;
				self.target_rot = tr;
			}
		}

		if p.rot != self.target_rot {
			let use_cw = prefer_cw_rot(p.kind, p.rot, self.target_rot);
			return Input {
				rot_cw: use_cw,
				rot_ccw: !use_cw,
				..Default::default()
			};
		}

		if p.x < self.target_x {
			return Input {
				right: true,
				..Default::default()
			};
		}
		if p.x > self.target_x {
			return Input {
				left: true,
				..Default::default()
			};
		}

		Input {
			down: true,
			sonic: true,
			..Default::default()
		}
	}
}

impl AutoplayBot {
	fn reset(&mut self) {
		self.queue.clear();
		self.greedy.reset();
	}

	fn pick_input(&mut self, g: &Game) -> Input {
		if g.phase != Phase::Falling {
			if g.piece.is_none() {
				self.queue.clear();
				self.greedy.reset();
			}
			return Input::default();
		}

		if g.options.big || g.options.reverse {
			return self.greedy.pick_input(g);
		}

		if g.piece.is_none() {
			self.queue.clear();
			return Input::default();
		}

		if self.queue.is_empty() {
			if let Some(path) = autoplay_plan_inputs(g) {
				self.queue.extend(path);
				self.greedy.reset();
			} else {
				return self.greedy.pick_input(g);
			}
		}

		self.queue
			.pop_front()
			.unwrap_or_else(|| self.greedy.pick_input(g))
	}
}

fn prefer_cw_rot(kind: PieceKind, from: u8, to: u8) -> bool {
	let mut cw = 0u8;
	let mut r = from;
	while r != to && cw < 6 {
		r = rotate_cw(kind, r);
		cw += 1;
	}
	let cw_ok = r == to;
	let mut ccw = 0u8;
	r = from;
	while r != to && ccw < 6 {
		r = rotate_ccw(kind, r);
		ccw += 1;
	}
	let ccw_ok = r == to;
	match (cw_ok, ccw_ok) {
		(true, false) => true,
		(false, true) => false,
		(true, true) => cw <= ccw,
		_ => true,
	}
}

fn landing_py(board: &Board, px: i32, kind: PieceKind, rot: u8) -> Option<i32> {
	let mut py = BOARD_HEIGHT as i32 + 4;
	for _ in 0..64 {
		if !board.collides(px, py, kind, rot) {
			return Some(board.drop_to_bottom(px, py, kind, rot));
		}
		py -= 1;
	}
	None
}

/// Line clears first (tetris over triple), then lower stack heuristic.
fn evaluate_placement(board: &Board, px: i32, py: i32, kind: PieceKind, rot: u8) -> (u32, i32) {
	let mut b = board.clone();
	let color = kind as u8 + 1;
	b.lock_piece(px, py, kind, rot, color);
	let full = find_full_lines(&b);
	let n = count_full_lines(&full);
	if n > 0 {
		clear_lines(&mut b, &full);
	}
	(n, board_heuristic(&b))
}

fn board_heuristic(board: &Board) -> i32 {
	let mut col_h = [0i32; BOARD_WIDTH];
	for x in 0..BOARD_WIDTH {
		let mut h = 0;
		for y in 0..BOARD_HEIGHT {
			if board.rows[y][x] != EMPTY {
				h = (y + 1) as i32;
			}
		}
		col_h[x] = h;
	}
	let agg: i32 = col_h.iter().sum();
	let bumps: i32 = col_h.windows(2).map(|w| (w[0] - w[1]).abs()).sum();
	let holes = count_holes_board(board);
	agg * 10 + holes * 40 + bumps * 2
}

fn count_holes_board(board: &Board) -> i32 {
	let mut n = 0;
	for x in 0..BOARD_WIDTH {
		let mut seen = false;
		for y in (0..BOARD_HEIGHT).rev() {
			if board.rows[y][x] != EMPTY {
				seen = true;
			} else if seen {
				n += 1;
			}
		}
	}
	n
}

fn best_placement(board: &Board, kind: PieceKind) -> (i32, u8) {
	let mut best: Option<(u32, i32)> = None;
	let mut best_x = 3i32;
	let mut best_r = 0u8;
	for rot in 0u8..4 {
		for px in -4i32..(BOARD_WIDTH as i32 + 4) {
			let Some(py) = landing_py(board, px, kind, rot) else {
				continue;
			};
			let (lines, h) = evaluate_placement(board, px, py, kind, rot);
			let better = match best {
				None => true,
				Some((bl, bh)) => lines > bl || (lines == bl && h < bh),
			};
			if better {
				best = Some((lines, h));
				best_x = px;
				best_r = rot;
			}
		}
	}
	(best_x, best_r)
}

fn draw_game_over_overlay(font: &ArcadeFont, g: &Game, fx: &PlayfieldFx, hud_time: f32) {
	let death_t = (fx.death_frames as f32 / DEATH_FRAMES_MAX as f32).min(1.0);
	let fade = (0.1 + 0.9 * death_t).min(1.0);

	let pulse = (hud_time * 2.8).sin() * 0.5 + 0.5;
	let head_lo = Color::from_rgba(140, 28, 40, 255);
	let head_hi = Color::from_rgba(255, 72, 80, 255);
	let head_color = Color::new(
		head_lo.r + (head_hi.r - head_lo.r) * pulse,
		head_lo.g + (head_hi.g - head_lo.g) * pulse,
		head_lo.b + (head_hi.b - head_lo.b) * pulse,
		(head_lo.a + (head_hi.a - head_lo.a) * pulse) * fade,
	);

	let head = "GAME OVER";
	let sz_head = 30.0;
	let stats = format!("SCORE {}   LV {}", g.score, g.level);
	let sz_stat = 16.0;
	let prompt = "PRESS ENTER TO RETURN";
	let sz_prompt = 14.0;

	let w_head = font.measure(head, sz_head).width;
	let w_stat = font.measure(&stats, sz_stat).width;
	let w_prompt = font.measure(prompt, sz_prompt).width;
	let block_w = w_head.max(w_stat).max(w_prompt);
	let pad_x = 28.0;
	let pad_y = 22.0;
	let line_gap = 12.0;
	let line_gap2 = 24.0;

	let m0 = font.measure(head, sz_head);
	let m1 = font.measure(&stats, sz_stat);
	let block_h =
		m0.height + line_gap + m1.height + line_gap2 + font.measure(prompt, sz_prompt).height;

	let bob = (hud_time * 1.8).sin() * 1.5;
	let panel_w = block_w + pad_x * 2.0;
	let panel_h = block_h + pad_y * 2.0;
	let panel_x = (DESIGN_WIDTH - panel_w) * 0.5;
	let y_panel_top = DESIGN_HEIGHT * 0.42 - pad_y + bob;

	let panel_bg = Color::new(0.03, 0.03, 0.055, 0.62 * fade);
	draw_rectangle(panel_x, y_panel_top, panel_w, panel_h, panel_bg);
	let border_a = Color::new(
		PANEL_BORDER.r,
		PANEL_BORDER.g,
		PANEL_BORDER.b,
		PANEL_BORDER.a * 0.55 * fade,
	);
	draw_rectangle_lines(panel_x, y_panel_top, panel_w, panel_h, 1.2, border_a);

	let mut y = y_panel_top + pad_y + m0.offset_y;
	font.draw(head, (DESIGN_WIDTH - w_head) * 0.5, y, sz_head, head_color);
	y += m0.height + line_gap;
	let stat_col = Color::new(
		theme::HUD_LABEL.r,
		theme::HUD_LABEL.g,
		theme::HUD_LABEL.b,
		theme::HUD_LABEL.a * fade,
	);
	font.draw(&stats, (DESIGN_WIDTH - w_stat) * 0.5, y, sz_stat, stat_col);
	y += m1.height + line_gap2;

	let blink = (get_time() * 2.0) as i64 % 2 == 0;
	if blink {
		let prompt_col = Color::new(
			theme::TEXT_MUTED.r,
			theme::TEXT_MUTED.g,
			theme::TEXT_MUTED.b,
			theme::TEXT_MUTED.a * fade,
		);
		font.draw(
			prompt,
			(DESIGN_WIDTH - w_prompt) * 0.5,
			y,
			sz_prompt,
			prompt_col,
		);
	}
}

fn draw_title_screen(font: &ArcadeFont, menu_idx: usize) {
	let title = "VIBE CODED TGM";
	let tw = font.measure(title, 22.0).width;
	font.draw(
		title,
		(DESIGN_WIDTH - tw) * 0.5,
		DESIGN_HEIGHT * 0.28,
		22.0,
		theme::TITLE_LINE,
	);

	let hint = "UP / DOWN    ENTER";
	let hw = font.measure(hint, 12.0).width;
	font.draw(
		hint,
		(DESIGN_WIDTH - hw) * 0.5,
		DESIGN_HEIGHT * 0.345,
		12.0,
		theme::TEXT_MUTED,
	);

	let line_h = 22.0;
	let start_y = DESIGN_HEIGHT * 0.40;
	let mut y = start_y;
	let sz = 16.0;
	for (i, label) in TITLE_MENU_ITEMS.iter().enumerate() {
		let sel = i == menu_idx;
		let col = if sel { WHITE } else { theme::TEXT_MUTED };
		let prefix = if sel { ">" } else { " " };
		let line = format!("{prefix} {label}");
		let lw = font.measure(&line, sz).width;
		font.draw(&line, (DESIGN_WIDTH - lw) * 0.5, y, sz, col);
		y += line_h;
	}

	font.draw(
		"F11 / ALT+ENTER: FULLSCREEN",
		MARGIN,
		DESIGN_HEIGHT - 72.0,
		10.0,
		theme::TEXT_HELP,
	);
}

fn poll_input(phase: Phase, pending_cw: &mut u8, pending_ccw: &mut u8) -> Input {
	let use_hold = matches!(phase, Phase::Are);
	let (rot_cw, rot_ccw) = if use_hold {
		(
			is_key_down(KeyCode::K),
			is_key_down(KeyCode::J) || is_key_down(KeyCode::L),
		)
	} else {
		let rot_ccw = if *pending_ccw > 0 {
			*pending_ccw -= 1;
			true
		} else {
			false
		};
		let rot_cw = if rot_ccw {
			false
		} else if *pending_cw > 0 {
			*pending_cw -= 1;
			true
		} else {
			false
		};
		(rot_cw, rot_ccw)
	};
	Input {
		left: is_key_down(KeyCode::A),
		right: is_key_down(KeyCode::D),
		down: is_key_down(KeyCode::S),
		sonic: is_key_down(KeyCode::W),
		rot_cw,
		rot_ccw,
	}
}

fn active_piece_color(base: Color, in_lock_delay: bool) -> Color {
	if !in_lock_delay {
		return base;
	}
	let pulse = (get_time() as f32 * 9.0).sin() * 0.5 + 0.5;
	Color::new(
		(base.r + 0.14 * pulse).min(1.0),
		(base.g + 0.10 * pulse).min(1.0),
		(base.b + 0.06 * pulse).min(1.0),
		0.52 + pulse * 0.46,
	)
}

/// Vertical padding (pixels) for sonic blur taps around each mino.
const SONIC_BLUR_PAD_PX: f32 = 8.0;

/// Four mino bounding boxes in normalized design UV (xy = min, zw = max) — post blur mask.
fn active_piece_mino_uvs(game: &Game, fx: &PlayfieldFx) -> [f32; 16] {
	let Some(p) = game.piece else {
		return [0.0; 16];
	};
	let (sx, sy) = fx.death_shake();
	let ox = FIELD_OX_BASE + sx;
	let big = game.options.big;
	let cell = if big { BIG_CELL } else { CELL };
	let slam_y = fx.sonic_slam_screen_y(cell);
	let mut out = [0.0f32; 16];
	if big {
		let def = piece_cells_big(p.kind, p.rot);
		for (i, &(dx, dy)) in def.cells.iter().enumerate() {
			let bx = p.x + dx as i32;
			let by = p.y + dy as i32;
			let px = ox + bx as f32 * cell;
			let py = board_screen_y_big(by) + sy + slam_y;
			let px0 = px;
			let px1 = px + cell;
			let py0 = (py - SONIC_BLUR_PAD_PX).max(0.0);
			let py1 = (py + cell + SONIC_BLUR_PAD_PX).min(DESIGN_HEIGHT);
			out[i * 4] = px0 / DESIGN_WIDTH;
			out[i * 4 + 1] = py0 / DESIGN_HEIGHT;
			out[i * 4 + 2] = px1 / DESIGN_WIDTH;
			out[i * 4 + 3] = py1 / DESIGN_HEIGHT;
		}
	} else {
		let def = piece_cells(p.kind, p.rot);
		for (i, &(dx, dy)) in def.cells.iter().enumerate() {
			let bx = p.x + dx as i32;
			let by = p.y + dy as i32;
			let px = ox + bx as f32 * cell;
			let py = board_screen_y(by) + sy + slam_y;
			let px0 = px;
			let px1 = px + cell;
			let py0 = (py - SONIC_BLUR_PAD_PX).max(0.0);
			let py1 = (py + cell + SONIC_BLUR_PAD_PX).min(DESIGN_HEIGHT);
			out[i * 4] = px0 / DESIGN_WIDTH;
			out[i * 4 + 1] = py0 / DESIGN_HEIGHT;
			out[i * 4 + 2] = px1 / DESIGN_WIDTH;
			out[i * 4 + 3] = py1 / DESIGN_HEIGHT;
		}
	}
	out
}

fn board_screen_y(row: i32) -> f32 {
	FIELD_TOP + (VISIBLE_ROWS as f32 - 1.0 - row as f32) * CELL
}

fn board_screen_y_big(row: i32) -> f32 {
	FIELD_TOP + (BIG_VISIBLE_ROWS as f32 - 1.0 - row as f32) * BIG_CELL
}

/// 0 = locked stack low, 1 = stack near top. Ignores the active piece (locked minos only).
fn bg_danger_proximity(game: &Game) -> f32 {
	let mut top_row = i32::MIN;
	if game.options.big {
		for y in 0..BIG_BOARD_HEIGHT {
			for x in 0..BIG_BOARD_WIDTH {
				if game.board_big.rows[y][x] != EMPTY {
					top_row = top_row.max(y as i32);
				}
			}
		}
	} else {
		for y in 0..BOARD_HEIGHT {
			for x in 0..BOARD_WIDTH {
				if game.board.rows[y][x] != EMPTY {
					top_row = top_row.max(y as i32);
				}
			}
		}
	}
	if top_row == i32::MIN {
		return 0.0;
	}
	let span = if game.options.big {
		(BIG_VISIBLE_ROWS - 1).max(1) as f32
	} else {
		(VISIBLE_ROWS - 1).max(1) as f32
	};
	(top_row as f32 / span).clamp(0.0, 1.0)
}

fn tls_show_ghost(game: &Game) -> bool {
	game.options.tls_always || game.level <= TLS_MAX_LEVEL
}

/// Outline + thickness for the landing ghost (TLS). Slow-gravity band 0..=100 needs more contrast.
fn ghost_hint_style(game: &Game) -> (Color, f32) {
	if game.level <= TLS_MAX_LEVEL {
		(Color::from_rgba(255, 255, 255, 200), 2.75)
	} else {
		(Color::from_rgba(255, 255, 255, 90), 2.0)
	}
}

/// Playfield, NEXT strip (on top), timer (below frame), then side HUD.
fn draw_gameplay_layer(
	font: &ArcadeFont,
	game: &Game,
	opts: &GameOptions,
	playfield_fx: &PlayfieldFx,
	rot: &HudRotFeel,
	hud_time: f32,
	hud_jolt_y: f32,
	big: bool,
	grade_up_t01: Option<f32>,
) {
	let (sx, sy) = playfield_fx.death_shake();
	let ox = FIELD_OX_BASE + sx;
	if big {
		let fw = BIG_BOARD_WIDTH as f32 * BIG_CELL;
		draw_field_big(game, playfield_fx);
		hud::draw_next_strip(
			font,
			game,
			opts,
			rot,
			hud_time,
			ox,
			fw,
			MARGIN + sy + hud_jolt_y,
			FIELD_TOP + sy + hud_jolt_y,
			true,
		);
		hud::draw_timer_below_field(
			font,
			game,
			ox,
			fw,
			FIELD_TOP + sy + BIG_VISIBLE_ROWS as f32 * BIG_CELL + hud_jolt_y,
		);
		hud::draw_hud_big(font, game, opts, rot, hud_time, hud_jolt_y, grade_up_t01);
	} else {
		let fw = BOARD_WIDTH as f32 * CELL;
		draw_field(game, opts, playfield_fx);
		hud::draw_next_strip(
			font,
			game,
			opts,
			rot,
			hud_time,
			ox,
			fw,
			MARGIN + sy + hud_jolt_y,
			FIELD_TOP + sy + hud_jolt_y,
			false,
		);
		hud::draw_timer_below_field(
			font,
			game,
			ox,
			fw,
			FIELD_TOP + sy + VISIBLE_ROWS as f32 * CELL + hud_jolt_y,
		);
		hud::draw_hud(font, game, opts, rot, hud_time, hud_jolt_y, grade_up_t01);
	}
}

fn draw_field(game: &Game, opts: &GameOptions, fx: &PlayfieldFx) {
	let (sx, sy) = fx.death_shake();
	let ox = FIELD_OX_BASE + sx;
	let mono = opts.monochrome;
	let w = BOARD_WIDTH as f32 * CELL;
	let h = VISIBLE_ROWS as f32 * CELL;
	draw_rectangle(ox, FIELD_TOP + sy, w, h, well_fill_color(mono));

	for y in 0..VISIBLE_ROWS as i32 {
		for x in 0..BOARD_WIDTH as i32 {
			let c = game.board.rows[y as usize][x as usize];
			if c == EMPTY {
				continue;
			}
			let px = ox + x as f32 * CELL;
			let py = board_screen_y(y) + sy;
			let col = dim_stack_cell(cell_color(c, mono));
			let col = fx.apply_stack_color(col, x, y, c);
			draw_cell_beveled(px, py, CELL, col);
		}
	}

	fx.draw_line_clear_normal(game, ox, CELL, |row| board_screen_y(row) + sy);

	if tls_show_ghost(game) {
		let (ghost_col, ghost_thick) = ghost_hint_style(game);
		if let Some(p) = game.piece {
			let gy = if game.options.reverse {
				game.board.rise_to_top(p.x, p.y, p.kind, p.rot)
			} else {
				game.board.drop_to_bottom(p.x, p.y, p.kind, p.rot)
			};
			let def = piece_cells(p.kind, p.rot);
			for (dx, dy) in def.cells {
				let bx = p.x + dx as i32;
				let by = gy + dy as i32;
				if by >= 0 && (by as usize) < VISIBLE_ROWS {
					let px = ox + bx as f32 * CELL;
					let py = board_screen_y(by) + sy;
					draw_rectangle_lines(px, py, CELL - 1.0, CELL - 1.0, ghost_thick, ghost_col);
				}
			}
		}
	}

	if let Some(p) = game.piece {
		let grounded = if game.options.reverse {
			game.board.collides(p.x, p.y + 1, p.kind, p.rot)
		} else {
			game.board.collides(p.x, p.y - 1, p.kind, p.rot)
		};
		let def = piece_cells(p.kind, p.rot);
		let col = active_piece_color(cell_color(p.kind as u8 + 1, mono), grounded);
		for (dx, dy) in def.cells {
			let bx = p.x + dx as i32;
			let by = p.y + dy as i32;
			if by >= 0 && (by as usize) < BOARD_HEIGHT {
				let px = ox + bx as f32 * CELL;
				let py = board_screen_y(by) + sy + fx.sonic_slam_screen_y(CELL);
				draw_cell_beveled(px, py, CELL, col);
			}
		}
	}

	draw_playfield_frame(
		ox,
		FIELD_TOP + sy,
		w,
		h,
		fx.death_frames,
		fx.wall_input.horizontal_activity(),
		fx.wall_input.smooth_x,
	);
}

fn draw_field_big(game: &Game, fx: &PlayfieldFx) {
	let (sx, sy) = fx.death_shake();
	let ox = FIELD_OX_BASE + sx;
	let mono = game.options.monochrome;
	let w = BIG_BOARD_WIDTH as f32 * BIG_CELL;
	let h = BIG_VISIBLE_ROWS as f32 * BIG_CELL;
	draw_rectangle(ox, FIELD_TOP + sy, w, h, well_fill_color(mono));

	for y in 0..BIG_VISIBLE_ROWS as i32 {
		for x in 0..BIG_BOARD_WIDTH as i32 {
			let c = game.board_big.rows[y as usize][x as usize];
			if c == EMPTY {
				continue;
			}
			let px = ox + x as f32 * BIG_CELL;
			let py = board_screen_y_big(y) + sy;
			let col = dim_stack_cell(cell_color(c, mono));
			let col = fx.apply_stack_color(col, x, y, c);
			draw_cell_beveled(px, py, BIG_CELL, col);
		}
	}

	fx.draw_line_clear_big(game, ox, BIG_CELL, |row| board_screen_y_big(row) + sy);

	if tls_show_ghost(game) {
		let (ghost_col, ghost_thick) = ghost_hint_style(game);
		if let Some(p) = game.piece {
			let gy = if game.options.reverse {
				game.board_big.rise_to_top(p.x, p.y, p.kind, p.rot)
			} else {
				game.board_big.drop_to_bottom(p.x, p.y, p.kind, p.rot)
			};
			let def = piece_cells_big(p.kind, p.rot);
			for (dx, dy) in def.cells {
				let bx = p.x + dx as i32;
				let by = gy + dy as i32;
				if by >= 0 && (by as usize) < BIG_VISIBLE_ROWS {
					let px = ox + bx as f32 * BIG_CELL;
					let py = board_screen_y_big(by) + sy;
					draw_rectangle_lines(
						px,
						py,
						BIG_CELL - 1.0,
						BIG_CELL - 1.0,
						ghost_thick,
						ghost_col,
					);
				}
			}
		}
	}

	if let Some(p) = game.piece {
		let grounded = if game.options.reverse {
			game.board_big.collides(p.x, p.y + 1, p.kind, p.rot)
		} else {
			game.board_big.collides(p.x, p.y - 1, p.kind, p.rot)
		};
		let def = piece_cells_big(p.kind, p.rot);
		let col = active_piece_color(cell_color(p.kind as u8 + 1, mono), grounded);
		for (dx, dy) in def.cells {
			let bx = p.x + dx as i32;
			let by = p.y + dy as i32;
			if by >= 0 && (by as usize) < BIG_BOARD_HEIGHT {
				let px = ox + bx as f32 * BIG_CELL;
				let py = board_screen_y_big(by) + sy + fx.sonic_slam_screen_y(BIG_CELL);
				draw_cell_beveled(px, py, BIG_CELL, col);
			}
		}
	}

	draw_playfield_frame(
		ox,
		FIELD_TOP + sy,
		w,
		h,
		fx.death_frames,
		fx.wall_input.horizontal_activity(),
		fx.wall_input.smooth_x,
	);
}

fn draw_debug(font: &ArcadeFont, game: &Game) {
	let hx = RIGHT_RAIL_X;
	let y = DESIGN_HEIGHT - 160.0;
	let phase = match game.phase {
		Phase::Falling => "Falling",
		Phase::LineClear => "LineClear",
		Phase::Are => "ARE",
	};
	let c = Color::from_rgba(120, 255, 160, 255);
	let mut ly = y;
	font.draw(
		&format!("frame {}  phase {}", game.frame, phase),
		hx,
		ly,
		10.0,
		c,
	);
	ly += 12.0;
	font.draw(
		&format!(
			"lock {}  das L{} R{}",
			game.lock_delay, game.das_left, game.das_right
		),
		hx,
		ly,
		10.0,
		c,
	);
	ly += 12.0;
	font.draw(&format!("accum {}", game.gravity_accum), hx, ly, 10.0, c);
}
