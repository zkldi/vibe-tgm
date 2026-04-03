//! ARS rotation: basic, then kick +1 right, then −1 left. I-piece never kicks.

use crate::board::collides_def;
use crate::piece::{piece_cells, rotate_ccw, rotate_cw, PieceKind, RotIndex};
use crate::Board;

pub fn try_rotate_cw(
    board: &Board,
    px: i32,
    py: i32,
    kind: PieceKind,
    rot: RotIndex,
) -> Option<(i32, i32, RotIndex)> {
    try_rotate(board, px, py, kind, rot, true)
}

pub fn try_rotate_ccw(
    board: &Board,
    px: i32,
    py: i32,
    kind: PieceKind,
    rot: RotIndex,
) -> Option<(i32, i32, RotIndex)> {
    try_rotate(board, px, py, kind, rot, false)
}

fn try_rotate(
    board: &Board,
    px: i32,
    py: i32,
    kind: PieceKind,
    rot: RotIndex,
    cw: bool,
) -> Option<(i32, i32, RotIndex)> {
    let new_rot = if cw {
        rotate_cw(kind, rot)
    } else {
        rotate_ccw(kind, rot)
    };
    let new_def = piece_cells(kind, new_rot);

    if kind == PieceKind::I {
        if !collides_def(board, px, py, &new_def) {
            return Some((px, py, new_rot));
        }
        return None;
    }

    if !collides_def(board, px, py, &new_def) {
        return Some((px, py, new_rot));
    }
    if !collides_def(board, px + 1, py, &new_def) {
        return Some((px + 1, py, new_rot));
    }
    if !collides_def(board, px - 1, py, &new_def) {
        return Some((px - 1, py, new_rot));
    }
    None
}
