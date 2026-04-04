//! Autoplay for normal (non-big, non-reverse) mode: goal-directed BFS to a chosen lock position.
//! Candidates are ordered by line clears first (so tetrises beat singles), then stack heuristic.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::board::{Board, clear_lines, count_full_lines, find_full_lines};
use crate::constants::{DAS_FRAMES, DAS_REPEAT_FRAMES, LOCK_DELAY_FRAMES};
use crate::game::{Game, Input, Phase};
use crate::gravity::effective_gravity;
use crate::options::GameOptions;
use crate::piece::{PieceKind, RotIndex};
use crate::rotation::{try_rotate_ccw, try_rotate_cw};

use crate::EMPTY;

/// Total BFS expansions per piece (shared across candidate goals — avoids multi-second stalls).
const MAX_BFS_NODES_TOTAL: usize = 80_000;
/// Hard cap per goal attempt so one unreachable placement cannot burn the whole budget.
const MAX_NODES_PER_GOAL: usize = 35_000;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct SimState {
	x: i32,
	y: i32,
	rot: u8,
	lock_delay: u32,
	das_left: u32,
	das_right: u32,
	gravity_accum: u32,
}

enum StepOutcome {
	Falling(SimState),
	Locked { x: i32, y: i32, rot: u8 },
}

fn advance_das_horizontal(s: &mut SimState, input: Input) -> (bool, bool) {
	let mut move_left = false;
	let mut move_right = false;
	if input.left {
		s.das_right = 0;
		s.das_left = s.das_left.saturating_add(1);
		move_left = s.das_left == 1
			|| (s.das_left >= DAS_FRAMES
				&& (s.das_left - DAS_FRAMES) % DAS_REPEAT_FRAMES == 0);
	} else {
		s.das_left = 0;
	}
	if input.right {
		s.das_left = 0;
		s.das_right = s.das_right.saturating_add(1);
		move_right = s.das_right == 1
			|| (s.das_right >= DAS_FRAMES
				&& (s.das_right - DAS_FRAMES) % DAS_REPEAT_FRAMES == 0);
	} else {
		s.das_right = 0;
	}
	(move_left, move_right)
}

/// One frame of [`crate::game::Game::step_falling_normal_fwd`] without side effects.
fn step_falling_sim(
	board: &Board,
	level: u16,
	opts: &GameOptions,
	kind: PieceKind,
	mut s: SimState,
	input: Input,
) -> StepOutcome {
	let g = effective_gravity(level, opts);

	let mut p_x = s.x;
	let mut p_y = s.y;
	let mut p_rot = s.rot;
	let mut lock_delay = s.lock_delay;
	let mut gravity_accum = s.gravity_accum;

	if input.rot_ccw {
		if let Some((nx, ny, nr)) = try_rotate_ccw(board, p_x, p_y, kind, p_rot) {
			p_x = nx;
			p_y = ny;
			p_rot = nr;
		}
	} else if input.rot_cw {
		if let Some((nx, ny, nr)) = try_rotate_cw(board, p_x, p_y, kind, p_rot) {
			p_x = nx;
			p_y = ny;
			p_rot = nr;
		}
	}

	let (move_left, move_right) = advance_das_horizontal(&mut s, input);

	if move_left && !board.collides(p_x - 1, p_y, kind, p_rot) {
		p_x -= 1;
	}
	if move_right && !board.collides(p_x + 1, p_y, kind, p_rot) {
		p_x += 1;
	}

	if input.sonic {
		let ny = board.drop_to_bottom(p_x, p_y, kind, p_rot);
		if ny != p_y {
			p_y = ny;
			lock_delay = LOCK_DELAY_FRAMES;
		}
	}

	if g >= 5120 {
		let ny = board.drop_to_bottom(p_x, p_y, kind, p_rot);
		p_y = ny;
		p_y = board.rise_to_valid(p_x, p_y, kind, p_rot);
	} else if input.down {
		if !board.collides(p_x, p_y - 1, kind, p_rot) {
			p_y -= 1;
			lock_delay = LOCK_DELAY_FRAMES;
		}
	} else {
		gravity_accum = gravity_accum.saturating_add(g as u32);
		while gravity_accum >= 256 {
			gravity_accum -= 256;
			if !board.collides(p_x, p_y - 1, kind, p_rot) {
				p_y -= 1;
				lock_delay = LOCK_DELAY_FRAMES;
			} else {
				break;
			}
		}
	}

	let grounded = board.collides(p_x, p_y - 1, kind, p_rot);
	if grounded {
		if input.down {
			return StepOutcome::Locked {
				x: p_x,
				y: p_y,
				rot: p_rot,
			};
		}
		if lock_delay > 0 {
			lock_delay -= 1;
		}
		if lock_delay == 0 {
			return StepOutcome::Locked {
				x: p_x,
				y: p_y,
				rot: p_rot,
			};
		}
	}

	StepOutcome::Falling(SimState {
		x: p_x,
		y: p_y,
		rot: p_rot,
		lock_delay,
		das_left: s.das_left,
		das_right: s.das_right,
		gravity_accum,
	})
}

#[cfg(test)]
fn all_inputs() -> impl Iterator<Item = Input> {
	let mut v = Vec::with_capacity(40);
	for rot in 0u8..3u8 {
		for h in 0u8..3u8 {
			for vert in 0u8..4u8 {
				let mut inp = Input::default();
				match rot {
					0 => {}
					1 => inp.rot_cw = true,
					2 => inp.rot_ccw = true,
					_ => unreachable!(),
				}
				match h {
					0 => {}
					1 => inp.left = true,
					2 => inp.right = true,
					_ => unreachable!(),
				}
				match vert {
					0 => {}
					1 => inp.down = true,
					2 => inp.sonic = true,
					3 => {
						inp.down = true;
						inp.sonic = true;
					}
					_ => unreachable!(),
				}
				v.push(inp);
			}
		}
	}
	v.into_iter()
}

fn board_heuristic(board: &Board) -> i32 {
	let mut col_h = [0i32; crate::constants::BOARD_WIDTH];
	for x in 0..crate::constants::BOARD_WIDTH {
		let mut h = 0;
		for y in 0..crate::constants::BOARD_HEIGHT {
			if board.rows[y][x] != EMPTY {
				h = (y + 1) as i32;
			}
		}
		col_h[x] = h;
	}
	let agg: i32 = col_h.iter().sum();
	let bumps: i32 = col_h.windows(2).map(|w| (w[0] - w[1]).abs()).sum();
	let holes = count_holes_board(board);
	agg * 10 + holes * 40 + bumps * 2
}

fn count_holes_board(board: &Board) -> i32 {
	let mut n = 0;
	for x in 0..crate::constants::BOARD_WIDTH {
		let mut seen = false;
		for y in (0..crate::constants::BOARD_HEIGHT).rev() {
			if board.rows[y][x] != EMPTY {
				seen = true;
			} else if seen {
				n += 1;
			}
		}
	}
	n
}

/// Lines cleared and resulting stack heuristic (lower is better). Sort by `(lines DESC, h ASC)`.
fn evaluate_lock(board: &Board, px: i32, py: i32, kind: PieceKind, rot: RotIndex) -> (u32, i32) {
	let mut b = board.clone();
	let color = kind as u8 + 1;
	b.lock_piece(px, py, kind, rot, color);
	let full = find_full_lines(&b);
	let n = count_full_lines(&full);
	if n > 0 {
		clear_lines(&mut b, &full);
	}
	(n, board_heuristic(&b))
}

fn reconstruct_path(
	parent: &HashMap<SimState, (SimState, Input)>,
	start: &SimState,
	s: &SimState,
) -> Vec<Input> {
	let mut out = Vec::new();
	let mut cur = s.clone();
	while cur != *start {
		let Some((prev, inp)) = parent.get(&cur) else {
			break;
		};
		out.push(*inp);
		cur = prev.clone();
	}
	out.reverse();
	out
}

/// Resting row after sonic drop from above the stack: scan downward until the piece fits in the
/// well, then hard-drop. (The previous client helper moved `py` the wrong way when resolving
/// ceiling collision, yielding no placements on an empty board.)
fn landing_py(board: &Board, px: i32, kind: PieceKind, rot: u8) -> Option<i32> {
	let mut py = (crate::constants::BOARD_HEIGHT as i32) + 4;
	for _ in 0..64 {
		if !board.collides(px, py, kind, rot) {
			return Some(board.drop_to_bottom(px, py, kind, rot));
		}
		py -= 1;
	}
	None
}

fn relevant_rots(kind: PieceKind) -> &'static [u8] {
	match kind {
		PieceKind::O => &[0],
		PieceKind::I | PieceKind::S | PieceKind::Z => &[0, 2],
		PieceKind::T | PieceKind::L | PieceKind::J => &[0, 1, 2, 3],
	}
}

fn candidate_placements(board: &Board, kind: PieceKind) -> Vec<(i32, i32, u8, u32, i32)> {
	let mut out = Vec::new();
	for &rot in relevant_rots(kind) {
		for px in -4i32..(crate::constants::BOARD_WIDTH as i32 + 4) {
			let Some(py) = landing_py(board, px, kind, rot) else {
				continue;
			};
			let (lines, h) = evaluate_lock(board, px, py, kind, rot);
			out.push((px, py, rot, lines, h));
		}
	}
	// Prefer more line clears (tetris over triple over …), then lower stack heuristic.
	out.sort_by(|a, b| b.3.cmp(&a.3).then(a.4.cmp(&b.4)));
	out.dedup_by(|a, b| (a.0, a.1, a.2) == (b.0, b.1, b.2));
	out
}

fn bfs_to_goal(
	board: &Board,
	level: u16,
	opts: &GameOptions,
	kind: PieceKind,
	start: &SimState,
	goal: (i32, i32, u8),
	budget: &mut usize,
) -> Option<Vec<Input>> {
	let mut queue = VecDeque::with_capacity(4096);
	let mut visited = HashSet::with_capacity(32_768);
	let mut parent: HashMap<SimState, (SimState, Input)> = HashMap::with_capacity(32_768);

	queue.push_back(start.clone());
	visited.insert(start.clone());

	let mut nodes_this_goal: usize = 0;

	while let Some(s) = queue.pop_front() {
		if *budget == 0 {
			return None;
		}
		*budget -= 1;
		nodes_this_goal += 1;
		if nodes_this_goal > MAX_NODES_PER_GOAL {
			return None;
		}

		for &inp in bfs_inputs() {
			match step_falling_sim(board, level, opts, kind, s.clone(), inp) {
				StepOutcome::Falling(next) => {
					if visited.insert(next.clone()) {
						parent.insert(next.clone(), (s.clone(), inp));
						queue.push_back(next);
					}
				}
				StepOutcome::Locked { x, y, rot } => {
					if x == goal.0 && y == goal.1 && rot == goal.2 {
						let mut path = reconstruct_path(&parent, start, &s);
						path.push(inp);
						return Some(path);
					}
				}
			}
		}
	}

	None
}

/// Smaller than the full 3×3×4 Cartesian product: enough for finesse, far fewer branches per node.
fn bfs_inputs() -> &'static [Input] {
	const fn inp(
		left: bool,
		right: bool,
		down: bool,
		sonic: bool,
		rot_cw: bool,
		rot_ccw: bool,
	) -> Input {
		Input {
			left,
			right,
			down,
			sonic,
			rot_cw,
			rot_ccw,
		}
	}
	// 20 inputs: single actions + common chord moves + drop variants.
	static BFS: [Input; 20] = [
		inp(false, false, false, false, false, false),
		inp(true, false, false, false, false, false),
		inp(false, true, false, false, false, false),
		inp(false, false, false, false, true, false),
		inp(false, false, false, false, false, true),
		inp(true, false, false, false, true, false),
		inp(true, false, false, false, false, true),
		inp(false, true, false, false, true, false),
		inp(false, true, false, false, false, true),
		inp(false, false, true, false, false, false),
		inp(false, false, false, true, false, false),
		inp(false, false, true, true, false, false),
		inp(true, false, true, false, false, false),
		inp(false, true, true, false, false, false),
		inp(true, false, true, true, false, false),
		inp(false, true, true, true, false, false),
		inp(false, false, true, false, true, false),
		inp(false, false, true, false, false, true),
		inp(false, false, false, true, true, false),
		inp(false, false, false, true, false, true),
	];
	&BFS
}

/// Returns a full input sequence for the current piece until lock, or `None` if search failed.
pub fn autoplay_plan_inputs(g: &Game) -> Option<Vec<Input>> {
	if g.game_over || g.phase != Phase::Falling {
		return None;
	}
	if g.options.big || g.options.reverse {
		return None;
	}
	let p = g.piece?;

	let start = SimState {
		x: p.x,
		y: p.y,
		rot: p.rot,
		lock_delay: g.lock_delay,
		das_left: g.das_left,
		das_right: g.das_right,
		gravity_accum: g.gravity_accum,
	};

	let board = &g.board;
	let kind = p.kind;

	let candidates = candidate_placements(board, kind);
	let mut budget = MAX_BFS_NODES_TOTAL;
	for (gx, gy, gr, _, _) in candidates {
		if budget == 0 {
			break;
		}
		if let Some(path) =
			bfs_to_goal(board, g.level, &g.options, kind, &start, (gx, gy, gr), &mut budget)
		{
			return Some(path);
		}
	}

	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::game::Input;

	fn game_with_falling_piece(seed: u64) -> Game {
		let mut g = Game::new(seed);
		for _ in 0..500 {
			g.step(Input::default());
			if g.phase == Phase::Falling && g.piece.is_some() {
				return g;
			}
			if g.game_over {
				break;
			}
		}
		panic!("could not reach Falling with a piece");
	}

	fn sim_matches_game(g: &mut Game, inp: Input) -> bool {
		let p = g.piece.expect("piece");
		let s0 = SimState {
			x: p.x,
			y: p.y,
			rot: p.rot,
			lock_delay: g.lock_delay,
			das_left: g.das_left,
			das_right: g.das_right,
			gravity_accum: g.gravity_accum,
		};
		let kind = p.kind;
		let board = g.board.clone();
		let level = g.level;
		let opts = g.options;
		let out = step_falling_sim(&board, level, &opts, kind, s0, inp);
		g.step(inp);
		match out {
			StepOutcome::Falling(ns) => {
				if g.game_over || g.phase != Phase::Falling {
					return false;
				}
				let p2 = g.piece.expect("piece");
				p2.x == ns.x
					&& p2.y == ns.y
					&& p2.rot == ns.rot
					&& g.lock_delay == ns.lock_delay
					&& g.das_left == ns.das_left
					&& g.das_right == ns.das_right
					&& g.gravity_accum == ns.gravity_accum
			}
			StepOutcome::Locked { .. } => g.piece.is_none(),
		}
	}

	#[test]
	fn step_sim_matches_game_single_steps() {
		let g = game_with_falling_piece(42);
		for inp in all_inputs().take(36) {
			let mut g2 = g.clone();
			if !sim_matches_game(&mut g2, inp) {
				panic!("mismatch for input {:?}", inp);
			}
		}
	}

	#[test]
	fn autoplay_finds_plan_on_empty_board() {
		let g = game_with_falling_piece(99);
		let plan = autoplay_plan_inputs(&g);
		assert!(plan.is_some());
		let p = plan.unwrap();
		assert!(!p.is_empty());
	}

	#[test]
	fn natural_lock_is_always_a_candidate() {
		let g = game_with_falling_piece(99);
		let p = g.piece.expect("piece");
		let kind = p.kind;
		let board = g.board.clone();
		let level = g.level;
		let opts = g.options;
		let mut s = SimState {
			x: p.x,
			y: p.y,
			rot: p.rot,
			lock_delay: g.lock_delay,
			das_left: g.das_left,
			das_right: g.das_right,
			gravity_accum: g.gravity_accum,
		};
		let (lx, ly, lr) = loop {
			match step_falling_sim(&board, level, &opts, kind, s.clone(), Input::default()) {
				StepOutcome::Falling(ns) => s = ns,
				StepOutcome::Locked { x, y, rot } => break (x, y, rot),
			}
		};
		let cands = candidate_placements(&board, kind);
		assert!(
			cands
				.iter()
				.any(|(cx, cy, cr, _, _)| *cx == lx && *cy == ly && *cr == lr),
			"natural lock ({},{},{}) not in candidates: first few {:?}",
			lx,
			ly,
			lr,
			&cands[..cands.len().min(5)]
		);
	}
}
