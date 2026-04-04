//! Internal gravity in units of (1/256) cells per frame. G = internal / 256.

/// Returns internal gravity for the given level (piecewise constant from wiki table).
pub fn internal_gravity(level: u16) -> u16 {
	match level {
		0..=29 => 4,
		30..=34 => 6,
		35..=39 => 8,
		40..=49 => 10,
		50..=59 => 12,
		60..=69 => 16,
		70..=79 => 32,
		80..=89 => 48,
		90..=99 => 64,
		100..=119 => 80,
		120..=139 => 96,
		140..=159 => 112,
		160..=169 => 128,
		170..=199 => 144,
		200..=219 => 4,
		220..=229 => 32,
		230..=239 => 64,
		240..=249 => 96,
		250..=259 => 128,
		260..=269 => 160,
		270..=279 => 192,
		280..=289 => 224,
		290..=299 => 256,
		300..=329 => 512,
		330..=359 => 768,
		360..=399 => 1024,
		400..=419 => 1280,
		420..=449 => 1024,
		450..=499 => 768,
		500..=u16::MAX => 5120,
	}
}

/// Alias for [`internal_gravity`]: gravity used during play (no alternate modes).
pub fn effective_gravity(level: u16) -> u16 {
	internal_gravity(level)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sample_levels() {
		assert_eq!(internal_gravity(0), 4);
		assert_eq!(internal_gravity(30), 6);
		assert_eq!(internal_gravity(90), 64);
		assert_eq!(internal_gravity(200), 4);
		assert_eq!(internal_gravity(220), 32);
		assert_eq!(internal_gravity(500), 5120);
		assert_eq!(internal_gravity(999), 5120);
	}
}
