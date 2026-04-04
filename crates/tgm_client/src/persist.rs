//! Machine-local JSON next to the executable: `highscores.json`, `replays/*.json`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

use serde::{Deserialize, Serialize};
use tgm_core::GameOptions;

pub const REPLAY_FORMAT: &str = "tgm1-replay";
pub const REPLAY_VERSION: u32 = 1;
pub const HISCORE_VERSION: u32 = 1;
pub const HISCORE_MAX: usize = 10;

/// Directory containing `highscores.json` and the `replays/` folder.
pub fn data_dir() -> PathBuf {
	std::env::current_exe()
		.ok()
		.and_then(|p| p.parent().map(|d| d.to_path_buf()))
		.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn highscores_path() -> PathBuf {
	data_dir().join("highscores.json")
}

pub fn settings_path() -> PathBuf {
	data_dir().join("settings.json")
}

/// Windowed size and fullscreen flag (restored on next launch).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientSettings {
	pub window_width: u32,
	pub window_height: u32,
	pub fullscreen: bool,
}

impl Default for ClientSettings {
	fn default() -> Self {
		Self {
			window_width: 1280,
			window_height: 720,
			fullscreen: false,
		}
	}
}

pub fn load_client_settings() -> ClientSettings {
	let path = settings_path();
	let Ok(s) = fs::read_to_string(&path) else {
		return ClientSettings::default();
	};
	serde_json::from_str(&s).unwrap_or_default()
}

pub fn save_client_settings(s: &ClientSettings) -> io::Result<()> {
	let path = settings_path();
	let json = serde_json::to_string_pretty(s)
		.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
	fs::write(path, json)
}

pub fn replays_dir() -> PathBuf {
	data_dir().join("replays")
}

/// Final run stats stored with a replay (same meaning as [`HighScoreEntry`] fields).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplaySummary {
	pub score: u64,
	pub level: u16,
	pub grade: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayFile {
	pub format: String,
	pub version: u32,
	pub seed: u64,
	pub options: GameOptions,
	pub inputs: Vec<u8>,
	/// Wall time when the file was written (`now_ms()`). Omitted in older files (deserializes as
	/// 0).
	#[serde(default)]
	pub saved_at_ms: u64,
	/// End-of-run stats. `None` in older files.
	#[serde(default)]
	pub summary: Option<ReplaySummary>,
}

impl ReplayFile {
	pub fn new(
		seed: u64,
		options: GameOptions,
		inputs: Vec<u8>,
		saved_at_ms: u64,
		summary: ReplaySummary,
	) -> Self {
		Self {
			format: REPLAY_FORMAT.to_string(),
			version: REPLAY_VERSION,
			seed,
			options,
			inputs,
			saved_at_ms,
			summary: Some(summary),
		}
	}

	pub fn validate(&self) -> Result<(), String> {
		if self.format != REPLAY_FORMAT {
			return Err(format!("unknown format {:?}", self.format));
		}
		if self.version != REPLAY_VERSION {
			return Err(format!("unsupported replay version {}", self.version));
		}
		for &b in &self.inputs {
			tgm_core::input_unpack(b).ok_or_else(|| format!("invalid input byte {b:#x}"))?;
		}
		Ok(())
	}
}

pub fn save_replay(path: &Path, replay: &ReplayFile) -> io::Result<()> {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}
	let json = serde_json::to_string_pretty(replay)
		.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
	fs::write(path, json)
}

pub fn load_replay(path: &Path) -> io::Result<ReplayFile> {
	let s = fs::read_to_string(path)?;
	let r: ReplayFile =
		serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
	r.validate()
		.map_err(|msg| io::Error::new(io::ErrorKind::InvalidData, msg))?;
	Ok(r)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HighScoresFile {
	pub version: u32,
	pub entries: Vec<HighScoreEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HighScoreEntry {
	pub score: u64,
	pub grade: String,
	pub level: u16,
	pub cleared: bool,
	pub gm: bool,
	pub saved_at_ms: u64,
}

impl Default for HighScoresFile {
	fn default() -> Self {
		Self {
			version: HISCORE_VERSION,
			entries: Vec::new(),
		}
	}
}

pub fn load_highscores() -> HighScoresFile {
	let path = highscores_path();
	let Ok(s) = fs::read_to_string(&path) else {
		return HighScoresFile::default();
	};
	serde_json::from_str(&s).unwrap_or_default()
}

pub fn save_highscores(h: &HighScoresFile) -> io::Result<()> {
	let path = highscores_path();
	let json = serde_json::to_string_pretty(h)
		.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
	fs::write(path, json)
}

pub fn now_ms() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_millis() as u64)
		.unwrap_or(0)
}

/// Insert `entry` if it belongs in the top `HISCORE_MAX` by `score`, then persist.
pub fn merge_highscore(
	mut file: HighScoresFile,
	entry: HighScoreEntry,
) -> io::Result<HighScoresFile> {
	file.version = HISCORE_VERSION;
	file.entries.push(entry);
	file.entries.sort_by(|a, b| b.score.cmp(&a.score));
	file.entries.truncate(HISCORE_MAX);
	save_highscores(&file)?;
	Ok(file)
}

/// Sorted by mtime descending (newest first). Only `*.json` in `replays/`.
pub fn list_replay_files() -> io::Result<Vec<PathBuf>> {
	let dir = replays_dir();
	if !dir.is_dir() {
		return Ok(Vec::new());
	}
	let mut paths: Vec<PathBuf> = fs::read_dir(&dir)?
		.filter_map(|e| e.ok())
		.map(|e| e.path())
		.filter(|p| {
			p.extension()
				.and_then(|x| x.to_str())
				.map(|x| x.eq_ignore_ascii_case("json"))
				.unwrap_or(false)
		})
		.collect();
	paths.sort_by(|a, b| {
		let ma = mtime_ms(a);
		let mb = mtime_ms(b);
		mb.cmp(&ma)
	});
	Ok(paths)
}

/// One row in the replay list UI: path, resolved display time, and optional run summary.
#[derive(Clone, Debug)]
pub struct ReplayListEntry {
	pub path: PathBuf,
	/// Milliseconds since Unix epoch for display (from JSON `saved_at_ms`, else filename, else
	/// mtime).
	pub display_ms: u64,
	pub summary: Option<ReplaySummary>,
}

/// Load replays in [`list_replay_files`] order; skips files that fail to load or validate.
pub fn load_replay_list_entries() -> Vec<ReplayListEntry> {
	let Ok(paths) = list_replay_files() else {
		return Vec::new();
	};
	let mut out = Vec::new();
	for path in paths {
		let Ok(r) = load_replay(&path) else {
			continue;
		};
		let display_ms = if r.saved_at_ms > 0 {
			r.saved_at_ms
		} else if let Some(ms) = replay_ms_from_filename(&path) {
			ms
		} else {
			mtime_ms(&path)
		};
		out.push(ReplayListEntry {
			path,
			display_ms,
			summary: r.summary,
		});
	}
	out
}

fn replay_ms_from_filename(path: &Path) -> Option<u64> {
	path.file_stem()
		.and_then(|s| s.to_str())
		.and_then(|s| s.strip_prefix("replay_"))
		.and_then(|s| s.parse().ok())
}

fn mtime_ms(path: &Path) -> u64 {
	fs::metadata(path)
		.and_then(|m| m.modified())
		.ok()
		.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
		.map(|d| d.as_millis() as u64)
		.unwrap_or(0)
}
