//! TGM1-style client: WASD + J/K/L (L = CCW duplicate), F3 debug, P pause.

use macroquad::prelude::*;

use tgm_core::{
    piece_cells, Game, Input, Phase, PieceKind, TLS_MAX_LEVEL, VISIBLE_ROWS, BOARD_HEIGHT,
    BOARD_WIDTH,
};

const CELL: f32 = 22.0;
const MARGIN: f32 = 24.0;
const HUD_W: f32 = 200.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "TGM1 (Rust)".to_string(),
        window_width: (BOARD_WIDTH as f32 * CELL + MARGIN * 3.0 + HUD_W) as i32,
        window_height: (VISIBLE_ROWS as f32 * CELL + MARGIN * 2.0 + 40.0) as i32,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let seed = 0xC0FFEE_u64;
    let mut game = Game::new(seed);
    let mut debug_overlay = false;
    let mut paused = false;

    loop {
        if is_key_pressed(KeyCode::F3) {
            debug_overlay = !debug_overlay;
        }
        if is_key_pressed(KeyCode::P) {
            paused = !paused;
        }

        let mut step_once = !paused;
        if paused && is_key_pressed(KeyCode::Period) {
            step_once = true;
        }

        if step_once {
            game.step(poll_input());
        }

        clear_background(Color::from_rgba(12, 14, 22, 255));

        draw_field(&game);
        draw_hud(&game);
        if debug_overlay {
            draw_debug(&game);
        }

        if game.game_over {
            draw_text(
                "GAME OVER",
                MARGIN,
                screen_height() * 0.45,
                36.0,
                RED,
            );
        } else if game.cleared {
            draw_text(
                "LEVEL 999",
                MARGIN,
                screen_height() * 0.42,
                36.0,
                GOLD,
            );
            let g = game.grade_label();
            draw_text(
                &format!("Grade: {g}"),
                MARGIN,
                screen_height() * 0.48,
                28.0,
                WHITE,
            );
        }

        if paused {
            draw_text(
                "PAUSED (P)  STEP (.)",
                MARGIN,
                18.0,
                18.0,
                Color::from_rgba(200, 200, 100, 255),
            );
        }

        next_frame().await;
    }
}

fn poll_input() -> Input {
    Input {
        left: is_key_down(KeyCode::A),
        right: is_key_down(KeyCode::D),
        down: is_key_down(KeyCode::S),
        sonic: is_key_down(KeyCode::W),
        rot_cw: is_key_down(KeyCode::K),
        rot_ccw: is_key_down(KeyCode::J) || is_key_down(KeyCode::L),
    }
}

fn cell_color(c: u8) -> Color {
    match c {
        1 => Color::from_rgba(0, 240, 240, 255),   // I
        2 => Color::from_rgba(200, 0, 240, 255),     // T
        3 => Color::from_rgba(240, 160, 0, 255),     // L
        4 => Color::from_rgba(0, 0, 240, 255),       // J
        5 => Color::from_rgba(0, 240, 0, 255),      // S
        6 => Color::from_rgba(240, 0, 0, 255),      // Z
        7 => Color::from_rgba(240, 240, 0, 255),     // O
        _ => Color::from_rgba(30, 30, 40, 255),
    }
}

fn board_screen_y(row: i32) -> f32 {
    MARGIN + (VISIBLE_ROWS as f32 - 1.0 - row as f32) * CELL
}

fn draw_field(game: &Game) {
    let ox = MARGIN;
    for y in 0..VISIBLE_ROWS as i32 {
        for x in 0..BOARD_WIDTH as i32 {
            let c = game.board.rows[y as usize][x as usize];
            let px = ox + x as f32 * CELL;
            let py = board_screen_y(y);
            draw_rectangle(px, py, CELL - 1.0, CELL - 1.0, cell_color(c));
        }
    }

    // Ghost (TLS levels 0..=100)
    if game.level <= TLS_MAX_LEVEL {
        if let Some(p) = game.piece {
            let gy = game.board.drop_to_bottom(p.x, p.y, p.kind, p.rot);
            let def = piece_cells(p.kind, p.rot);
            for (dx, dy) in def.cells {
                let bx = p.x + dx as i32;
                let by = gy + dy as i32;
                if by >= 0 && (by as usize) < VISIBLE_ROWS {
                    let px = ox + bx as f32 * CELL;
                    let py = board_screen_y(by);
                    draw_rectangle_lines(px, py, CELL - 1.0, CELL - 1.0, 2.0, Color::from_rgba(255, 255, 255, 80));
                }
            }
        }
    }

    // Active piece
    if let Some(p) = game.piece {
        let def = piece_cells(p.kind, p.rot);
        let col = cell_color(p.kind as u8 + 1);
        for (dx, dy) in def.cells {
            let bx = p.x + dx as i32;
            let by = p.y + dy as i32;
            if by >= 0 && (by as usize) < BOARD_HEIGHT {
                let px = ox + bx as f32 * CELL;
                let py = board_screen_y(by);
                draw_rectangle(px, py, CELL - 1.0, CELL - 1.0, col);
            }
        }
    }

    // Border
    let w = BOARD_WIDTH as f32 * CELL;
    let h = VISIBLE_ROWS as f32 * CELL;
    draw_rectangle_lines(ox - 2.0, MARGIN - 2.0, w + 4.0, h + 4.0, 3.0, WHITE);
}

fn draw_hud(game: &Game) {
    let hx = MARGIN + BOARD_WIDTH as f32 * CELL + MARGIN;
    let mut y = MARGIN;
    let line = 22.0;
    draw_text(&format!("Level {}", game.level), hx, y, 24.0, WHITE);
    y += line + 8.0;
    draw_text(&format!("Score {}", game.score), hx, y, 20.0, WHITE);
    y += line;
    let sec = game.frame as f32 / 60.0;
    draw_text(&format!("Time {:.2}", sec), hx, y, 18.0, Color::from_rgba(180, 180, 200, 255));
    y += line;
    draw_text(&format!("Grade {}", game.grade_label()), hx, y, 20.0, GOLD);
    y += line + 12.0;
    draw_text("NEXT", hx, y, 18.0, GRAY);
    y += line;
    draw_mini_piece(hx, y, game.next_kind);
    y += 5.0 * 14.0 + 20.0;
    draw_text("W sonic  S soft", hx, y, 14.0, GRAY);
    y += 16.0;
    draw_text("A/D move  J/K/L rot", hx, y, 14.0, GRAY);
    y += 16.0;
    draw_text("P pause  . step", hx, y, 14.0, GRAY);
}

fn draw_mini_piece(x: f32, y: f32, kind: PieceKind) {
    let def = piece_cells(kind, 0);
    let s = 12.0;
    let col = cell_color(kind as u8 + 1);
    for (dx, dy) in def.cells {
        let px = x + dx as f32 * s;
        let py = y + (3.0 - dy as f32) * s;
        draw_rectangle(px, py, s - 1.0, s - 1.0, col);
    }
}

fn draw_debug(game: &Game) {
    let hx = MARGIN + BOARD_WIDTH as f32 * CELL + MARGIN;
    let y = screen_height() - 160.0;
    let phase = match game.phase {
        Phase::Falling => "Falling",
        Phase::LineClear => "LineClear",
        Phase::Are => "ARE",
    };
    draw_text(
        &format!(
            "frame {}  phase {}\nlock {}  das L{} R{}\naccum {}",
            game.frame,
            phase,
            game.lock_delay,
            game.das_left,
            game.das_right,
            game.gravity_accum
        ),
        hx,
        y,
        14.0,
        Color::from_rgba(150, 255, 150, 255),
    );
}
