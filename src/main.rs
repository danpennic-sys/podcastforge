use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

mod dialogue;
mod evidence;
mod stitch;

use dialogue::{compile_dialogue, CompileOpts};
use evidence::EvidencePack;
use stitch::{stitch_segments, StitchConfig};

#[derive(Parser)]
#[command(name = "podcastforge", version, about = "Minimal offline 2-person podcast generator with Maxx guest")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a podcast from text or file
    Generate {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(long)]
        text: Option<String>,
        #[arg(short, long, default_value = "output/podcast.wav")]
        output: PathBuf,
        #[arg(long, default_value = "models")]
        models_dir: PathBuf,
        #[arg(long, default_value = "en_US-amy-medium")]
        voice_a: String,
        #[arg(long, default_value = "en_US-ryan-medium")]
        voice_b: String,
        #[arg(long, default_value = "en_US-lessac-medium")]
        voice_guest: String,
        #[arg(long, default_value = "Alex")]
        host_a: String,
        #[arg(long, default_value = "Sam")]
        host_b: String,
        #[arg(long, default_value = "Maxx")]
        host_guest: String,
        /// Explicit seed for reproducibility. Omit for random.
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, default_value = "8")]
        target_minutes: f32,
    },
    /// Show version + build info
    Info,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate {
            input,
            text,
            output,
            models_dir,
            voice_a,
            voice_b,
            voice_guest,
            host_a,
            host_b,
            host_guest,
            seed,
            target_minutes,
        } => {
            let source = if let Some(p) = input {
                fs::read_to_string(&p).with_context(|| format!("read {:?}", p))?
            } else if let Some(t) = text {
                t
            } else {
                bail!("provide --input <file> or --text \"...\"");
            };

            run_generate(
                &source,
                &output,
                &models_dir,
                &voice_a,
                &voice_b,
                &voice_guest,
                &host_a,
                &host_b,
                &host_guest,
                seed,
                target_minutes,
            )?;
        }
        Commands::Info => {
            println!("podcastforge 0.1.0 — minimal deterministic core + Maxx guest");
            println!("offline, seeded variance, evidence pack emitted");
        }
    }
    Ok(())
}

fn run_generate(
    source: &str,
    output: &Path,
    models_dir: &Path,
    voice_a: &str,
    voice_b: &str,
    voice_guest: &str,
    host_a: &str,
    host_b: &str,
    host_guest: &str,
    seed_opt: Option<u64>,
    target_minutes: f32,
) -> Result<()> {
    let seed = seed_opt.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    });
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    println!("seed: {}", seed);
    println!("analyzing + compiling dialogue...");

    let opts = CompileOpts {
        host_a: host_a.to_string(),
        host_b: host_b.to_string(),
        host_guest: host_guest.to_string(),
        voice_a: voice_a.to_string(),
        voice_b: voice_b.to_string(),
        voice_guest: voice_guest.to_string(),
        target_minutes,
    };
    let script = compile_dialogue(source, &opts, &mut rng)?;

    println!("segments: {}", script.turns.len());
    println!("synthesizing with local piper...");

    let piper = which::which("piper").or_else(|_| which::which("piper-tts"))
        .context("piper binary not found on PATH. Install from https://github.com/rhasspy/piper and place models in models/")?;

    let work = PathBuf::from("output/work");
    fs::create_dir_all(&work)?;
    let mut wav_paths = Vec::new();

    for (i, turn) in script.turns.iter().enumerate() {
        let model = models_dir.join(format!("{}.onnx", turn.voice));
        if !model.exists() {
            bail!("model missing: {:?}", model);
        }
        let out_wav = work.join(format!("{:03}_{}.wav", i, turn.role));
        let status = Command::new(&piper)
            .arg("--model")
            .arg(&model)
            .arg("--output_file")
            .arg(&out_wav)
            .arg("--")
            .arg(&turn.text)
            .status()
            .context("piper failed")?;
        if !status.success() {
            bail!("piper exit {:?}", status.code());
        }
        wav_paths.push((out_wav, turn.speaker.clone()));
        print!(".");
        std::io::Write::flush(&mut std::io::stdout())?;
    }
    println!();

    println!("stitching...");
    let cfg = StitchConfig::default();
    let final_wav = stitch_segments(&wav_paths, &cfg)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, &final_wav)?;
    println!("wrote {:?}", output);

    // Evidence pack
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(seed.to_le_bytes());
    hasher.update(serde_json::to_vec(&script)?);
    let content_hash = format!("{:x}", hasher.finalize());

    let pack = EvidencePack {
        version: "0.1.0".into(),
        seed,
        content_hash,
        generated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        voice_a: voice_a.into(),
        voice_b: voice_b.into(),
        voice_guest: voice_guest.into(),
        host_a: host_a.into(),
        host_b: host_b.into(),
        host_guest: host_guest.into(),
        turn_count: script.turns.len(),
        output_path: output.display().to_string(),
        notes: "Deterministic under same seed + models. Piper external binary. Maxx guest included. No network.".into(),
    };
    let evidence_path = output.with_extension("evidence.json");
    fs::write(&evidence_path, serde_json::to_string_pretty(&pack)?)?;
    println!("evidence: {:?}", evidence_path);

    Ok(())
}

// Minimal which implementation to avoid extra dep
mod which {
    use std::path::PathBuf;
    pub fn which(bin: &str) -> anyhow::Result<PathBuf> {
        if let Ok(p) = std::env::var("PATH") {
            for dir in std::env::split_paths(&p) {
                let candidate = dir.join(bin);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
        anyhow::bail!("{} not found", bin)
    }
}
