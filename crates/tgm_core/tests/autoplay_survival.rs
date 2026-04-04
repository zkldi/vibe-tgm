//! Autoplay: one full run on a random seed (`thread_rng`), then assert level-999 clear.

use rand::{Rng, thread_rng};
use tgm_core::{AutoplayDriver, Game, GameOptions, Input, Phase};

/// Frames before giving up on a single attempt (safety valve).
const MAX_FRAMES: u64 = 120_000_000;

fn run_one(seed: u64) -> Game {
	let mut g = Game::with_options(seed, GameOptions { autoplay: true });
	let mut driver = AutoplayDriver::default();
	for _ in 0..MAX_FRAMES {
		if g.game_over || g.cleared {
			break;
		}
		let inp = driver.pick_input(&g);
		g.step(inp);
	}
	g
}

#[test]
#[ignore]
fn autoplay_survives_to_level_999() {
	let seed: u64 = thread_rng().gen();
	let g = run_one(seed);
	assert!(
		g.cleared && g.level >= 999 && !g.game_over,
		"random seed {seed:#x}: expected clear at level 999, got level={} cleared={} game_over={} \
		 frames={}",
		g.level,
		g.cleared,
		g.game_over,
		g.frame
	);
}

#[test]
fn autoplay_driver_reaches_falling_with_piece() {
	let seed: u64 = thread_rng().gen();
	let mut g = Game::with_options(seed, GameOptions { autoplay: true });
	let mut driver = AutoplayDriver::default();
	for _ in 0..50_000 {
		if g.phase == Phase::Falling && g.piece.is_some() {
			let _ = driver.pick_input(&g);
			return;
		}
		g.step(Input::default());
	}
	panic!("expected Falling phase with a piece");
}
