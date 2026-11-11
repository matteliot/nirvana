# ai.md — Project Requirements Log

This file records every requirement given for the **nirvana** project, so the whole
project can be recreated from scratch by handing this file to an AI assistant as the prompt.

It is kept up to date as new requirements are given. Requirements are listed in the order
they were given; later entries override earlier ones where they conflict (and the earlier
entry is marked as superseded).

---

## Meta / Process Requirements

1. Maintain this `ai.md` file at the repo root. Its purpose is to keep a list of all
   requirements so the entire project can be recreated from this file alone. Update it
   as work progresses, whenever a new requirement is given.

## Project Requirements

2. **Build a Polyend Endless patch binary, the same way as the sibling project
   `../soundGarden`, for "the most Nirvana 'Smells Like Teen Spirit' guitar sound
   ever."** Build it with `cargo build --release`.
   - The target sound is quiet/loud dynamics: a clean verse with a Small Clone-style
     chorus, and a loud chorus with DS-1-style hard-clipping distortion and its scooped
     tone, into a hot amp and a 4x12.
   - The distorted guitars are double-tracked, like *Nevermind*.
3. Push the project to its GitHub remote (`origin`).

4. On `master`, add the demo render (`cargo sim` output) as `samples/nirvana_demo.wav`. Commit with the message "ai" and push.

### Implementation decisions (needed to recreate requirement 2)

- **Toolchain and packaging (copied from `../soundGarden`).**
  - Language and target: `no_std` Rust, edition 2024, nightly toolchain,
    `thumbv7em-none-eabihf`.
  - Copied files: `src/abi.rs` (FxPatchSDK ABI 0x000B header), `patch.ld`, `build.rs`,
    `.cargo/config.toml` (pedal is the default target, `-fp64`), and `src/dsp.rs`
    (f32-only sin/cos/exp2/tanh approximations, biquads, one-pole, delay lines).
  - `build-endl.sh` makes `target/endless/nirvana.endl` with `llvm-objcopy`. It checks
    for f64 instructions and the 512 KB limit; `--deploy` copies the file to
    `/Volumes/ENDLESS`.
- **Two voices.** Both always run (input is the left jack), and the footswitch press
  crossfades between them over 12 ms.
  - **Verse:**
    1. HP 90 Hz.
    2. Fast-attack peak compressor.
    3. `tanh(1.8x) * 0.7`.
    4. Small Clone-style chorus: triangle LFO at 0.9 Hz, 260 ± 110 sample delay, 50/50
       mix. The right channel uses the inverted sweep.
    5. Level: -6 dB relative to the chorus, plus 29 dB of clean makeup. That lands the
       verse about 8 dB RMS under the chorus.
  - **Chorus:**
    1. HP 80 Hz input.
    2. +1.5x of the content above 700 Hz is added, as emphasis into the clipper.
    3. Distortion gain 12–44 dB (Dist knob), then a cubic hard clipper
       (`1.5c - 0.5c³`, flat beyond |u| = 1.5) with 2x oversampling.
    4. LP 9 kHz.
    5. DS-1 tone: `0.3·y + 0.7·(LP230·(1-t) + 1.6·HP1300·t)`.
    6. HP 110 Hz, then a +10 dB preamp into tanh (2x oversampling).
    7. Amp EQ: +3 dB at 110 Hz, -3 dB at 500 Hz, +3 dB at 3.2 kHz, and the lead mid peak
       at 900 Hz.
    8. Gate (opens at 0.0012, closes at 0.0006).
  - **Double-track:** the right channel of the chorus voice is the same signal delayed
    820 samples. The delay drifts by ±22 samples from 0.31 Hz and 0.53 Hz sines, then
    goes through LP 7 kHz.
- **Cab and output.**
  - Each channel has its own 4x12 sim: HP 80 Hz, +3 dB at 110 Hz, +2.5 dB at 2.2 kHz,
    -3 dB at 4.5 kHz, LP 5.6 kHz and LP 7.8 kHz.
  - Output is Level × 0.3 trim, then `tanh` × 0.98.
- **Controls.**
  - Knobs: Dist (default 0.8), Tone (default 0.45), Level (square law, unity at 0.75).
  - Press: Verse ↔ Chorus. The patch starts in Chorus.
  - Hold: Lead boost (+6 dB gain, +4 dB at 900 Hz, +2 dB level).
  - LED: verse light blue / blue, chorus red / magenta (normal / lead). Red also means
    the working buffer is missing.
- **Host audition.** `cargo sim` renders an *original* power-chord riff at 117 bpm, not
  the song's actual riff. It uses Karplus-Strong strums plus muted ghost strums and
  plays chorus → verse dyads → chorus → lead line. The output is
  `target/render/nirvana_demo.wav`, and it prints per-section peak, RMS, side level and
  L/R correlation.
