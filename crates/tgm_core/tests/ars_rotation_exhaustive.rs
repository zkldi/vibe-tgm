//! Exhaustive ARS rotation tests.
//!
//! Canonical shapes are derived by placing each spawn orientation in a 4×4 raster
//! (x right, y up, y=0 bottom), rotating the raster 90° CW (Sega-style rigid rotation
//! in a fixed box), then normalizing to bbox min at (0,0). This matches how ARS differs
//! from SRS: states are fixed orientations in a 4×4, not SRS kicks.
//!
//! The T-piece `rot 3` in `piece_cells` is the mirror of `rot 1` (not the rigid 4×4
//! result from `rot 2`), so the stem rotates left/right symmetrically around the center.
//!
//! `piece_cells` must match these sequences and `rotate_cw` / `rotate_ccw` must step
//! indices consistently.

use std::collections::HashSet;

use tgm_core::{
	Board, PieceKind, piece_cells, rotate_ccw, rotate_cw, try_rotate_ccw, try_rotate_cw,
};

// ---------------------------------------------------------------------------
// 4×4 raster: row r=0 is TOP of field, r=3 is BOTTOM (screen coords).
// Game coords: y=0 bottom, y=3 top  =>  r = 3 - y.
// ---------------------------------------------------------------------------

type Raster = [[bool; 4]; 4];

fn cells_to_raster(cells: &[(i8, i8)]) -> Raster {
	let mut m = [[false; 4]; 4];
	for &(x, y) in cells {
		assert!((0..4).contains(&x) && (0..4).contains(&y));
		let r = (3 - y) as usize;
		let c = x as usize;
		m[r][c] = true;
	}
	m
}

fn rotate_raster_cw(m: &Raster) -> Raster {
	let mut o = [[false; 4]; 4];
	for r in 0..4 {
		for c in 0..4 {
			o[r][c] = m[3 - c][r];
		}
	}
	o
}

fn raster_to_cells_normalized(m: &Raster) -> [(i8, i8); 4] {
	let mut v = Vec::with_capacity(4);
	for r in 0..4 {
		for c in 0..4 {
			if m[r][c] {
				let y = (3 - r) as i8;
				let x = c as i8;
				v.push((x, y));
			}
		}
	}
	assert_eq!(v.len(), 4);
	let min_x = v.iter().map(|p| p.0).min().unwrap();
	let min_y = v.iter().map(|p| p.1).min().unwrap();
	v.sort_by_key(|&(x, y)| (y, x));
	let mut out = [(0i8, 0i8); 4];
	for (i, &(x, y)) in v.iter().enumerate() {
		out[i] = (x - min_x, y - min_y);
	}
	out
}

fn sort_cells(mut c: [(i8, i8); 4]) -> [(i8, i8); 4] {
	c.sort_by_key(|&(x, y)| (y, x));
	c
}

fn geom_sequence_from_seed(seed: [(i8, i8); 4]) -> [[(i8, i8); 4]; 4] {
	let mut r = cells_to_raster(&seed);
	let mut seq = [seed; 4];
	for i in 1..4 {
		r = rotate_raster_cw(&r);
		seq[i] = sort_cells(raster_to_cells_normalized(&r));
	}
	seq
}

fn assert_cells_eq(a: &[(i8, i8); 4], b: &[(i8, i8); 4]) {
	assert_eq!(sort_cells(*a), sort_cells(*b), "cell mismatch");
}

// ---------------------------------------------------------------------------
// L / J: four distinct orientations from spawn seed (TGM spawn: flat up, foot down-right for L)
// ---------------------------------------------------------------------------

/// L spawn matching current `piece_cells(L, 0)` before any fix — used as seed only.
const L_SEED: [(i8, i8); 4] = [(2, 0), (0, 1), (1, 1), (2, 1)];

/// J spawn matching `piece_cells(J, 0)`.
const J_SEED: [(i8, i8); 4] = [(0, 0), (0, 1), (1, 1), (2, 1)];

/// T piece rot 3 is the mirror of rot 1 (column at x=1, tab at (2,1) vs rot 1’s tab at (0,1)),
/// not the 4×4 rigid-CW+normalize result from rot 2, which shifts the T one column left
/// (Sega-style “plus” rotation around the center cell).
const T_EXPECTED: [[(i8, i8); 4]; 4] = [
	[(1, 0), (0, 1), (1, 1), (2, 1)],
	[(1, 0), (0, 1), (1, 1), (1, 2)],
	[(0, 0), (1, 0), (2, 0), (1, 1)],
	[(1, 0), (1, 1), (1, 2), (2, 1)],
];

#[test]
fn l_piece_geometric_cycle_is_length_4() {
	let seq = geom_sequence_from_seed(L_SEED);
	let mut set = HashSet::new();
	for s in &seq {
		set.insert(sort_cells(*s));
	}
	assert_eq!(
		set.len(),
		4,
		"L should have 4 distinct orientations under 4×4 CW rotation"
	);
}

#[test]
fn piece_cells_l_matches_geometry() {
	let seq = geom_sequence_from_seed(L_SEED);
	for (i, expected) in seq.iter().enumerate() {
		assert_cells_eq(&piece_cells(PieceKind::L, i as u8).cells, expected);
	}
}

#[test]
fn piece_cells_j_matches_geometry() {
	let seq = geom_sequence_from_seed(J_SEED);
	for (i, expected) in seq.iter().enumerate() {
		assert_cells_eq(&piece_cells(PieceKind::J, i as u8).cells, expected);
	}
}

#[test]
fn piece_cells_t_matches_sega_plus_sequence() {
	for (i, expected) in T_EXPECTED.iter().enumerate() {
		assert_cells_eq(&piece_cells(PieceKind::T, i as u8).cells, expected);
	}
}

#[test]
fn rotate_index_ljt_is_cw_step() {
	for &k in &[PieceKind::L, PieceKind::J, PieceKind::T] {
		for r in 0u8..4u8 {
			assert_eq!(rotate_cw(k, r), (r + 1) % 4);
			assert_eq!(rotate_ccw(k, r), (r + 3) % 4);
			assert_eq!(rotate_ccw(k, rotate_cw(k, r)), r);
		}
	}
}

#[test]
fn rotate_index_isz_cycles_two_states() {
	for &k in &[PieceKind::I, PieceKind::S, PieceKind::Z] {
		assert_eq!(rotate_cw(k, 0), 2);
		assert_eq!(rotate_cw(k, 2), 0);
		assert_eq!(rotate_ccw(k, rotate_cw(k, 0)), 0);
	}
}

#[test]
fn o_never_changes_rot() {
	for r in 0u8..4u8 {
		assert_eq!(rotate_cw(PieceKind::O, r), r);
		assert_eq!(rotate_ccw(PieceKind::O, r), r);
	}
}

// ---------------------------------------------------------------------------
// I / S / Z: index uses rot 0↔2 but shapes are one 90° step apart (ARS toggle).
// Compare up to translation (normalize bbox to origin).
// ---------------------------------------------------------------------------

fn normalize_shape(mut c: [(i8, i8); 4]) -> [(i8, i8); 4] {
	let min_x = c.iter().map(|p| p.0).min().unwrap();
	let min_y = c.iter().map(|p| p.1).min().unwrap();
	for i in 0..4 {
		c[i] = (c[i].0 - min_x, c[i].1 - min_y);
	}
	sort_cells(c)
}

fn one_geom_cw(seed: [(i8, i8); 4]) -> [(i8, i8); 4] {
	let r0 = cells_to_raster(&seed);
	let r1 = rotate_raster_cw(&r0);
	sort_cells(raster_to_cells_normalized(&r1))
}

#[test]
fn piece_cells_i_sz_match_one_quarter_turn_normalized() {
	let i0 = piece_cells(PieceKind::I, 0).cells;
	assert_cells_eq(
		&normalize_shape(piece_cells(PieceKind::I, 2).cells),
		&normalize_shape(one_geom_cw(i0)),
	);

	let s0 = piece_cells(PieceKind::S, 0).cells;
	assert_cells_eq(
		&normalize_shape(piece_cells(PieceKind::S, 2).cells),
		&normalize_shape(one_geom_cw(s0)),
	);

	let z0 = piece_cells(PieceKind::Z, 0).cells;
	assert_cells_eq(
		&normalize_shape(piece_cells(PieceKind::Z, 2).cells),
		&normalize_shape(one_geom_cw(z0)),
	);
}

// ---------------------------------------------------------------------------
// Kick order on empty board: basic, +1 x, −1 x
// ---------------------------------------------------------------------------

fn empty_board() -> Board {
	Board::new()
}

#[test]
fn try_rotate_empty_board_no_kick_needed() {
	let b = empty_board();
	let px = 4;
	let py = 10;
	for kind in PieceKind::ALL {
		for rot in 0u8..4u8 {
			if kind == PieceKind::O {
				continue;
			}
			let new_rot = rotate_cw(kind, rot);
			if new_rot == rot {
				continue;
			}
			let r = try_rotate_cw(&b, px, py, kind, rot).expect("cw rotate on empty");
			assert_eq!(r.0, px);
			assert_eq!(r.1, py);
			assert_eq!(r.2, new_rot);
		}
	}
}

#[test]
fn try_rotate_ccw_empty_board_inverse_of_cw() {
	let b = empty_board();
	let px = 4;
	let py = 10;
	for kind in PieceKind::ALL {
		for rot in 0u8..4u8 {
			if kind == PieceKind::O {
				continue;
			}
			let prev_rot = rotate_ccw(kind, rot);
			if prev_rot == rot {
				continue;
			}
			let r = try_rotate_ccw(&b, px, py, kind, rot).expect("ccw rotate on empty");
			assert_eq!(r.0, px);
			assert_eq!(r.1, py);
			assert_eq!(r.2, prev_rot);
			assert_eq!(rotate_cw(kind, prev_rot), rot);
		}
	}
}

#[test]
fn i_piece_never_kicks_even_when_blocked() {
	let mut b = Board::new();
	// Block the column where vertical I (rot 2) places its stem: local x=2, so px+2.
	let px = 3i32;
	let py = 0i32;
	for y in 0..4 {
		b.rows[y as usize][(px + 2) as usize] = 1;
	}
	let r = try_rotate_cw(&b, px, py, PieceKind::I, 0);
	assert!(r.is_none());
}

#[test]
fn all_kinds_exactly_four_cells() {
	for kind in PieceKind::ALL {
		for rot in 0u8..4u8 {
			let d = piece_cells(kind, rot);
			assert_eq!(d.cells.len(), 4);
			let mut seen = HashSet::new();
			for c in d.cells {
				assert!(
					seen.insert(c),
					"duplicate cell {:?} kind {:?} rot {}",
					c,
					kind,
					rot
				);
			}
		}
	}
}

#[test]
fn all_kinds_orthogonally_connected() {
	for kind in PieceKind::ALL {
		for rot in 0u8..4u8 {
			let cells: Vec<(i8, i8)> = piece_cells(kind, rot).cells.to_vec();
			assert!(connected_4(&cells), "{kind:?} rot {rot}");
		}
	}
}

fn connected_4(cells: &[(i8, i8)]) -> bool {
	let set: HashSet<(i8, i8)> = cells.iter().copied().collect();
	let start = cells[0];
	let mut stack = vec![start];
	let mut seen = HashSet::new();
	seen.insert(start);
	while let Some(p) = stack.pop() {
		for d in [(0i8, 1i8), (0, -1), (1, 0), (-1, 0)] {
			let n = (p.0 + d.0, p.1 + d.1);
			if set.contains(&n) && seen.insert(n) {
				stack.push(n);
			}
		}
	}
	seen.len() == 4
}
