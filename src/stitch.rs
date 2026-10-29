use anyhow::{Context, Result};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Clone)]
pub struct StitchConfig {
    pub crossfade_ms: u32,
    pub sample_rate: u32,
}

impl Default for StitchConfig {
    fn default() -> Self {
        Self {
            crossfade_ms: 120,
            sample_rate: 22050,
        }
    }
}

pub fn stitch_segments(
    paths: &[(PathBuf, String)],
    cfg: &StitchConfig,
) -> Result<Vec<u8>> {
    let mut all_samples: Vec<f32> = Vec::new();
    let mut target_rate = cfg.sample_rate;

    for (i, (path, _)) in paths.iter().enumerate() {
        let mut reader = WavReader::open(path)
            .with_context(|| format!("open {:?}", path))?;
        let spec = reader.spec();
        target_rate = spec.sample_rate;

        let samples: Result<Vec<f32>, _> = reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect();
        let samples = samples?;

        if i == 0 {
            all_samples.extend_from_slice(&samples);
        } else {
            let fade = ((cfg.crossfade_ms as f32 / 1000.0) * target_rate as f32) as usize;
            let fade = fade.min(all_samples.len()).min(samples.len());

            for j in 0..fade {
                let t = j as f32 / fade as f32;
                let idx = all_samples.len() - fade + j;
                all_samples[idx] = all_samples[idx] * (1.0 - t) + samples[j] * t;
            }
            all_samples.extend_from_slice(&samples[fade..]);
        }
    }

    // Peak normalize
    let peak = all_samples
        .iter()
        .map(|x| x.abs())
        .fold(0.0f32, f32::max);
    if peak > 0.0 {
        let gain = 0.9 / peak;
        for s in &mut all_samples {
            *s *= gain;
        }
    }

    let mut cursor = Cursor::new(Vec::new());
    {
        let spec = WavSpec {
            channels: 1,
            sample_rate: target_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::new(&mut cursor, spec)?;
        for s in all_samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(v)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}
