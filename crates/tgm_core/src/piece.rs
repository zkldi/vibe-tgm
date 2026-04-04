//! Tetromino definitions for TGM / ARS (Sega-like orientations).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PieceKind {
	I = 0,
	T = 1,
	L = 2,
	J = 3,
	S = 4,
	Z = 5,
	O = 6,
}

impl PieceKind {
	pub const ALL: [PieceKind; 7] = [
		PieceKind::I,
		PieceKind::T,
		PieceKind::L,
		PieceKind::J,
		PieceKind::S,
		PieceKind::Z,
		PieceKind::O,
	];

	pub fn from_u8(v: u8) -> Option<Self> {
		if v < 7 {
			Some(unsafe { std::mem::transmute(v) })
		} else {
			None
		}
	}
}

/// Rotation index 0..4 (O uses only 0; I,S,Z use 0,2 effectively).
pub type RotIndex = u8;

/// Occupied cells as offsets from piece origin (bottom-left of 4×4 bounding box).
/// Coordinates: x right, y up from bbox bottom (so y=0 is lowest row of bbox).
#[derive(Clone, Copy, Debug)]
pub struct PieceDef {
	pub cells: [(i8, i8); 4],
}

/// ARS rotation states. For I,S,Z only rot 0 and 2 are distinct (toggle).
pub fn piece_cells(kind: PieceKind, rot: RotIndex) -> PieceDef {
	let r = rot % 4;
	match kind {
		PieceKind::I => {
			// I toggles rot 0 <-> 2 only; those must be horizontal vs vertical (not both flat).
			if r == 0 {
				PieceDef {
					cells: [(0, 1), (1, 1), (2, 1), (3, 1)],
				}
			} else if r == 2 {
				PieceDef {
					cells: [(2, 0), (2, 1), (2, 2), (2, 3)],
				}
			} else {
				// r == 1 | 3: unused with (rot+2)%4 but keep valid ARS shapes
				PieceDef {
					cells: [(2, 0), (2, 1), (2, 2), (2, 3)],
				}
			}
		}
		PieceKind::O => PieceDef {
			cells: [(1, 0), (2, 0), (1, 1), (2, 1)],
		},
		// L, J: four states from 4×4 CW + bbox normalize. T matches rot 3 mirror to rot 1
		// (Sega plus); see tests/ars_rotation_exhaustive.rs.
		PieceKind::T => match r {
			0 => PieceDef {
				cells: [(1, 0), (0, 1), (1, 1), (2, 1)],
			},
			1 => PieceDef {
				cells: [(1, 0), (0, 1), (1, 1), (1, 2)],
			},
			2 => PieceDef {
				cells: [(0, 0), (1, 0), (2, 0), (1, 1)],
			},
			3 => PieceDef {
				cells: [(1, 0), (1, 1), (1, 2), (2, 1)],
			},
			_ => unreachable!(),
		},
		PieceKind::L => match r {
			0 => PieceDef {
				cells: [(2, 0), (0, 1), (1, 1), (2, 1)],
			},
			1 => PieceDef {
				cells: [(0, 0), (1, 0), (1, 1), (1, 2)],
			},
			2 => PieceDef {
				cells: [(0, 0), (1, 0), (2, 0), (0, 1)],
			},
			3 => PieceDef {
				cells: [(0, 0), (0, 1), (0, 2), (1, 2)],
			},
			_ => unreachable!(),
		},
		PieceKind::J => match r {
			0 => PieceDef {
				cells: [(0, 0), (0, 1), (1, 1), (2, 1)],
			},
			1 => PieceDef {
				cells: [(1, 0), (1, 1), (0, 2), (1, 2)],
			},
			2 => PieceDef {
				cells: [(0, 0), (1, 0), (2, 0), (2, 1)],
			},
			3 => PieceDef {
				cells: [(0, 0), (1, 0), (0, 1), (0, 2)],
			},
			_ => unreachable!(),
		},
		PieceKind::S => {
			// S toggles rot 0 <-> 2; must be two distinct orientations (flat / vertical).
			if r == 0 {
				PieceDef {
					cells: [(1, 0), (2, 0), (0, 1), (1, 1)],
				}
			} else if r == 2 {
				PieceDef {
					cells: [(1, 0), (1, 1), (2, 1), (2, 2)],
				}
			} else {
				PieceDef {
					cells: [(1, 0), (1, 1), (2, 1), (2, 2)],
				}
			}
		}
		PieceKind::Z => {
			if r == 0 {
				PieceDef {
					cells: [(0, 0), (1, 0), (1, 1), (2, 1)],
				}
			} else if r == 2 {
				PieceDef {
					cells: [(2, 0), (1, 1), (2, 1), (1, 2)],
				}
			} else {
				PieceDef {
					cells: [(2, 0), (1, 1), (2, 1), (1, 2)],
				}
			}
		}
	}
}

pub fn rotate_cw(kind: PieceKind, rot: RotIndex) -> RotIndex {
	match kind {
		PieceKind::O => rot,
		PieceKind::I | PieceKind::S | PieceKind::Z => (rot + 2) % 4,
		_ => (rot + 1) % 4,
	}
}

pub fn rotate_ccw(kind: PieceKind, rot: RotIndex) -> RotIndex {
	match kind {
		PieceKind::O => rot,
		PieceKind::I | PieceKind::S | PieceKind::Z => (rot + 2) % 4,
		_ => (rot + 3) % 4,
	}
}

/// Spawn origin (bottom-left of bbox) — tuned for 10-wide field (TGM-style).
///
/// For [`PieceKind::I`], horizontal (`rot % 4 == 0`) uses `y = 18` so cells sit in rows 18–19.
/// Vertical orientations need `y = 17` so the column fits rows 17–20 (buffer row 20); at `y = 18`
/// the top cell would be row 21 and collide with the ceiling.
///
/// All other kinds use `y = 18` so two-row default orientations match the O-piece entry height
/// (previously T/L/J/S/Z used `y = 17` and appeared one row lower than O and horizontal I).
pub fn spawn_origin(kind: PieceKind, rot: RotIndex) -> (i32, i32) {
	match kind {
		PieceKind::I => {
			if rot.is_multiple_of(4) {
				(3, 18)
			} else {
				(3, 17)
			}
		}
		_ => (3, 18),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn s_z_i_toggle_distinct_shapes() {
		let s0 = piece_cells(PieceKind::S, 0).cells;
		let s2 = piece_cells(PieceKind::S, 2).cells;
		assert_ne!(s0, s2);

		let z0 = piece_cells(PieceKind::Z, 0).cells;
		let z2 = piece_cells(PieceKind::Z, 2).cells;
		assert_ne!(z0, z2);

		let i0 = piece_cells(PieceKind::I, 0).cells;
		let i2 = piece_cells(PieceKind::I, 2).cells;
		assert_ne!(i0, i2);
	}

	#[test]
	fn sz_rotate_cw_toggles() {
		assert_eq!(rotate_cw(PieceKind::S, 0), 2);
		assert_eq!(rotate_cw(PieceKind::S, 2), 0);
	}

	#[test]
	fn i_vertical_spawn_fits_in_board_height() {
		let (_, py) = spawn_origin(PieceKind::I, 2);
		let def = piece_cells(PieceKind::I, 2);
		let max_y = def
			.cells
			.iter()
			.map(|(_, dy)| py + *dy as i32)
			.max()
			.unwrap();
		assert!(
			max_y < crate::constants::BOARD_HEIGHT as i32,
			"vertical I must not extend past row {}",
			crate::constants::BOARD_HEIGHT - 1
		);
	}
}
