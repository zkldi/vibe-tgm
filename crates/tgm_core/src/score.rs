//! TGM1 scoring.

pub fn add_score(
	score: u64,
	level_before: u16,
	lines: u32,
	soft_frames: u32,
	combo: &mut u32,
	bravo: u32,
) -> u64 {
	if lines == 0 {
		*combo = 1;
		return score;
	}
	let prev = *combo;
	*combo = prev + 2 * lines - 2;
	let ceil = ((level_before as u64 + lines as u64) + 3) / 4;
	let soft = soft_frames as u64 + 1;
	let add = (ceil + soft) * lines as u64 * (*combo as u64) * bravo as u64;
	score.saturating_add(add)
}

pub fn bravo_factor(board_empty_after_clear: bool) -> u32 {
	if board_empty_after_clear { 4 } else { 1 }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn combo_reset_no_clear() {
		let mut c = 5u32;
		let s = add_score(0, 0, 0, 0, &mut c, 1);
		assert_eq!(s, 0);
		assert_eq!(c, 1);
	}

	#[test]
	fn simple_single() {
		let mut c = 1u32;
		let s = add_score(0, 0, 1, 0, &mut c, 1);
		assert_eq!(c, 1);
		// ceil((0+1)/4)+1 soft = 1+1 = 2
		assert_eq!(s, 2);
	}
}
