# nirvana

A [Polyend Endless](https://polyend.com/endless/) patch for the most *Smells Like
Teen Spirit* guitar sound ever, built around the song's quiet/loud dynamics. It's
written in `no_std` Rust against the patch ABI of Polyend's
[FxPatchSDK](https://github.com/polyend/FxPatchSDK) (ABI 0x000B), so no ARM GCC is needed.

Two voices run all the time and the footswitch crossfades between them:

- **Verse**: a clean amp with gentle compression into a Small Clone-style chorus
  (a triangle LFO at 0.9 Hz with a fixed 50/50 mix). The left and right channels
  sweep in opposite directions.
- **Chorus**: DS-1-style hard-clipping distortion with highs pushed into the
  clipper. Then the DS-1 tone blend (230 Hz low-pass against a 1.3 kHz high-pass,
  which gives its mid scoop), a hot preamp and an amp EQ. It's **double-tracked**:
  the right channel is a second "take", ~17 ms late, drifting and a little darker,
  like the layered *Nevermind* guitars.

Both go through a 4x12 cab sim per channel. Input comes from the left jack.

| Control | Function |
|---|---|
| Left knob | **Dist**: distortion gain, about 12 to 44 dB |
| Mid knob | **Tone**: DS-1 tone. CCW = full/dark, noon = scooped, CW = bright/fizzy |
| Right knob | **Level**: output, unity at 3 o'clock |
| Footswitch press | **Verse ↔ Chorus**. The verse sits about 8 dB under the chorus |
| Footswitch hold | **Lead boost**: +6 dB gain, +4 dB at 900 Hz, +2 dB level, for the solo |

| LED | Normal | Lead boost |
|---|---|---|
| Verse | light blue | blue |
| Chorus | red | magenta |

## Build

```sh
rustup target add thumbv7em-none-eabihf --toolchain nightly
rustup component add llvm-tools --toolchain nightly
cargo build --release        # ELF for the pedal (default target is thumbv7em-none-eabihf)
./build-endl.sh              # build + make target/endless/nirvana.endl, check f64 use and size
./build-endl.sh --deploy     # ...and copy to /Volumes/ENDLESS
```

## Audition on your computer

```sh
cargo sim                    # -> target/render/nirvana_demo.wav
```

This renders an original power-chord riff at 117 bpm (strums plus muted ghost
strums) through the same patch code. It goes chorus → verse → chorus → lead
boost, then prints the level and stereo width of each section.
