//! Tetromino definitions for TGM1 Big mode (5-wide logical field).

use crate::piece::{PieceKind, RotIndex};

#[derive(Clone, Copy, Debug)]
pub struct PieceDefBig {
	pub cells: [(i8, i8); 4],
}

pub fn piece_cells_big(kind: PieceKind, rot: RotIndex) -> PieceDefBig {
	let r = rot % 4;
	match kind {
		PieceKind::I => {
			if r == 0 {
				PieceDefBig {
					cells: [(0, 1), (1, 1), (2, 1), (3, 1)],
				}
			} else if r == 2 {
				PieceDefBig {
					cells: [(2, 0), (2, 1), (2, 2), (2, 3)],
				}
			} else {
				PieceDefBig {
					cells: [(2, 0), (2, 1), (2, 2), (2, 3)],
				}
			}
		}
		PieceKind::O => PieceDefBig {
			cells: [(1, 0), (2, 0), (1, 1), (2, 1)],
		},
		PieceKind::T => match r {
			0 => PieceDefBig {
				cells: [(1, 0), (0, 1), (1, 1), (2, 1)],
			},
			1 => PieceDefBig {
				cells: [(1, 0), (0, 1), (1, 1), (1, 2)],
			},
			2 => PieceDefBig {
				cells: [(0, 0), (1, 0), (2, 0), (1, 1)],
			},
			3 => PieceDefBig {
				cells: [(1, 0), (1, 1), (1, 2), (2, 1)],
			},
			_ => unreachable!(),
		},
		PieceKind::L => match r {
			0 => PieceDefBig {
				cells: [(2, 0), (0, 1), (1, 1), (2, 1)],
			},
			1 => PieceDefBig {
				cells: [(0, 0), (1, 0), (1, 1), (1, 2)],
			},
			2 => PieceDefBig {
				cells: [(0, 0), (1, 0), (2, 0), (0, 1)],
			},
			3 => PieceDefBig {
				cells: [(0, 0), (0, 1), (0, 2), (1, 2)],
			},
			_ => unreachable!(),
		},
		PieceKind::J => match r {
			0 => PieceDefBig {
				cells: [(0, 0), (0, 1), (1, 1), (2, 1)],
			},
			1 => PieceDefBig {
				cells: [(1, 0), (1, 1), (0, 2), (1, 2)],
			},
			2 => PieceDefBig {
				cells: [(0, 0), (1, 0), (2, 0), (2, 1)],
			},
			3 => PieceDefBig {
				cells: [(0, 0), (1, 0), (0, 1), (0, 2)],
			},
			_ => unreachable!(),
		},
		PieceKind::S => {
			if r == 0 {
				PieceDefBig {
					cells: [(1, 0), (2, 0), (0, 1), (1, 1)],
				}
			} else if r == 2 {
				PieceDefBig {
					cells: [(1, 0), (1, 1), (2, 1), (2, 2)],
				}
			} else {
				PieceDefBig {
					cells: [(1, 0), (1, 1), (2, 1), (2, 2)],
				}
			}
		}
		PieceKind::Z => {
			if r == 0 {
				PieceDefBig {
					cells: [(0, 0), (1, 0), (1, 1), (2, 1)],
				}
			} else if r == 2 {
				PieceDefBig {
					cells: [(2, 0), (1, 1), (2, 1), (1, 2)],
				}
			} else {
				PieceDefBig {
					cells: [(2, 0), (1, 1), (2, 1), (1, 2)],
				}
			}
		}
	}
}

/// Spawn origin for 5-wide × 11-tall big field (buffer row 10).
pub fn spawn_origin_big(kind: PieceKind, rot: RotIndex) -> (i32, i32) {
	match kind {
		PieceKind::I => {
			if rot % 4 == 0 {
				(0, 8)
			} else {
				(1, 7)
			}
		}
		_ => (1, 8),
	}
}

/// Big mode reverse: spawn at bottom of field.
pub fn spawn_origin_big_rev(kind: PieceKind, rot: RotIndex) -> (i32, i32) {
	match kind {
		PieceKind::I => {
			if rot % 4 == 0 {
				(0, 1)
			} else {
				(1, 0)
			}
		}
		_ => (1, 0),
	}
}
