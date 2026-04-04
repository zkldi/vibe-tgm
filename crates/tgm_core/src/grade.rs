//! Grade from score; GM time gates (60 Hz frames).

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
	Nine,
	Eight,
	Seven,
	Six,
	Five,
	Four,
	Three,
	Two,
	One,
	S1,
	S2,
	S3,
	S4,
	S5,
	S6,
	S7,
	S8,
	S9,
	Gm,
}

impl Grade {
	pub fn from_score(score: u64) -> Self {
		match score {
			s if s >= 120_000 => Grade::S9,
			s if s >= 100_000 => Grade::S8,
			s if s >= 82_000 => Grade::S7,
			s if s >= 66_000 => Grade::S6,
			s if s >= 52_000 => Grade::S5,
			s if s >= 40_000 => Grade::S4,
			s if s >= 30_000 => Grade::S3,
			s if s >= 22_000 => Grade::S2,
			s if s >= 16_000 => Grade::S1,
			s if s >= 12_000 => Grade::One,
			s if s >= 8_000 => Grade::Two,
			s if s >= 5_500 => Grade::Three,
			s if s >= 3_500 => Grade::Four,
			s if s >= 2_000 => Grade::Five,
			s if s >= 1_400 => Grade::Six,
			s if s >= 800 => Grade::Seven,
			s if s >= 400 => Grade::Eight,
			_ => Grade::Nine,
		}
	}

	pub fn display(self) -> &'static str {
		match self {
			Grade::Nine => "9",
			Grade::Eight => "8",
			Grade::Seven => "7",
			Grade::Six => "6",
			Grade::Five => "5",
			Grade::Four => "4",
			Grade::Three => "3",
			Grade::Two => "2",
			Grade::One => "1",
			Grade::S1 => "S1",
			Grade::S2 => "S2",
			Grade::S3 => "S3",
			Grade::S4 => "S4",
			Grade::S5 => "S5",
			Grade::S6 => "S6",
			Grade::S7 => "S7",
			Grade::S8 => "S8",
			Grade::S9 => "S9",
			Grade::Gm => "GM",
		}
	}
}
