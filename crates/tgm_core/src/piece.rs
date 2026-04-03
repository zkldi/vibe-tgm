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
            // Longest side horizontal at rot 0; vertical at rot 2.
            if r == 0 || r == 2 {
                PieceDef {
                    cells: [(0, 1), (1, 1), (2, 1), (3, 1)],
                }
            } else {
                PieceDef {
                    cells: [(2, 0), (2, 1), (2, 2), (2, 3)],
                }
            }
        }
        PieceKind::O => PieceDef {
            cells: [(1, 0), (2, 0), (1, 1), (2, 1)],
        },
        PieceKind::T => match r {
            0 => PieceDef {
                cells: [(1, 0), (0, 1), (1, 1), (2, 1)],
            },
            1 => PieceDef {
                cells: [(1, 0), (1, 1), (2, 1), (1, 2)],
            },
            2 => PieceDef {
                cells: [(0, 1), (1, 1), (2, 1), (1, 2)],
            },
            _ => PieceDef {
                cells: [(1, 0), (0, 1), (1, 1), (1, 2)],
            },
        },
        PieceKind::L => match r {
            0 => PieceDef {
                cells: [(2, 0), (0, 1), (1, 1), (2, 1)],
            },
            1 => PieceDef {
                cells: [(1, 0), (1, 1), (1, 2), (2, 2)],
            },
            2 => PieceDef {
                cells: [(0, 1), (1, 1), (2, 1), (0, 2)],
            },
            _ => PieceDef {
                cells: [(0, 0), (1, 0), (1, 1), (1, 2)],
            },
        },
        PieceKind::J => match r {
            0 => PieceDef {
                cells: [(0, 0), (0, 1), (1, 1), (2, 1)],
            },
            1 => PieceDef {
                cells: [(1, 0), (2, 0), (1, 1), (1, 2)],
            },
            2 => PieceDef {
                cells: [(0, 1), (1, 1), (2, 1), (2, 2)],
            },
            _ => PieceDef {
                cells: [(1, 0), (1, 1), (1, 2), (2, 0)],
            },
        },
        PieceKind::S => {
            if r == 0 || r == 2 {
                PieceDef {
                    cells: [(1, 0), (2, 0), (0, 1), (1, 1)],
                }
            } else {
                PieceDef {
                    cells: [(1, 0), (1, 1), (2, 1), (2, 2)],
                }
            }
        }
        PieceKind::Z => {
            if r == 0 || r == 2 {
                PieceDef {
                    cells: [(0, 0), (1, 0), (1, 1), (2, 1)],
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
pub fn spawn_origin(kind: PieceKind) -> (i32, i32) {
    match kind {
        PieceKind::I => (3, 18),
        PieceKind::O => (3, 18),
        _ => (3, 17),
    }
}
