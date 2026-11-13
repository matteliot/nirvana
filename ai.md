# ai.md — Project Requirements Log

This file records every requirement given for the **nirvana** project, so the whole project can be recreated from scratch by handing this file to an AI assistant as the prompt.

It is kept up to date as new requirements are given. Requirements are listed in the order they were given; later entries override earlier ones where they conflict (and the earlier entry is marked as
superseded).

---

## Meta / Process Requirements

1. Maintain this `ai.md` file at the repo root. Its purpose is to keep a list of all requirements so the entire project can be recreated from this file alone. Update it as work progresses, whenever a
   new requirement is given.

## Project Requirements

2. **Build a Polyend Endless patch binary, the same way as the sibling project `../soundGarden`, for "the most Nirvana 'Smells Like Teen Spirit' guitar sound ever."** Build it with `cargo build
   --release`.
   - The target sound is quiet/loud dynamics: a clean verse with a Small Clone-style chorus, and a loud chorus with DS-1-style hard-clipping distortion and its scooped tone, into a hot amp and a 4x12.
   - The distorted guitars are double-tracked, like *Nevermind*.
3. Push the project to its GitHub remote (`origin`).

4. On `master`, add the demo render (`cargo sim` output) as `samples/nirvana_demo.wav`. Commit with the message "ai" and push.

5. Keep `ai.md` lines unwrapped: only wrap where needed, or at a natural word break around 200 characters.

6. The demo's guitar sounded fake ("like MIDI"): plain Karplus-Strong noise plucks, out of tune up high, robotic timing. Make the demo guitar realistic by modelling it better (not by using samples), the same way as `../ledZeppelin` (its `src/guitar.rs`, copied here unchanged), then re-render `samples/nirvana_demo.wav`.

### Implementation decisions (needed to recreate requirement 2)

- **Toolchain and packaging (copied from `../soundGarden`).**
  - Language and target: `no_std` Rust, edition 2024, nightly toolchain, `thumbv7em-none-eabihf`.
  - Copied files: `src/abi.rs` (FxPatchSDK ABI 0x000B header), `patch.ld`, `build.rs`, `.cargo/config.toml` (pedal is the default target, `-fp64`), and `src/dsp.rs` (f32-only sin/cos/exp2/tanh
    approximations, biquads, one-pole, delay lines).
  - `build-endl.sh` makes `target/endless/nirvana.endl` with `llvm-objcopy`. It checks for f64 instructions and the 512 KB limit; `--deploy` copies the file to `/Volumes/ENDLESS`.
- **Two voices.** Both always run (input is the left jack), and the footswitch press crossfades between them over 12 ms.
  - **Verse:**
    1. HP 90 Hz.
    2. Fast-attack peak compressor.
    3. `tanh(1.8x) * 0.7`.
    4. Small Clone-style chorus: triangle LFO at 0.9 Hz, 260 ± 110 sample delay, 50/50 mix. The right channel uses the inverted sweep.
    5. Level: -6 dB relative to the chorus, plus 29 dB of clean makeup. That lands the verse about 8 dB RMS under the chorus.
  - **Chorus:**
    1. HP 80 Hz input.
    2. +1.5x of the content above 700 Hz is added, as emphasis into the clipper.
    3. Distortion gain 12–44 dB (Dist knob), then a cubic hard clipper (`1.5c - 0.5c³`, flat beyond |u| = 1.5) with 2x oversampling.
    4. LP 9 kHz.
    5. DS-1 tone: `0.3·y + 0.7·(LP230·(1-t) + 1.6·HP1300·t)`.
    6. HP 110 Hz, then a +10 dB preamp into tanh (2x oversampling).
    7. Amp EQ: +3 dB at 110 Hz, -3 dB at 500 Hz, +3 dB at 3.2 kHz, and the lead mid peak at 900 Hz.
    8. Gate (opens at 0.0012, closes at 0.0006).
  - **Double-track:** the right channel of the chorus voice is the same signal delayed 820 samples. The delay drifts by ±22 samples from 0.31 Hz and 0.53 Hz sines, then goes through LP 7 kHz.
- **Cab and output.**
  - Each channel has its own 4x12 sim: HP 80 Hz, +3 dB at 110 Hz, +2.5 dB at 2.2 kHz, -3 dB at 4.5 kHz, LP 5.6 kHz and LP 7.8 kHz.
  - Output is Level × 0.3 trim, then `tanh` × 0.98.
- **Controls.**
  - Knobs: Dist (default 0.8), Tone (default 0.45), Level (square law, unity at 0.75).
  - Press: Verse ↔ Chorus. The patch starts in Chorus.
  - Hold: Lead boost (+6 dB gain, +4 dB at 900 Hz, +2 dB level).
  - LED: verse light blue / blue, chorus red / magenta (normal / lead). Red also means the working buffer is missing.
- **Host audition.** `cargo sim` renders an *original* power-chord riff at 117 bpm, not the song's actual riff, on the modelled guitar (see below), at ×0.6 to match the level the patch was voiced for. Power chords are strummed (8 ms per string), the ghost strums are fret-muted "chk"s (T60 0.04 s), the long lead notes get vibrato, and timing is nudged by up to ±6 ms, repeatably. It plays chorus → verse dyads → chorus → lead line. The output is `target/render/nirvana_demo.wav`, and it prints per-section peak, RMS, side level and L/R correlation.

### Implementation decisions (needed to recreate requirement 6): the guitar model, `src/guitar.rs` (host only)

- **String.** Extended Karplus-Strong per note: a 4096-sample delay line read with 4-point Lagrange interpolation, so the loop delay is fractional. The loop has a one-zero loss filter `g·((1-s)·y + s·y₋₁)`, and the read delay is `SR/f - s`, so every note is in tune (measured within 0.2 cents from E2 to B5; the old integer-length string was up to ~16 cents flat).
  - Loop gain per period `g = 10^(-3 / (T60·f))`. Held T60 = `6·sqrt(82.41/f)` s, clamped to 1.8–6 s. Palm mute: T60 0.18 s, s = 0.5. Otherwise s = 0.18 + 0.12·(1 - velocity).
  - Releasing a note (the next note, unless it's allowed to ring) switches T60 to 0.06 s.
  - Two polarizations: the second is detuned +0.7 cents, decays at 0.8× the T60, and is mixed at 0.45.
- **Pick.** The excitation is one period of one-pole low-passed noise at 1.2 kHz + 6 kHz × velocity (×0.45 when palm muted), combed at the picking position (0.16 ± 0.04 of the string, random per note), with the mean removed and normalized to 0.12 × velocity RMS. Plus a click: 96 samples of differentiated noise × 0.015 × velocity, decaying over 20 samples.
  - Attack pitch: +7 cents × velocity, decaying with a 50 ms time constant.
- **Pitch movement (cents).** Vibrato 5.3 Hz, ±18 cents, fading in after 0.22 s over 0.35 s. Bend: smoothstep to the target over its duration. Slide: from N semitones below, smoothstep over its duration.
- **Pickup.** Each polarization is heard as `y - y(0.075·period)` (a bridge humbucker comb). The sum is ×0.55, then a 4.3 kHz low-pass with Q 1.3 (pickup and cable resonance) and HP 60 Hz.
- **Player.** A note can override its T60 (a fret-hand-muted "chk", or a chord sustained in front of a loud amp). Velocity varies ±12% per note, and onsets by up to 1.5 ms. Chords strum low to high (14 ms per string down, 10 ms up). The voice is dropped 0.4 s after release, or after 1.5 × T60.

7. **Buffers are zero-initialized in `.bss` and explicitly zeroed in `patch_init` before use** (rather than shipped as zeros inside the patch image). The delay lines (and reverb, where there is one) move out of the patch state into a separate `Bufs` static that is all zero at rest, so it lands in `.bss` instead of being stored as zeros in the `.endl` (runtime-length reverb combs/allpasses start with length 0 and get their lengths in init). The `Global` cell is generic; `tick` and the methods that use a delay line take `&mut Bufs`. `patch_init` then clears each buffer static in place with `core::ptr::write_bytes` (no heap or `calloc` in `no_std`, and a zeroed copy would be a stack temporary) before `init`: the loader should already have cleared `.bss`, but nothing promises it, and a NaN left in a feedback loop never decays. The firmware working buffer stays unused.
