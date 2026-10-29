use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EvidencePack {
    pub version: String,
    pub seed: u64,
    pub content_hash: String,
    pub generated_at_unix: u64,
    pub voice_a: String,
    pub voice_b: String,
    pub voice_guest: String,
    pub host_a: String,
    pub host_b: String,
    pub host_guest: String,
    pub turn_count: usize,
    pub output_path: String,
    pub notes: String,
}
