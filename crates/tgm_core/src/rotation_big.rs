//! ARS rotation for Big mode 5-wide field.

use crate::board_big::{BoardBig, collides_def_big};
use crate::piece::{PieceKind, RotIndex, rotate_ccw, rotate_cw};
use crate::piece_big::piece_cells_big;

pub fn try_rotate_cw_big(
	board: &BoardBig,
	px: i32,
	py: i32,
	kind: PieceKind,
	rot: RotIndex,
) -> Option<(i32, i32, RotIndex)> {
	try_rotate_big(board, px, py, kind, rot, true)
}

pub fn try_rotate_ccw_big(
	board: &BoardBig,
	px: i32,
	py: i32,
	kind: PieceKind,
	rot: RotIndex,
) -> Option<(i32, i32, RotIndex)> {
	try_rotate_big(board, px, py, kind, rot, false)
}

fn try_rotate_big(
	board: &BoardBig,
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
	let new_def = piece_cells_big(kind, new_rot);

	if kind == PieceKind::I {
		if !collides_def_big(board, px, py, &new_def) {
			return Some((px, py, new_rot));
		}
		if !collides_def_big(board, px, py - 1, &new_def) {
			return Some((px, py - 1, new_rot));
		}
		return None;
	}

	if !collides_def_big(board, px, py, &new_def) {
		return Some((px, py, new_rot));
	}
	if !collides_def_big(board, px + 1, py, &new_def) {
		return Some((px + 1, py, new_rot));
	}
	if !collides_def_big(board, px - 1, py, &new_def) {
		return Some((px - 1, py, new_rot));
	}
	None
}
