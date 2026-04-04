//! Title-screen cheat codes ([TetrisWiki — TGM1](https://tetris.wiki/Tetris_The_Grand_Master)).

use tgm_core::GameOptions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleToken {
	L,
	D,
	U,
	R,
	A,
	B,
	C,
}

pub const TITLE_BUFFER_CAP: usize = 64;

fn contains_subseq(haystack: &[TitleToken], needle: &[TitleToken]) -> bool {
	if needle.is_empty() {
		return true;
	}
	if needle.len() > haystack.len() {
		return false;
	}
	haystack.windows(needle.len()).any(|w| w == needle)
}

/// Decode combined hidden modes from the title input buffer (order-independent substrings).
pub fn decode_options(buf: &[TitleToken]) -> GameOptions {
	use TitleToken::*;
	let mut o = GameOptions::default();

	const CODE_20G: &[TitleToken] = &[D, D, D, D, D, D, D, C, B, A];
	const CODE_BIG: &[TitleToken] = &[L, L, L, L, D, C, B, A];
	const CODE_REV: &[TitleToken] = &[D, U, U, D, C, B, A];
	const CODE_MONO: &[TitleToken] = &[R, R, R, U, C, B, A];
	const CODE_TLS: &[TitleToken] = &[A, B, C, C, B, A, A, C, B];
	const CODE_UKI: &[TitleToken] = &[A, B, A, B, A, B, A, B, A, B, A, B, A, B, A, B, A, B, B];

	if contains_subseq(buf, CODE_20G) {
		o.force_20g = true;
	}
	if contains_subseq(buf, CODE_BIG) {
		o.big = true;
	}
	if contains_subseq(buf, CODE_REV) {
		o.reverse = true;
	}
	if contains_subseq(buf, CODE_MONO) {
		o.monochrome = true;
	}
	if contains_subseq(buf, CODE_TLS) {
		o.tls_always = true;
	}
	if contains_subseq(buf, CODE_UKI) {
		o.uki = true;
	}
	o
}

#[cfg(test)]
mod tests {
	use super::*;

	fn t(s: &str) -> Vec<TitleToken> {
		s.chars()
			.map(|c| match c {
				'L' => TitleToken::L,
				'D' => TitleToken::D,
				'U' => TitleToken::U,
				'R' => TitleToken::R,
				'A' => TitleToken::A,
				'B' => TitleToken::B,
				'C' => TitleToken::C,
				_ => panic!("bad token"),
			})
			.collect()
	}

	#[test]
	fn code_20g() {
		let o = decode_options(&t("DDDDDDDDCBA"));
		assert!(o.force_20g);
	}

	#[test]
	fn code_big_rev_combo() {
		let o = decode_options(&t("LLLLDCBADUUDCBA"));
		assert!(o.big);
		assert!(o.reverse);
	}
}
