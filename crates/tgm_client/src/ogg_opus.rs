//! Decode Ogg Opus (RFC 7845) to 48 kHz 16-bit WAV bytes for macroquad/quad-snd.

use std::io::Cursor;

use audiopus::coder::Decoder;
use audiopus::{Channels, SampleRate};
use hound::{SampleFormat, WavSpec, WavWriter};
use ogg::PacketReader;

const OPUS_HEAD: &[u8; 8] = b"OpusHead";
const OPUS_TAGS: &[u8; 8] = b"OpusTags";

/// Max samples per channel per Opus packet at 48 kHz (120 ms).
const MAX_FRAME_SAMPLES_PER_CHANNEL: usize = 5760;

pub fn decode_ogg_opus_to_wav_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
	let mut reader = PacketReader::new(Cursor::new(data));
	let head = reader
		.read_packet()
		.map_err(|e| e.to_string())?
		.ok_or_else(|| "empty ogg stream".to_string())?;
	if head.data.len() < 19 || !head.data.starts_with(OPUS_HEAD) {
		return Err("missing OpusHead".to_string());
	}
	if head.data[8] != 1 {
		return Err(format!("unsupported OpusHead version {}", head.data[8]));
	}
	let channels = head.data[9];
	let pre_skip = u16::from_le_bytes([head.data[10], head.data[11]]);
	let mapping_family = head.data.get(18).copied().unwrap_or(0);
	if mapping_family != 0 {
		return Err(format!(
			"unsupported Opus channel mapping family {mapping_family}"
		));
	}
	let ch = match channels {
		1 => Channels::Mono,
		2 => Channels::Stereo,
		n => return Err(format!("unsupported channel count {n}")),
	};

	let tags = reader
		.read_packet()
		.map_err(|e| e.to_string())?
		.ok_or_else(|| "missing OpusTags".to_string())?;
	if !tags.data.starts_with(OPUS_TAGS) {
		return Err("second packet is not OpusTags".to_string());
	}

	let mut decoder = Decoder::new(SampleRate::Hz48000, ch).map_err(|e| e.to_string())?;
	let mut pcm: Vec<i16> = Vec::new();
	let mut scratch = vec![0i16; MAX_FRAME_SAMPLES_PER_CHANNEL * channels as usize];
	// Interleaved i16 samples still to discard (RFC 7845 pre-skip, per channel).
	let mut pending_skip = pre_skip as usize * channels as usize;

	while let Some(packet) = reader.read_packet().map_err(|e| e.to_string())? {
		if packet.data.is_empty() {
			continue;
		}
		if packet.data.starts_with(OPUS_HEAD) || packet.data.starts_with(OPUS_TAGS) {
			continue;
		}

		let n_per_ch = decoder
			.decode(Some(&packet.data[..]), &mut scratch[..], false)
			.map_err(|e| e.to_string())?;
		let n_interleaved = n_per_ch * channels as usize;
		let decoded = &scratch[..n_interleaved];

		if pending_skip > 0 {
			if pending_skip >= decoded.len() {
				pending_skip -= decoded.len();
			} else {
				pcm.extend_from_slice(&decoded[pending_skip..]);
				pending_skip = 0;
			}
		} else {
			pcm.extend_from_slice(decoded);
		}
	}

	let spec = WavSpec {
		channels: channels as u16,
		sample_rate: 48_000,
		bits_per_sample: 16,
		sample_format: SampleFormat::Int,
	};
	let mut out = Vec::new();
	{
		let mut w = WavWriter::new(Cursor::new(&mut out), spec).map_err(|e| e.to_string())?;
		for s in pcm {
			w.write_sample(s).map_err(|e| e.to_string())?;
		}
		w.finalize().map_err(|e| e.to_string())?;
	}
	Ok(out)
}

#[cfg(test)]
mod tests {
	#[test]
	fn decode_bgm_intro_smoke() {
		let bytes = include_bytes!("../assets/audio/bgm1_intro.opus");
		let wav = super::decode_ogg_opus_to_wav_bytes(bytes).expect("decode");
		assert!(wav.len() > 1000);
		assert_eq!(&wav[0..4], b"RIFF");
	}
}
