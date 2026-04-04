use tgm_core::{
	ARE_FRAMES, BOARD_WIDTH, DAS_FRAMES, Game, GameOptions, Input, Phase, PieceKind, VISIBLE_ROWS,
	input_pack, input_unpack, internal_gravity, level_after_line_clear,
	line_clear_only_for_increment,
};

#[test]
fn gravity_table_end() {
	assert_eq!(internal_gravity(999), 5120);
}

#[test]
fn line_clear_level_steps() {
	assert_eq!(level_after_line_clear(998, 1), Some(999));
	assert_eq!(level_after_line_clear(997, 2), Some(999));
}

#[test]
fn hundreds_spawn_gate() {
	assert!(line_clear_only_for_increment(299));
}

#[test]
fn game_runs_frames() {
	let mut g = Game::new(1);
	for _ in 0..120 {
		g.step(Input::default());
		if g.game_over {
			break;
		}
	}
}

/// IRS must not force a rotation that collides at spawn when the default orientation would fit.
#[test]
fn irs_cannot_spawn_into_death_when_default_rotation_fits() {
	let mut g = Game::new(1);
	g.level = 0;
	for y in 0..VISIBLE_ROWS {
		for x in 0..BOARD_WIDTH {
			let hole = y == 19 && (3..=6).contains(&x);
			if !hole {
				g.board.rows[y][x] = 1;
			}
		}
	}
	g.next_kind = PieceKind::I;
	g.phase = Phase::Are;
	g.are_timer = 0;
	g.piece = None;

	g.step(Input {
		rot_cw: true,
		..Default::default()
	});
	assert!(
		!g.game_over,
		"IRS should be ignored when IRS orientation would collide"
	);
	assert_eq!(
		g.piece.as_ref().unwrap().rot % 4,
		0,
		"should fall back to horizontal I"
	);
}

/// Instant DAS: holding a direction during ARE charges DAS so the next piece isn’t stuck at 0.
#[test]
fn das_charges_during_are_like_irs() {
	let mut g = Game::new(1);
	assert_eq!(g.phase, Phase::Are);
	assert_eq!(g.are_timer, ARE_FRAMES);
	for _ in 0..DAS_FRAMES {
		g.step(Input {
			right: true,
			..Default::default()
		});
	}
	assert_eq!(
		g.das_right, DAS_FRAMES,
		"DAS counter should reach charge threshold while waiting in ARE"
	);
}

#[test]
fn soft_drop_insta_locks_when_grounded() {
	let mut g = Game::new(42);
	for _ in 0..500 {
		if g.phase == Phase::Falling && g.piece.is_some() {
			break;
		}
		g.step(Input::default());
	}
	assert!(g.piece.is_some(), "expected a live piece");
	g.step(Input {
		sonic: true,
		..Default::default()
	});
	assert!(g.piece.is_some(), "sonic should not lock by itself");
	g.step(Input {
		down: true,
		..Default::default()
	});
	assert!(
		g.piece.is_none(),
		"soft drop while grounded should lock immediately"
	);
}

/// Packed replay bytes fed to `Game::step` must reproduce the same outcome every time.
#[test]
fn replay_inputs_are_deterministic() {
	let seed = 0xC0FFEE_u64;
	let opts = GameOptions::default();
	let packed: Vec<u8> = (0..400).map(|_| input_pack(Input::default())).collect();
	let run = || {
		let mut g = Game::with_options(seed, opts);
		for b in &packed {
			g.step(input_unpack(*b).unwrap());
			if g.game_over || g.cleared {
				break;
			}
		}
		(g.score, g.level, g.frame, g.game_over, g.cleared)
	};
	assert_eq!(run(), run());
}
