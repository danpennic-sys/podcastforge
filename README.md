# PodcastForge

Minimal offline 2-person podcast generator with seeded **Maxx** guest.

- Pure Rust CLI
- External Piper TTS (fully offline once models are local)
- Deterministic under seed
- Evidence JSON pack on every run
- Host A / Host B + occasional Maxx (distinct voice + style)

## Quick start

```bash
# 1. Install piper binary from https://github.com/rhasspy/piper/releases
# 2. Place models in ./models/
#    en_US-amy-medium.onnx (+ .json)
#    en_US-ryan-medium.onnx (+ .json)
#    en_US-lessac-medium.onnx (+ .json)   # Maxx voice

cargo build --release

./target/release/podcastforge generate \
  --text "Your article or notes here..." \
  --seed 42 \
  --output output/proof.wav
```

Same seed + same models = identical structure and Maxx placement.

## Evidence

Every run writes `*.evidence.json` next to the WAV containing seed, content hash, voices, and timestamps.

## License

MIT OR Apache-2.0
