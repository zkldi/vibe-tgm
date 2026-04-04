//! Game options (timing and autoplay).

use serde::{Deserialize, Serialize};

use crate::constants::{ARE_FRAMES, LINE_CLEAR_FRAMES};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameOptions {
	/// Autoplay mode: minimal line-clear and ARE (next-piece) delay for faster bot runs.
	#[serde(default)]
	pub autoplay: bool,
}

impl GameOptions {
	/// ARE (spawn delay after lock / line clear): 1 frame when [`Self::autoplay`], else TGM1
	/// default.
	pub fn are_frames(self) -> u32 {
		if self.autoplay { 1 } else { ARE_FRAMES }
	}

	/// Line clear animation delay: 1 frame when [`Self::autoplay`], else TGM1 default.
	pub fn line_clear_frames(self) -> u32 {
		if self.autoplay { 1 } else { LINE_CLEAR_FRAMES }
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
