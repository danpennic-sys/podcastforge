use anyhow::Result;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOpts {
    pub host_a: String,
    pub host_b: String,
    pub host_guest: String,
    pub voice_a: String,
    pub voice_b: String,
    pub voice_guest: String,
    pub target_minutes: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub speaker: String,
    pub voice: String,
    pub text: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub turns: Vec<Turn>,
    pub seed_used: u64,
}

pub fn compile_dialogue(
    source: &str,
    opts: &CompileOpts,
    rng: &mut ChaCha8Rng,
) -> Result<Script> {
    let paragraphs: Vec<&str> = source
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| p.len() > 40)
        .take(8)
        .collect();

    if paragraphs.is_empty() {
        anyhow::bail!("input too short or empty after segmentation");
    }

    let mut turns = Vec::new();

    // Opener (always Host A)
    let opener_style = rng.gen_range(0..3);
    let opener = match opener_style {
        0 => format!(
            "Hey, I'm {}, and with me is {}. Today we're looking at some material that just came across our desk.",
            opts.host_a, opts.host_b
        ),
        1 => format!(
            "Welcome. {} here with {}. Let's dig into this.",
            opts.host_a, opts.host_b
        ),
        _ => format!(
            "Quick take: {} and I are walking through a few key points from this text.",
            opts.host_a
        ),
    };
    turns.push(Turn {
        speaker: opts.host_a.clone(),
        voice: opts.voice_a.clone(),
        text: opener,
        role: "host_a".into(),
    });

    // Main content
    for (i, para) in paragraphs.iter().enumerate() {
        let is_a = i % 2 == 0;
        let (speaker, voice, role) = if is_a {
            (opts.host_a.clone(), opts.voice_a.clone(), "host_a")
        } else {
            (opts.host_b.clone(), opts.voice_b.clone(), "host_b")
        };

        let snippet: String = para.chars().take(280).collect();
        let text = if is_a {
            format!("Here's a core piece: {}", snippet)
        } else {
            format!("Looking at that, the interesting angle is: {}", snippet)
        };

        turns.push(Turn {
            speaker,
            voice,
            text,
            role: role.into(),
        });

        // Occasional short host reaction
        if rng.gen_bool(0.22) {
            let reaction = if rng.gen_bool(0.5) {
                "Yeah, that tracks.".to_string()
            } else {
                "That's the part that stands out.".to_string()
            };
            let (r_speaker, r_voice, r_role) = if is_a {
                (opts.host_b.clone(), opts.voice_b.clone(), "host_b")
            } else {
                (opts.host_a.clone(), opts.voice_a.clone(), "host_a")
            };
            turns.push(Turn {
                speaker: r_speaker,
                voice: r_voice,
                text: reaction,
                role: r_role.into(),
            });
        }

        // Maxx guest insertion (low probability, distinct style)
        if rng.gen_bool(0.14) {
            let maxx_lines = [
                "Hold up — Maxx here. That part hit different.",
                "Yo, Maxx jumping in real quick. That's the real signal.",
                "Maxx with a hot take: most people are missing that angle completely.",
                "Quick side note from Maxx — this is where the actual leverage lives.",
                "Maxx here. Don't sleep on that detail.",
            ];
            let line = maxx_lines[rng.gen_range(0..maxx_lines.len())];
            turns.push(Turn {
                speaker: opts.host_guest.clone(),
                voice: opts.voice_guest.clone(),
                text: line.to_string(),
                role: "guest".into(),
            });
        }
    }

    // Closer
    turns.push(Turn {
        speaker: opts.host_b.clone(),
        voice: opts.voice_b.clone(),
        text: format!(
            "That's the core of it. Thanks for listening. I'm {}, with {}, and sometimes Maxx drops in.",
            opts.host_b, opts.host_a
        ),
        role: "host_b".into(),
    });

    Ok(Script {
        turns,
        seed_used: 0,
    })
}
