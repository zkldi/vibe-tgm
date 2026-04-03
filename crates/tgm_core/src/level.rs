//! Level increment rules for TGM1.

/// Returns true if only line clears may advance level (…99→…00, or 998→999).
pub fn line_clear_only_for_increment(level: u16) -> bool {
    if level == 998 {
        return true;
    }
    // Levels 99, 199, … 899: next increment crosses a hundreds boundary; only line clears count.
    level < 999 && level % 100 == 99
}

/// Apply level increment for a new piece entering the field (not a line clear).
pub fn level_after_piece_spawn(level: u16) -> Option<u16> {
    if level >= 999 {
        return None;
    }
    if line_clear_only_for_increment(level) {
        return Some(level);
    }
    Some(level + 1)
}

/// Apply level increment from line clears (lines cleared this lock).
pub fn level_after_line_clear(level: u16, lines: u32) -> Option<u16> {
    let mut lv = level;
    for _ in 0..lines {
        if lv >= 999 {
            break;
        }
        lv += 1;
    }
    Some(lv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hundreds_gate() {
        assert!(line_clear_only_for_increment(299));
        assert!(!line_clear_only_for_increment(298));
        assert_eq!(level_after_piece_spawn(299), Some(299));
        assert!(line_clear_only_for_increment(99));
    }

    #[test]
    fn nine_nine_eight() {
        assert!(line_clear_only_for_increment(998));
        assert_eq!(level_after_line_clear(998, 1), Some(999));
    }
}
