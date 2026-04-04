//! TGM1 title-screen hidden modes ([TetrisWiki](https://tetris.wiki/Tetris_The_Grand_Master)).

use serde::{Deserialize, Serialize};

use crate::constants::{ARE_FRAMES, LINE_CLEAR_FRAMES};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameOptions {
	/// Constant 20G gravity at all levels.
	pub force_20g: bool,
	/// TLS (ghost) for all levels, not only 0..=100.
	pub tls_always: bool,
	/// Monochrome blocks (client rendering).
	pub monochrome: bool,
	/// Uki line-clear sounds (client audio).
	pub uki: bool,
	/// Big blocks on a 5×10 logical field.
	pub big: bool,
	/// Reverse gravity: spawn low, pieces move upward.
	pub reverse: bool,
	/// Autoplay mode: minimal line-clear and ARE (next-piece) delay for faster bot runs.
	#[serde(default)]
	pub autoplay: bool,
}

impl GameOptions {
	/// ARE (spawn delay after lock / line clear): 1 frame when [`Self::autoplay`], else TGM1 default.
	pub fn are_frames(self) -> u32 {
		if self.autoplay {
			1
		} else {
			ARE_FRAMES
		}
	}

	/// Line clear animation delay: 1 frame when [`Self::autoplay`], else TGM1 default.
	pub fn line_clear_frames(self) -> u32 {
		if self.autoplay {
			1
		} else {
			LINE_CLEAR_FRAMES
		}
	}

	/// True when any hidden-mode code was used (ineligible for default high score table).
	pub fn any_hidden_mode(self) -> bool {
		self.force_20g || self.tls_always || self.monochrome || self.uki || self.big || self.reverse
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn autoplay_shortens_are_and_line_clear() {
		let mut o = GameOptions::default();
		assert_eq!(o.are_frames(), ARE_FRAMES);
		assert_eq!(o.line_clear_frames(), LINE_CLEAR_FRAMES);
		o.autoplay = true;
		assert_eq!(o.are_frames(), 1);
		assert_eq!(o.line_clear_frames(), 1);
	}
}
