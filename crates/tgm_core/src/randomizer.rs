//! TGM1 randomizer: history of 4, initial ZZZZ, 4 tries, first piece never S/Z/O.

use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;

use crate::piece::PieceKind;

#[derive(Clone, Debug)]
pub struct TgmRandomizer {
    rng: SmallRng,
    history: [u8; 4],
    first_piece: bool,
}

impl TgmRandomizer {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            history: [PieceKind::Z as u8; 4],
            first_piece: true,
        }
    }

    pub fn next_piece(&mut self) -> PieceKind {
        let mut p = self.gen_piece();
        if self.first_piece {
            while matches!(p, PieceKind::S | PieceKind::Z | PieceKind::O) {
                p = self.gen_piece();
            }
            self.first_piece = false;
        }
        self.push_history(p);
        p
    }

    fn gen_piece(&mut self) -> PieceKind {
        for _ in 0..4 {
            let v = self.rng.gen_range(0u8..7);
            if !self.history.contains(&v) {
                return PieceKind::from_u8(v).unwrap();
            }
        }
        let v = self.rng.gen_range(0u8..7);
        PieceKind::from_u8(v).unwrap()
    }

    fn push_history(&mut self, p: PieceKind) {
        self.history[0] = self.history[1];
        self.history[1] = self.history[2];
        self.history[2] = self.history[3];
        self.history[3] = p as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_not_sz_o() {
        for seed in 0u64..500 {
            let mut r = TgmRandomizer::new(seed);
            let p = r.next_piece();
            assert!(
                !matches!(p, PieceKind::S | PieceKind::Z | PieceKind::O),
                "seed {seed}"
            );
        }
    }
}
