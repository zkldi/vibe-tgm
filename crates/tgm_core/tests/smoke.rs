use tgm_core::{internal_gravity, level_after_line_clear, line_clear_only_for_increment, Game, Input};

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
