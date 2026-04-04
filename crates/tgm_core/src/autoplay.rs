//! Autoplay: survival-first placement ranking with 1-ply lookahead and BFS pathfinding.
//!
//! Each candidate placement is ranked by the resulting board quality (higher = better).
//! The evaluation strongly penalises **holes** (empty cells below the stack surface),
//! **aggregate height**, and **bumpiness**, while rewarding line clears (with extra
//! weight when the stack was already high before the lock).  At 20G
//! (level >= 500), a pyramid preference keeps column 5 at the peak so pieces can
//! slide laterally ([Lesson 6](https://tgm.tips/6.htm),
//! [Lesson 10](https://tgm.tips/10.htm)).
//!
//! For each piece the bot ranks all reachable (x, rot) placements, then tries a fast
//! **macro path** (rotate -> shift -> drop) and, if that fails, a full **BFS** over
//! frame-level inputs.  At 20G the BFS uses a compact state (x, rot, lock_delay, DAS)
//! since y is implied by the floor.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::EMPTY;
use crate::board::{Board, clear_lines, count_full_lines, find_full_lines};
use crate::constants::{
	BOARD_HEIGHT, BOARD_WIDTH, DAS_FRAMES, DAS_REPEAT_FRAMES, LOCK_DELAY_FRAMES,
};
use crate::game::{Game, Input, Phase};
use crate::gravity::effective_gravity;
use crate::piece::{PieceKind, rotate_ccw, rotate_cw, spawn_origin};
use crate::rotation::{try_rotate_ccw, try_rotate_cw};

// ---------------------------------------------------------------------------
// Tuning knobs
// ---------------------------------------------------------------------------

const MAX_BFS_NODES_TOTAL: usize = 100_000;
const MAX_NODES_PER_GOAL: usize = 25_000;
const MAX_CANDIDATES_PATHFIND: usize = 24;
const W_LOOKAHEAD_DIV: i64 = 3;
/// Stand-in for 1-ply lookahead when the true next-next piece is not exposed on `Game`.
const LOOKAHEAD_PLACEHOLDER: PieceKind = PieceKind::T;
/// Below this column height, line-clear bonus uses the base value only (no danger ramp).
const LINE_CLEAR_DANGER_FLOOR: i64 = 6;

// ---------------------------------------------------------------------------
// Simulation types (frame-level physics for pathfinding)
// ---------------------------------------------------------------------------

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

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct SimState20g {
	x: i32,
	rot: u8,
	lock_delay: u32,
	das_left: u32,
	das_right: u32,
}

enum StepOutcome {
	Falling(SimState),
	Locked { x: i32, y: i32, rot: u8 },
}

// ---------------------------------------------------------------------------
// Frame-level simulation (unchanged from original — matches Game::step_falling)
// ---------------------------------------------------------------------------

fn advance_das_horizontal(s: &mut SimState, input: Input) -> (bool, bool) {
	let mut move_left = false;
	let mut move_right = false;
	if input.left {
		s.das_right = 0;
		s.das_left = s.das_left.saturating_add(1);
		move_left = s.das_left == 1
			|| (s.das_left >= DAS_FRAMES && (s.das_left - DAS_FRAMES).is_multiple_of(DAS_REPEAT_FRAMES));
	} else {
		s.das_left = 0;
	}
	if input.right {
		s.das_left = 0;
		s.das_right = s.das_right.saturating_add(1);
		move_right = s.das_right == 1
			|| (s.das_right >= DAS_FRAMES && (s.das_right - DAS_FRAMES).is_multiple_of(DAS_REPEAT_FRAMES));
	} else {
		s.das_right = 0;
	}
	(move_left, move_right)
}

fn step_falling_sim(
	board: &Board,
	level: u16,
	kind: PieceKind,
	mut s: SimState,
	input: Input,
) -> StepOutcome {
	let g = effective_gravity(level);

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
		let old_y = p_y;
		p_y = board.drop_to_bottom(p_x, p_y, kind, p_rot);
		p_y = board.rise_to_valid(p_x, p_y, kind, p_rot);
		if p_y < old_y {
			lock_delay = LOCK_DELAY_FRAMES;
		}
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
		lock_delay = lock_delay.saturating_sub(1);
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

fn sim_state_from_20g(board: &Board, kind: PieceKind, s: &SimState20g) -> Option<SimState> {
	let y = landing_py(board, s.x, kind, s.rot)?;
	Some(SimState {
		x: s.x,
		y,
		rot: s.rot,
		lock_delay: s.lock_delay,
		das_left: s.das_left,
		das_right: s.das_right,
		gravity_accum: 0,
	})
}

fn to_20g_compact(s: &SimState) -> SimState20g {
	SimState20g {
		x: s.x,
		rot: s.rot,
		lock_delay: s.lock_delay,
		das_left: s.das_left,
		das_right: s.das_right,
	}
}

// ---------------------------------------------------------------------------
// Board metrics
// ---------------------------------------------------------------------------

fn col_heights(board: &Board) -> [i32; BOARD_WIDTH] {
	let mut h = [0i32; BOARD_WIDTH];
	for x in 0..BOARD_WIDTH {
		for y in 0..BOARD_HEIGHT {
			if board.rows[y][x] != EMPTY {
				h[x] = (y + 1) as i32;
			}
		}
	}
	h
}

/// Empty cells below the column surface.  The single most important survival metric.
fn count_holes(board: &Board, h: &[i32; BOARD_WIDTH]) -> i32 {
	let mut count = 0;
	for x in 0..BOARD_WIDTH {
		let top = h[x] as usize;
		for y in 0..top {
			if board.rows[y][x] == EMPTY {
				count += 1;
			}
		}
	}
	count
}

/// For each hole, count filled cells above it in the same column (deeper = harder to fix).
fn hole_depth_sum(board: &Board, h: &[i32; BOARD_WIDTH]) -> i32 {
	let mut sum = 0;
	for x in 0..BOARD_WIDTH {
		let top = h[x] as usize;
		let mut filled_above = 0i32;
		for y in (0..top).rev() {
			if board.rows[y][x] != EMPTY {
				filled_above += 1;
			} else {
				sum += filled_above;
			}
		}
	}
	sum
}

fn landing_py(board: &Board, px: i32, kind: PieceKind, rot: u8) -> Option<i32> {
	let mut py = (BOARD_HEIGHT as i32) + 4;
	for _ in 0..64 {
		if !board.collides(px, py, kind, rot) {
			return Some(board.drop_to_bottom(px, py, kind, rot));
		}
		py -= 1;
	}
	None
}

// ---------------------------------------------------------------------------
// Placement evaluation  (higher = better board)
// ---------------------------------------------------------------------------

fn evaluate_board(
	board: &Board,
	lines_cleared: u32,
	level: u16,
	max_h_before_move: i64,
) -> i64 {
	let h = col_heights(board);
	let max_h = *h.iter().max().unwrap_or(&0) as i64;
	let agg: i64 = h.iter().map(|&x| x as i64).sum();
	let holes = count_holes(board, &h) as i64;
	let depth = hole_depth_sum(board, &h) as i64;
	let bumps: i64 = h
		.windows(2)
		.map(|w| (w[0] as i64 - w[1] as i64).abs())
		.sum();

	let is_20g = effective_gravity(level) >= 5120;
	let mut score: i64 = 0;

	// Extra reward for clears when the stack was already high before this lock — avoids
	// pyramid/bump penalties outweighing an urgent clear.
	let danger = (max_h_before_move - LINE_CLEAR_DANGER_FLOOR).max(0);
	let line_per = if is_20g {
		1400 + danger * 90
	} else {
		650 + danger * 70
	};
	if lines_cleared > 0 {
		score += lines_cleared as i64 * line_per;
		if lines_cleared == 4 {
			score += if is_20g {
				2400 + danger * 200
			} else {
				1500 + danger * 150
			};
		}
	}

	if is_20g {
		// At 20G holes are nearly unfixable and smooth terrain is critical for movement.
		score -= holes * 1600;
		score -= depth * 300;
		score -= agg * 10;
		score -= bumps * 40;

		let excess = (max_h - 5).max(0);
		score -= excess * excess * 50;

		// Pyramid: column 5 (index 4) must be the peak so pieces can slide both ways.
		let h4 = h[4] as i64;
		let not_peak = (max_h - h4).max(0);
		score -= not_peak * not_peak * 200;

		// Any column taller than column 5 blocks lateral movement.
		for x in 0..BOARD_WIDTH {
			if x == 4 {
				continue;
			}
			let over = (h[x] as i64 - h4).max(0);
			score -= over * over * 70;
		}

		// Center valley (column 5 lower than neighbours) traps pieces.
		let center_neigh = h[3].max(h[5]) as i64;
		let center_valley = (center_neigh - h4).max(0);
		score -= center_valley * center_valley * 160;
	} else {
		score -= holes * 600;
		score -= depth * 120;
		score -= agg * 5;
		score -= bumps * 18;

		let excess = (max_h - 8).max(0);
		score -= excess * excess * 25;
	}

	score
}

/// Best 1-ply evaluation for the next piece on the resulting board.
fn best_next_eval(board: &Board, kind: PieceKind, level: u16) -> i64 {
	let max_before = *col_heights(board).iter().max().unwrap_or(&0) as i64;
	let mut best = i64::MIN / 4;
	for &rot in relevant_rots(kind) {
		for px in -2..(BOARD_WIDTH as i32 + 2) {
			let Some(py) = landing_py(board, px, kind, rot) else {
				continue;
			};
			let mut b = board.clone();
			b.lock_piece(px, py, kind, rot, 1);
			let full = find_full_lines(&b);
			let n = count_full_lines(&full);
			if n > 0 {
				clear_lines(&mut b, &full);
			}
			let eval = evaluate_board(&b, n, level, max_before);
			if eval > best {
				best = eval;
			}
		}
	}
	best
}

fn rank_placement(
	board: &Board,
	px: i32,
	py: i32,
	kind: PieceKind,
	rot: u8,
	level: u16,
	next_kind: PieceKind,
) -> i64 {
	let max_before = *col_heights(board).iter().max().unwrap_or(&0) as i64;
	let mut b = board.clone();
	let color = kind as u8 + 1;
	b.lock_piece(px, py, kind, rot, color);
	let full = find_full_lines(&b);
	let lines = count_full_lines(&full);
	if lines > 0 {
		clear_lines(&mut b, &full);
	}

	let board_score = evaluate_board(&b, lines, level, max_before);
	let look = best_next_eval(&b, next_kind, level);

	board_score + look / W_LOOKAHEAD_DIV
}

// ---------------------------------------------------------------------------
// Candidate generation
// ---------------------------------------------------------------------------

fn relevant_rots(kind: PieceKind) -> &'static [u8] {
	match kind {
		PieceKind::O => &[0],
		PieceKind::I | PieceKind::S | PieceKind::Z => &[0, 2],
		PieceKind::T | PieceKind::L | PieceKind::J => &[0, 1, 2, 3],
	}
}

fn candidate_placements_ranked(
	board: &Board,
	kind: PieceKind,
	level: u16,
	next_kind: PieceKind,
) -> Vec<(i32, i32, u8, i64)> {
	let mut out = Vec::new();
	for &rot in relevant_rots(kind) {
		for px in -4i32..(BOARD_WIDTH as i32 + 4) {
			let Some(py) = landing_py(board, px, kind, rot) else {
				continue;
			};
			let rank = rank_placement(board, px, py, kind, rot, level, next_kind);
			out.push((px, py, rot, rank));
		}
	}
	out.sort_by(|a, b| b.3.cmp(&a.3).then(a.2.cmp(&b.2)));
	out.dedup_by(|a, b| (a.0, a.1, a.2) == (b.0, b.1, b.2));
	out
}

pub fn board_heuristic_static(board: &Board) -> i32 {
	let h = col_heights(board);
	let agg: i32 = h.iter().sum();
	let bumps: i32 = h.windows(2).map(|w| (w[0] - w[1]).abs()).sum();
	agg * 14 + bumps * 8
}

pub fn autoplay_best_placement_fallback(
	board: &Board,
	kind: PieceKind,
	level: u16,
	_combo: u32,
	next_kind: PieceKind,
) -> (i32, u8) {
	let v = candidate_placements_ranked(board, kind, level, next_kind);
	v.first()
		.map(|(px, _, rot, _)| (*px, *rot))
		.unwrap_or((3, 0))
}

// ---------------------------------------------------------------------------
// Pathfinding helpers
// ---------------------------------------------------------------------------

fn prefer_cw_rot(kind: PieceKind, from: u8, to: u8) -> bool {
	let mut cw = 0u8;
	let mut r = from;
	while r != to && cw < 6 {
		r = rotate_cw(kind, r);
		cw += 1;
	}
	let cw_ok = r == to;
	let mut ccw = 0u8;
	r = from;
	while r != to && ccw < 6 {
		r = rotate_ccw(kind, r);
		ccw += 1;
	}
	let ccw_ok = r == to;
	match (cw_ok, ccw_ok) {
		(true, false) => true,
		(false, true) => false,
		(true, true) => cw <= ccw,
		_ => true,
	}
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

fn reconstruct_path_20g(
	parent: &HashMap<SimState20g, (SimState20g, Input)>,
	start: &SimState20g,
	s: &SimState20g,
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

// ---------------------------------------------------------------------------
// Macro path (fast rotate -> shift -> drop)
// ---------------------------------------------------------------------------

fn macro_choose_input(
	_board: &Board,
	_level: u16,
	kind: PieceKind,
	s: &SimState,
	goal: (i32, i32, u8),
) -> Input {
	let (gx, _gy, gr) = goal;
	if s.rot != gr {
		let cw = prefer_cw_rot(kind, s.rot, gr);
		return Input {
			rot_cw: cw,
			rot_ccw: !cw,
			..Default::default()
		};
	}
	if s.x != gx {
		return Input {
			left: s.x > gx,
			right: s.x < gx,
			..Default::default()
		};
	}
	Input {
		down: true,
		sonic: true,
		..Default::default()
	}
}

fn try_macro_path(
	board: &Board,
	level: u16,
	kind: PieceKind,
	start: &SimState,
	goal: (i32, i32, u8),
) -> Option<Vec<Input>> {
	let (gx, gy, gr) = goal;
	let mut s = start.clone();
	let mut path = Vec::new();
	for _ in 0..400 {
		match step_falling_sim(board, level, kind, s.clone(), Input::default()) {
			StepOutcome::Locked { x, y, rot } => {
				if x == gx && y == gy && rot == gr {
					return Some(path);
				}
				return None;
			}
			StepOutcome::Falling(_) => {}
		}
		let inp = macro_choose_input(board, level, kind, &s, goal);
		path.push(inp);
		match step_falling_sim(board, level, kind, s.clone(), inp) {
			StepOutcome::Falling(ns) => s = ns,
			StepOutcome::Locked { x, y, rot } => {
				if x == gx && y == gy && rot == gr {
					return Some(path);
				}
				return None;
			}
		}
	}
	None
}

// ---------------------------------------------------------------------------
// BFS pathfinding
// ---------------------------------------------------------------------------

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

fn bfs_to_goal(
	board: &Board,
	level: u16,
	kind: PieceKind,
	start: &SimState,
	goal: (i32, i32, u8),
	budget: &mut usize,
) -> Option<Vec<Input>> {
	let mut queue = VecDeque::with_capacity(4096);
	let mut visited = HashSet::with_capacity(16_384);
	let mut parent: HashMap<SimState, (SimState, Input)> = HashMap::with_capacity(16_384);

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
			match step_falling_sim(board, level, kind, s.clone(), inp) {
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

fn bfs_to_goal_20g(
	board: &Board,
	level: u16,
	kind: PieceKind,
	start: &SimState,
	goal: (i32, i32, u8),
	budget: &mut usize,
) -> Option<Vec<Input>> {
	let start_c = to_20g_compact(start);
	let mut queue = VecDeque::with_capacity(4096);
	let mut visited = HashSet::with_capacity(16_384);
	let mut parent: HashMap<SimState20g, (SimState20g, Input)> = HashMap::with_capacity(16_384);

	queue.push_back(start_c.clone());
	visited.insert(start_c.clone());

	let mut nodes_this_goal: usize = 0;

	while let Some(s20) = queue.pop_front() {
		if *budget == 0 {
			return None;
		}
		*budget -= 1;
		nodes_this_goal += 1;
		if nodes_this_goal > MAX_NODES_PER_GOAL {
			return None;
		}

		let Some(full) = sim_state_from_20g(board, kind, &s20) else {
			continue;
		};

		for &inp in bfs_inputs() {
			match step_falling_sim(board, level, kind, full.clone(), inp) {
				StepOutcome::Falling(next) => {
					let nc = to_20g_compact(&next);
					if visited.insert(nc.clone()) {
						parent.insert(nc.clone(), (s20.clone(), inp));
						queue.push_back(nc);
					}
				}
				StepOutcome::Locked { x, y, rot } => {
					if x == goal.0 && y == goal.1 && rot == goal.2 {
						let mut path = reconstruct_path_20g(&parent, &start_c, &s20);
						path.push(inp);
						return Some(path);
					}
				}
			}
		}
	}

	None
}

fn bfs_for_level(
	board: &Board,
	level: u16,
	kind: PieceKind,
	start: &SimState,
	goal: (i32, i32, u8),
	budget: &mut usize,
) -> Option<Vec<Input>> {
	if effective_gravity(level) >= 5120 {
		bfs_to_goal_20g(board, level, kind, start, goal, budget)
	} else {
		bfs_to_goal(board, level, kind, start, goal, budget)
	}
}

/// At 20G, explore ALL reachable positions from the current state and return
/// the path to the best landing (by `rank_placement`).  This avoids wasting
/// budget on unreachable goal positions — every BFS node contributes to
/// finding good reachable landings.
fn bfs_best_reachable_20g(
	board: &Board,
	level: u16,
	kind: PieceKind,
	start: &SimState,
	max_nodes: usize,
	next_kind: PieceKind,
) -> Option<Vec<Input>> {
	let start_c = to_20g_compact(start);
	let mut queue = VecDeque::with_capacity(4096);
	let mut visited = HashSet::with_capacity(16_384);
	let mut parent: HashMap<SimState20g, (SimState20g, Input)> = HashMap::with_capacity(16_384);

	queue.push_back(start_c.clone());
	visited.insert(start_c.clone());

	let mut best_path: Option<Vec<Input>> = None;
	let mut best_score = i64::MIN;
	let mut evaluated: HashSet<(i32, i32, u8)> = HashSet::new();

	let mut nodes: usize = 0;
	while let Some(s20) = queue.pop_front() {
		if nodes >= max_nodes {
			break;
		}
		nodes += 1;

		let Some(full) = sim_state_from_20g(board, kind, &s20) else {
			continue;
		};

		for &inp in bfs_inputs() {
			match step_falling_sim(board, level, kind, full.clone(), inp) {
				StepOutcome::Falling(next) => {
					let nc = to_20g_compact(&next);
					if visited.insert(nc.clone()) {
						parent.insert(nc.clone(), (s20.clone(), inp));
						queue.push_back(nc);
					}
				}
				StepOutcome::Locked { x, y, rot } => {
					if evaluated.insert((x, y, rot)) {
						let score = rank_placement(board, x, y, kind, rot, level, next_kind);
						if score > best_score {
							best_score = score;
							let mut path = reconstruct_path_20g(&parent, &start_c, &s20);
							path.push(inp);
							best_path = Some(path);
						}
					}
				}
			}
		}
	}

	best_path
}

// ---------------------------------------------------------------------------
// Public planning API
// ---------------------------------------------------------------------------

pub fn autoplay_plan_inputs(g: &Game) -> Option<Vec<Input>> {
	if g.game_over || g.phase != Phase::Falling {
		return None;
	}
	let p = g.piece?;

	let mut start = SimState {
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

	if effective_gravity(g.level) >= 5120 {
		if let Some(y) = landing_py(board, start.x, kind, start.rot) {
			start.y = y;
			start.gravity_accum = 0;
		}
	}

	let candidates = candidate_placements_ranked(board, kind, g.level, g.next_kind);
	let mut budget = MAX_BFS_NODES_TOTAL;
	for (i, (gx, gy, gr, _)) in candidates.iter().enumerate() {
		if i >= MAX_CANDIDATES_PATHFIND {
			break;
		}
		if budget == 0 {
			break;
		}
		let goal = (*gx, *gy, *gr);
		if let Some(path) = try_macro_path(board, g.level, kind, &start, goal) {
			return Some(path);
		}
		if let Some(path) = bfs_for_level(board, g.level, kind, &start, goal, &mut budget) {
			return Some(path);
		}
	}

	// At 20G, replace the greedy fallback with an exhaustive reachability
	// search that finds the best position we can actually get to.
	if effective_gravity(g.level) >= 5120 {
		return bfs_best_reachable_20g(board, g.level, kind, &start, 50_000, g.next_kind);
	}

	None
}

// ---------------------------------------------------------------------------
// IRS (Initial Rotation System) helpers
// ---------------------------------------------------------------------------

/// Quick evaluation (no lookahead) to find the best rotation for IRS.
fn quick_best_rot(board: &Board, kind: PieceKind, level: u16) -> u8 {
	let max_before = *col_heights(board).iter().max().unwrap_or(&0) as i64;
	let mut best_rot = 0u8;
	let mut best_score = i64::MIN;
	for &rot in relevant_rots(kind) {
		for px in -2..(BOARD_WIDTH as i32 + 2) {
			let Some(py) = landing_py(board, px, kind, rot) else {
				continue;
			};
			let mut b = board.clone();
			b.lock_piece(px, py, kind, rot, 1);
			let full = find_full_lines(&b);
			let n = count_full_lines(&full);
			if n > 0 {
				clear_lines(&mut b, &full);
			}
			let eval = evaluate_board(&b, n, level, max_before);
			if eval > best_score {
				best_score = eval;
				best_rot = rot;
			}
		}
	}
	best_rot
}

/// Partial DAS charge (1..DAS_FRAMES-1) between locks **hurts** first-move timing: the
/// spawn frame does not advance DAS, so the first falling frame can miss `das_left == 1`.
/// Only pre-hold when delays are long enough to reach full DAS (or ARE alone suffices).
fn allow_das_prehold(g: &Game) -> bool {
	match g.phase {
		Phase::LineClear => {
			g.options.line_clear_frames() + g.options.are_frames() >= DAS_FRAMES
		}
		Phase::Are => g.options.are_frames() >= DAS_FRAMES,
		_ => false,
	}
}

/// Hold left/right during line clear / ARE so DAS is charged when the piece spawns.
/// Uses the same rotation heuristic as IRS (`quick_best_rot`) and the best ranked
/// placement with that rotation (fallback: overall best) to pick a side.
fn merge_prehold_from_irs_rot(
	inp: &mut Input,
	board: &Board,
	kind: PieceKind,
	level: u16,
	irs_rot: u8,
) {
	let candidates = candidate_placements_ranked(board, kind, level, LOOKAHEAD_PLACEHOLDER);
	let Some((tx, _, _, _)) = candidates.iter().find(|(_, _, r, _)| *r == irs_rot) else {
		return;
	};
	// Spawn column must match IRS rotation (`quick_best_rot`), not another candidate rot.
	let sx = spawn_origin(kind, irs_rot).0;
	if *tx < sx {
		inp.left = true;
	} else if *tx > sx {
		inp.right = true;
	}
}

fn prehold_input_for_next(board: &Board, kind: PieceKind, level: u16) -> Input {
	let irs_rot = quick_best_rot(board, kind, level);
	let mut inp = Input::default();
	merge_prehold_from_irs_rot(&mut inp, board, kind, level, irs_rot);
	inp
}

/// Map a target rotation to the IRS input that gets closest to it at spawn.
fn irs_input_for(kind: PieceKind, target_rot: u8) -> Input {
	if target_rot == 0 {
		return Input::default();
	}
	match kind {
		PieceKind::O => Input::default(),
		PieceKind::I | PieceKind::S | PieceKind::Z => {
			if target_rot == 2 {
				Input {
					rot_cw: true,
					..Default::default()
				}
			} else {
				Input::default()
			}
		}
		_ => match target_rot {
			1 | 2 => Input {
				rot_cw: true,
				..Default::default()
			},
			3 => Input {
				rot_ccw: true,
				..Default::default()
			},
			_ => Input::default(),
		},
	}
}

// ---------------------------------------------------------------------------
// AutoplayDriver
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct AutoplayDriver {
	queue: VecDeque<Input>,
	greedy_had_piece: bool,
	greedy_target_x: i32,
	greedy_target_rot: u8,
}

impl AutoplayDriver {
	pub fn reset(&mut self) {
		self.queue.clear();
		self.greedy_had_piece = false;
	}

	pub fn pick_input(&mut self, g: &Game) -> Input {
		if g.phase != Phase::Falling {
			if g.piece.is_none() {
				self.queue.clear();
				self.greedy_had_piece = false;
			}
			// IRS + DAS pre-hold: on the spawn frame, rotate before the piece appears;
			// during line clear / ARE, hold toward the planned column so DAS charges.
			if g.phase == Phase::Are && g.are_timer == 0 {
				let target_rot = quick_best_rot(&g.board, g.next_kind, g.level);
				let mut inp = irs_input_for(g.next_kind, target_rot);
				if allow_das_prehold(g) {
					merge_prehold_from_irs_rot(
						&mut inp,
						&g.board,
						g.next_kind,
						g.level,
						target_rot,
					);
				}
				return inp;
			}
			if (g.phase == Phase::LineClear || g.phase == Phase::Are) && allow_das_prehold(g) {
				return prehold_input_for_next(&g.board, g.next_kind, g.level);
			}
			return Input::default();
		}

		if g.piece.is_none() {
			self.queue.clear();
			return Input::default();
		}

		if self.queue.is_empty() {
			if let Some(path) = autoplay_plan_inputs(g) {
				self.queue.extend(path);
				self.greedy_had_piece = false;
			} else {
				return self.greedy_pick_input(g);
			}
		}

		self.queue
			.pop_front()
			.unwrap_or_else(|| self.greedy_pick_input(g))
	}

	fn greedy_pick_input(&mut self, g: &Game) -> Input {
		let Some(p) = g.piece else {
			self.greedy_had_piece = false;
			return Input::default();
		};

		let new_piece = !self.greedy_had_piece;
		self.greedy_had_piece = true;
		if new_piece {
			let (tx, tr) = autoplay_best_placement_fallback(
				&g.board,
				p.kind,
				g.level,
				g.combo,
				g.next_kind,
			);
			self.greedy_target_x = tx;
			self.greedy_target_rot = tr;
		}

		if p.rot != self.greedy_target_rot {
			let use_cw = prefer_cw_rot(p.kind, p.rot, self.greedy_target_rot);
			return Input {
				rot_cw: use_cw,
				rot_ccw: !use_cw,
				..Default::default()
			};
		}

		if p.x < self.greedy_target_x {
			return Input {
				right: true,
				..Default::default()
			};
		}
		if p.x > self.greedy_target_x {
			return Input {
				left: true,
				..Default::default()
			};
		}

		Input {
			down: true,
			sonic: true,
			..Default::default()
		}
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::board::Board;
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
		let out = step_falling_sim(&board, level, kind, s0, inp);
		g.step(inp);
		match out {
			StepOutcome::Falling(ns) => {
				if g.game_over || g.phase != Phase::Falling {
					return false;
				}
				let p2 = g.piece.expect("piece");
				p2.x == ns.x
					&& p2.y == ns.y && p2.rot == ns.rot
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
			match step_falling_sim(&board, level, kind, s.clone(), Input::default()) {
				StepOutcome::Falling(ns) => s = ns,
				StepOutcome::Locked { x, y, rot } => break (x, y, rot),
			}
		};
		let cands = candidate_placements_ranked(&board, kind, level, g.next_kind);
		assert!(
			cands
				.iter()
				.any(|(cx, cy, cr, _)| *cx == lx && *cy == ly && *cr == lr),
			"natural lock ({},{},{}) not in candidates: first few {:?}",
			lx,
			ly,
			lr,
			&cands[..cands.len().min(5)]
		);
	}

	#[test]
	fn autoplay_finds_plan_at_20g() {
		let mut g = game_with_falling_piece(123);
		g.level = 500;
		let plan = autoplay_plan_inputs(&g).expect("plan at 20g");
		assert!(!plan.is_empty());
	}

	#[test]
	fn no_holes_on_empty_board() {
		let b = Board::new();
		let h = col_heights(&b);
		assert_eq!(count_holes(&b, &h), 0);
		assert_eq!(hole_depth_sum(&b, &h), 0);
	}

	#[test]
	fn holes_detected_at_column_bottom() {
		let mut b = Board::new();
		b.rows[2][3] = 1;
		let h = col_heights(&b);
		assert_eq!(h[3], 3);
		assert_eq!(count_holes(&b, &h), 2);
	}

	#[test]
	fn hole_depth_grows_with_burial() {
		let mut shallow = Board::new();
		shallow.rows[0][0] = 1;
		shallow.rows[2][0] = 1;
		let hs = col_heights(&shallow);
		let ds = hole_depth_sum(&shallow, &hs);

		let mut deep = Board::new();
		deep.rows[0][0] = 1;
		deep.rows[2][0] = 1;
		deep.rows[3][0] = 1;
		deep.rows[4][0] = 1;
		let hd = col_heights(&deep);
		let dd = hole_depth_sum(&deep, &hd);

		assert!(dd > ds, "deeper burial {dd} > shallow {ds}");
	}
}
