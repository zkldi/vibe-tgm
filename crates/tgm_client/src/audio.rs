//! Embedded gameplay audio: dual BGM (level 0–479 early, **silent 480–499**, 500+ late) and SFX.
//!
//! BGM is shipped as **Ogg Opus** (`.opus`) and decoded to WAV at startup to keep the binary small.
//! Re-encode with e.g. `ffmpeg -i bgm1.wav -c:a libopus -b:a 128k bgm1.opus`.
//!
//! Replace files under [`crates/tgm_client/assets/audio/`](../assets/audio/) with your own
//! licensed/original assets. Expected filenames (wired in [`AudioAssets::load`]):
//! - `bgm1_intro.opus` — one-shot intro for early BGM (bar 10 @ 140 BPM from the full track)
//! - `bgm1.opus` — early BGM loop (plays after intro; seamless with intro)
//! - `bgm2.opus` — late-section BGM loop (level 500+ @ **150 BPM** for HUD beat sync)
//! - `lock.wav` — short click on every piece lock (also when lines clear; line SFX layer on top)
//! - `sonic.wav`
//! - `next_i.wav` … `next_o.wav` — one cue per tetromino for the **NEXT** preview
//!   (`Game::next_kind` on spawn), not the active piece; **I** highest pitch, **O** lowest
//! - `line_1.wav` … `line_4.wav`, `line_uki_1.wav` … `line_uki_4.wav` (uki line-clear set)
//! - `grade_up.wav`, `game_over.wav`, `cleared.wav`
//! - `gate_bell.wav` — three strikes when crossing a hundreds gate (…98→…99, etc.; must clear to
//!   advance)
//! - `menu_move.wav`, `menu_confirm.wav`, `menu_cancel.wav` — UI navigation (title/settings/etc.)
//! - `ready.wav`, `go.wav` — TGM-style start countdown (loaded via [`bitcrush_wav_bytes`])

use std::io::Cursor;

use crate::ogg_opus;
use hound::{SampleFormat, WavReader, WavWriter};
use macroquad::audio::{
	PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
};
use tgm_core::{
	Game, Input, Phase, PieceKind, PieceState, count_full_lines, find_full_lines,
	line_clear_only_for_increment,
};

/// BGM loop volume (SFX use `SFX_VOL`).
pub const BGM_VOL: f32 = 0.35;
/// Early BGM (`bgm1`) tempo — used for HUD beat sync ([`AudioRuntime::bgm1_elapsed_sec`]).
pub const BGM1_BPM: f32 = 140.0;
/// Late BGM (`bgm2`) tempo — used for HUD beat sync ([`AudioRuntime::bgm2_elapsed_sec`]).
pub const BGM2_BPM: f32 = 150.0;
/// Early BGM intro length (seconds): bar **10** @ **140 BPM** in 4/4 (nearest bar to ~00:17.143).
/// `10 × (4 × 60/140) = 120/7`.
const EARLY_BGM_INTRO_SEC: f64 = 120.0 / 7.0;
const SFX_VOL: f32 = 0.55;
/// Piece lock click — subtle so it stacks with line-clear SFX.
const LOCK_CLICK_VOL: f32 = 0.34;
const GATE_BELL_VOL: f32 = 0.48;
const MENU_SFX_VOL: f32 = 0.5;
const READY_GO_VOL: f32 = 0.62;

/// Early BGM stops at this level; no BGM until [`LATE_BGM_LEVEL`].
const BGM_CUTOUT_LEVEL: u16 = 480;
const LATE_BGM_LEVEL: u16 = 500;

/// `480..500` — early BGM has cut out; late BGM not yet playing (intentional silence).
#[inline]
pub fn bgm_silent_zone(level: u16) -> bool {
	level >= BGM_CUTOUT_LEVEL && level < LATE_BGM_LEVEL
}

/// All loaded `Sound` handles (embedded at compile time).
pub struct AudioAssets {
	pub bgm_early_intro: Sound,
	pub bgm_early: Sound,
	pub bgm_late: Sound,
	lock: Sound,
	sonic: Sound,
	next_i: Sound,
	next_t: Sound,
	next_l: Sound,
	next_j: Sound,
	next_s: Sound,
	next_z: Sound,
	next_o: Sound,
	line_1: Sound,
	line_2: Sound,
	line_3: Sound,
	line_4: Sound,
	line_uki_1: Sound,
	line_uki_2: Sound,
	line_uki_3: Sound,
	line_uki_4: Sound,
	grade_up: Sound,
	game_over: Sound,
	cleared: Sound,
	gate_bell: Sound,
	menu_move: Sound,
	menu_confirm: Sound,
	menu_cancel: Sound,
	ready_voice: Sound,
	go_voice: Sound,
}

async fn load_embedded_opus_bgm(bytes: &[u8]) -> Result<Sound, macroquad::Error> {
	let wav = ogg_opus::decode_ogg_opus_to_wav_bytes(bytes).map_err(|_| {
		macroquad::Error::UnknownError("ogg opus decode failed")
	})?;
	load_sound_from_bytes(&wav).await
}

impl AudioAssets {
	/// Load all embedded samples. Call after [`macroquad::prelude::next_frame`].
	pub async fn load() -> Result<Self, macroquad::Error> {
		macro_rules! emb {
			($path:literal) => {
				load_sound_from_bytes(include_bytes!($path)).await?
			};
		}
		Ok(Self {
			bgm_early_intro: load_embedded_opus_bgm(include_bytes!(
				"../assets/audio/bgm1_intro.opus"
			))
			.await?,
			bgm_early: load_embedded_opus_bgm(include_bytes!("../assets/audio/bgm1.opus")).await?,
			bgm_late: load_embedded_opus_bgm(include_bytes!("../assets/audio/bgm2.opus")).await?,
			lock: emb!("../assets/audio/lock.wav"),
			sonic: emb!("../assets/audio/sonic.wav"),
			next_i: emb!("../assets/audio/next_i.wav"),
			next_t: emb!("../assets/audio/next_t.wav"),
			next_l: emb!("../assets/audio/next_l.wav"),
			next_j: emb!("../assets/audio/next_j.wav"),
			next_s: emb!("../assets/audio/next_s.wav"),
			next_z: emb!("../assets/audio/next_z.wav"),
			next_o: emb!("../assets/audio/next_o.wav"),
			line_1: emb!("../assets/audio/line_1.wav"),
			line_2: emb!("../assets/audio/line_2.wav"),
			line_3: emb!("../assets/audio/line_3.wav"),
			line_4: emb!("../assets/audio/line_4.wav"),
			line_uki_1: emb!("../assets/audio/line_uki_1.wav"),
			line_uki_2: emb!("../assets/audio/line_uki_2.wav"),
			line_uki_3: emb!("../assets/audio/line_uki_3.wav"),
			line_uki_4: emb!("../assets/audio/line_uki_4.wav"),
			grade_up: emb!("../assets/audio/grade_up.wav"),
			game_over: emb!("../assets/audio/game_over.wav"),
			cleared: emb!("../assets/audio/cleared.wav"),
			gate_bell: emb!("../assets/audio/gate_bell.wav"),
			menu_move: emb!("../assets/audio/menu_move.wav"),
			menu_confirm: emb!("../assets/audio/menu_confirm.wav"),
			menu_cancel: emb!("../assets/audio/menu_cancel.wav"),
			ready_voice: load_sound_from_bytes(&bitcrush_wav_bytes(include_bytes!(
				"../assets/audio/ready.wav"
			)))
			.await?,
			go_voice: load_sound_from_bytes(&bitcrush_wav_bytes(include_bytes!(
				"../assets/audio/go.wav"
			)))
			.await?,
		})
	}
}

/// Quantize + sample-and-hold for a crunchy arcade announcer.
fn bitcrush_i16(samples: &[i16]) -> Vec<i16> {
	const HOLD: usize = 3;
	const SHIFT: i32 = 9;
	let mut out = Vec::with_capacity(samples.len());
	let mut hold = 0i16;
	for (i, &s) in samples.iter().enumerate() {
		if i % HOLD == 0 {
			let q = (s as i32 >> SHIFT) << SHIFT;
			hold = q.clamp(-32768, 32767) as i16;
		}
		out.push(hold);
	}
	out
}

/// Stretch waveform in time (linear interpolation) so playback at the same sample rate sounds
/// lower-pitched. `stretch` > 1.0 deepens the voice; clip is slightly longer.
fn deepen_voice_i16(samples: &[i16], stretch: f32) -> Vec<i16> {
	let stretch = stretch.clamp(1.0, 1.6);
	if stretch <= 1.0001 {
		return samples.to_vec();
	}
	let n = samples.len();
	if n < 2 {
		return samples.to_vec();
	}
	let out_len = (n as f32 * stretch).round().max(2.0) as usize;
	let mut out = Vec::with_capacity(out_len);
	for j in 0..out_len {
		let pos = j as f32 / stretch;
		let i0 = pos.floor() as usize;
		let i1 = (i0 + 1).min(n - 1);
		let t = pos - i0 as f32;
		let s0 = samples[i0] as f32;
		let s1 = samples[i1] as f32;
		let v = s0 + (s1 - s0) * t;
		out.push(v.clamp(-32768.0, 32767.0) as i16);
	}
	out
}

/// Bitcrush then deepen announcer voice (macroquad has no playback pitch control).
fn bitcrush_wav_bytes(input: &[u8]) -> Vec<u8> {
	const VOICE_DEEPEN_STRETCH: f32 = 1.22;
	let mut reader = WavReader::new(Cursor::new(input)).expect("embedded ready/go wav");
	let spec = reader.spec();
	assert_eq!(spec.sample_format, SampleFormat::Int);
	assert_eq!(spec.bits_per_sample, 16);
	let samples: Vec<i16> = reader
		.samples::<i16>()
		.map(|s| s.expect("sample"))
		.collect();
	let crushed = bitcrush_i16(&samples);
	let out_samples = deepen_voice_i16(&crushed, VOICE_DEEPEN_STRETCH);
	let mut out = Vec::new();
	{
		let mut w = WavWriter::new(Cursor::new(&mut out), spec).expect("wav writer");
		for s in out_samples {
			w.write_sample(s).unwrap();
		}
		w.finalize().unwrap();
	}
	out
}

#[inline]
fn vol_unless_muted(vol: f32, muted: bool) -> f32 {
	if muted {
		0.0
	} else {
		vol
	}
}

/// BGM uses three [`Sound`] handles; [`set_sound_volume`] applies to currently playing instances.
fn set_bgm_track_volumes(a: &AudioAssets, muted: bool) {
	set_bgm_track_volumes_scaled(a, muted, 1.0);
}

fn set_bgm_track_volumes_scaled(a: &AudioAssets, muted: bool, scale: f32) {
	let v = vol_unless_muted(BGM_VOL * scale.clamp(0.0, 1.0), muted);
	set_sound_volume(&a.bgm_early_intro, v);
	set_sound_volume(&a.bgm_early, v);
	set_sound_volume(&a.bgm_late, v);
}

pub fn play_ready_voice(a: &AudioAssets, muted: bool) {
	play_sound_once_with_vol(&a.ready_voice, vol_unless_muted(READY_GO_VOL, muted));
}

pub fn play_go_voice(a: &AudioAssets, muted: bool) {
	play_sound_once_with_vol(&a.go_voice, vol_unless_muted(READY_GO_VOL, muted));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveBgm {
	None,
	Early,
	Late,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EarlyBgmPhase {
	Idle,
	Intro,
	Loop,
}

/// Tracks which BGM loop is playing (including intentional silence 480–499 before late BGM).
pub struct AudioRuntime {
	active_bgm: ActiveBgm,
	early_phase: EarlyBgmPhase,
	early_intro_started: std::time::Instant,
	late_bgm_started: std::time::Instant,
	/// When set, BGM and one-shot SFX use zero volume (F2 toggles in the client).
	pub muted: bool,
}

impl Default for AudioRuntime {
	fn default() -> Self {
		Self::new()
	}
}

impl AudioRuntime {
	pub fn new() -> Self {
		Self {
			active_bgm: ActiveBgm::None,
			early_phase: EarlyBgmPhase::Idle,
			early_intro_started: std::time::Instant::now(),
			late_bgm_started: std::time::Instant::now(),
			muted: false,
		}
	}

	fn tick_early_intro(&mut self, a: &AudioAssets) {
		if self.early_phase != EarlyBgmPhase::Intro {
			return;
		}
		if self.early_intro_started.elapsed().as_secs_f64() < EARLY_BGM_INTRO_SEC {
			return;
		}
		stop_sound(&a.bgm_early_intro);
		play_sound(
			&a.bgm_early,
			PlaySoundParams {
				looped: true,
				volume: vol_unless_muted(BGM_VOL, self.muted),
			},
		);
		self.early_phase = EarlyBgmPhase::Loop;
	}

	pub fn stop_bgm(&mut self, a: &AudioAssets) {
		stop_sound(&a.bgm_early_intro);
		stop_sound(&a.bgm_early);
		stop_sound(&a.bgm_late);
		self.active_bgm = ActiveBgm::None;
		self.early_phase = EarlyBgmPhase::Idle;
	}

	/// Apply [`Self::muted`] and `volume_scale` (0..=1) to the three BGM [`Sound`] handles.
	/// `volume_scale` is typically `1.0`, or ramps down during death (see [`crate::playfield_fx`]).
	/// Call after toggling mute, when [`Self::sync_bgm_for_level`] is skipped for a frame, or
	/// after `sync_bgm_for_level` each frame.
	pub fn apply_bgm_volume_scale(&self, a: &AudioAssets, volume_scale: f32) {
		set_bgm_track_volumes_scaled(a, self.muted, volume_scale);
	}

	/// Keep BGM in sync with `level` (early below 480, **none** 480–499, late 500+).
	pub fn sync_bgm_for_level(&mut self, level: u16, a: &AudioAssets) {
		let want = if level >= LATE_BGM_LEVEL {
			ActiveBgm::Late
		} else if level >= BGM_CUTOUT_LEVEL {
			ActiveBgm::None
		} else {
			ActiveBgm::Early
		};
		if self.active_bgm == want {
			if want == ActiveBgm::Early {
				self.tick_early_intro(a);
			}
			set_bgm_track_volumes(a, self.muted);
			return;
		}
		stop_sound(&a.bgm_early_intro);
		stop_sound(&a.bgm_early);
		stop_sound(&a.bgm_late);
		self.active_bgm = want;
		self.early_phase = EarlyBgmPhase::Idle;
		match want {
			ActiveBgm::Early => {
				self.early_intro_started = std::time::Instant::now();
				self.early_phase = EarlyBgmPhase::Intro;
				play_sound(
					&a.bgm_early_intro,
					PlaySoundParams {
						looped: false,
						volume: vol_unless_muted(BGM_VOL, self.muted),
					},
				);
			}
			ActiveBgm::Late => {
				self.late_bgm_started = std::time::Instant::now();
				play_sound(
					&a.bgm_late,
					PlaySoundParams {
						looped: true,
						volume: vol_unless_muted(BGM_VOL, self.muted),
					},
				);
			}
			ActiveBgm::None => {}
		}
		set_bgm_track_volumes(a, self.muted);
	}

	/// Seconds since early BGM (`bgm1` intro + loop) started, if that track is active.
	/// Use with [`crate::hud::beat_stress_bgm1`] for 140 BPM UI pulses.
	pub fn bgm1_elapsed_sec(&self) -> Option<f64> {
		if self.active_bgm != ActiveBgm::Early {
			return None;
		}
		Some(self.early_intro_started.elapsed().as_secs_f64())
	}

	/// Seconds since late BGM (`bgm2`) started, if that track is active.
	/// Use with [`crate::hud::beat_stress_bgm2`] for 150 BPM UI pulses.
	pub fn bgm2_elapsed_sec(&self) -> Option<f64> {
		if self.active_bgm != ActiveBgm::Late {
			return None;
		}
		Some(self.late_bgm_started.elapsed().as_secs_f64())
	}
}

pub fn play_grade_up(a: &AudioAssets, muted: bool) {
	play_sound_once_with_vol(&a.grade_up, vol_unless_muted(SFX_VOL, muted));
}

pub fn play_menu_move(a: &AudioAssets, muted: bool) {
	play_sound_once_with_vol(&a.menu_move, vol_unless_muted(MENU_SFX_VOL, muted));
}

pub fn play_menu_confirm(a: &AudioAssets, muted: bool) {
	play_sound_once_with_vol(&a.menu_confirm, vol_unless_muted(MENU_SFX_VOL, muted));
}

pub fn play_menu_cancel(a: &AudioAssets, muted: bool) {
	play_sound_once_with_vol(&a.menu_cancel, vol_unless_muted(MENU_SFX_VOL, muted));
}

/// Labels for the settings “Sound FX test” browser (same order as [`play_sfx_test_sample`]).
pub const SFX_TEST_LABELS: &[&str] = &[
	"Lock",
	"Sonic",
	"Next — I",
	"Next — T",
	"Next — L",
	"Next — J",
	"Next — S",
	"Next — Z",
	"Next — O",
	"Line clear 1×",
	"Line clear 2×",
	"Line clear 3×",
	"Line clear 4×",
	"Line clear (uki) 1×",
	"Line clear (uki) 2×",
	"Line clear (uki) 3×",
	"Line clear (uki) 4×",
	"Grade up",
	"Game over",
	"Cleared",
	"Gate bell (hundreds)",
	"Menu — move",
	"Menu — confirm",
	"Menu — cancel",
	"Start — READY",
	"Start — GO",
];

/// Play one embedded SFX by index (see [`SFX_TEST_LABELS`]).
pub fn play_sfx_test_sample(a: &AudioAssets, idx: usize, muted: bool) {
	let sound = match idx {
		0 => &a.lock,
		1 => &a.sonic,
		2 => &a.next_i,
		3 => &a.next_t,
		4 => &a.next_l,
		5 => &a.next_j,
		6 => &a.next_s,
		7 => &a.next_z,
		8 => &a.next_o,
		9 => &a.line_1,
		10 => &a.line_2,
		11 => &a.line_3,
		12 => &a.line_4,
		13 => &a.line_uki_1,
		14 => &a.line_uki_2,
		15 => &a.line_uki_3,
		16 => &a.line_uki_4,
		17 => &a.grade_up,
		18 => &a.game_over,
		19 => &a.cleared,
		20 => &a.gate_bell,
		21 => &a.menu_move,
		22 => &a.menu_confirm,
		23 => &a.menu_cancel,
		24 => &a.ready_voice,
		25 => &a.go_voice,
		_ => return,
	};
	let vol = match idx {
		0 => LOCK_CLICK_VOL,
		20 => GATE_BELL_VOL,
		21..=23 => MENU_SFX_VOL,
		24..=25 => READY_GO_VOL,
		_ => SFX_VOL,
	};
	play_sound_once_with_vol(sound, vol_unless_muted(vol, muted));
}

fn play_sound_once_with_vol(s: &Sound, vol: f32) {
	play_sound(
		s,
		PlaySoundParams {
			looped: false,
			volume: vol,
		},
	);
}

fn line_clear_count(game: &Game) -> u32 {
	if game.phase != Phase::LineClear {
		return 0;
	}
	count_full_lines(&find_full_lines(&game.board))
}

fn line_sound(a: &AudioAssets, n: u32) -> &Sound {
	match n {
		1 => &a.line_1,
		2 => &a.line_2,
		3 => &a.line_3,
		4 => &a.line_4,
		_ => &a.line_1,
	}
}

fn next_piece_sound(kind: PieceKind, a: &AudioAssets) -> &Sound {
	match kind {
		PieceKind::I => &a.next_i,
		PieceKind::T => &a.next_t,
		PieceKind::L => &a.next_l,
		PieceKind::J => &a.next_j,
		PieceKind::S => &a.next_s,
		PieceKind::Z => &a.next_z,
		PieceKind::O => &a.next_o,
	}
}

/// True if this step increased level through any [`line_clear_only_for_increment`] boundary
/// (99, 199, … 899, or 998): piece spawns no longer raise level until lines are cleared.
fn crossed_hundreds_gate_level(level_before: u16, level_after: u16) -> bool {
	if level_after <= level_before {
		return false;
	}
	((level_before + 1)..=level_after).any(line_clear_only_for_increment)
}

fn sonic_moved(pb: &PieceState, pa: &PieceState, input: Input) -> bool {
	if !input.sonic {
		return false;
	}
	(pb.y - pa.y).max(0) > 0
}

/// Call once per simulation step after [`Game::step`], with state captured **before** `step`.
pub fn feed_game_audio_cues(
	game: &Game,
	piece_before: Option<PieceState>,
	input: Input,
	phase_before_step: Phase,
	level_before_step: u16,
	game_over_before: bool,
	cleared_before: bool,
	a: &AudioAssets,
	muted: bool,
) {
	if game.game_over && !game_over_before {
		play_sound_once_with_vol(&a.game_over, vol_unless_muted(SFX_VOL, muted));
	}
	if game.cleared && !cleared_before {
		play_sound_once_with_vol(&a.cleared, vol_unless_muted(SFX_VOL, muted));
	}

	if crossed_hundreds_gate_level(level_before_step, game.level) {
		play_sound_once_with_vol(&a.gate_bell, vol_unless_muted(GATE_BELL_VOL, muted));
	}

	if phase_before_step == Phase::Are && game.phase == Phase::Falling
		&& game.piece.is_some() {
			// Matches the NEXT window: `next_kind` is already advanced after spawn.
			play_sound_once_with_vol(
				next_piece_sound(game.next_kind, a),
				vol_unless_muted(SFX_VOL, muted),
			);
		}

	if let (Some(pb), Some(pa)) = (piece_before, game.piece) {
		if game.phase == Phase::Falling
			&& sonic_moved(&pb, &pa, input) {
				play_sound_once_with_vol(&a.sonic, vol_unless_muted(SFX_VOL, muted));
			}
	}

	if piece_before.is_some() && game.piece.is_none() && !game.game_over {
		play_sound_once_with_vol(&a.lock, vol_unless_muted(LOCK_CLICK_VOL, muted));
		if game.phase == Phase::LineClear {
			let n = line_clear_count(game);
			if (1..=4).contains(&n) {
				let s = line_sound(a, n);
				play_sound_once_with_vol(s, vol_unless_muted(SFX_VOL, muted));
			}
		}
	}
}
