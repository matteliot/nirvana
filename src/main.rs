//! nirvana: a Polyend Endless patch for the most "Smells Like Teen Spirit"
//! guitar sound ever, built around the song's quiet/loud dynamics.
//!
//! Two voices run all the time and the footswitch crossfades between them:
//!   Verse   clean amp, light compression, Small Clone-style chorus (stereo)
//!   Chorus  DS-1-style hard-clipping distortion -> DS-1 tone blend (mid
//!           scoop) -> hot preamp -> amp EQ, double-tracked: the right channel
//!           is a second, slightly late and drifting "take", like layered
//!           Nevermind guitars
//! then a 4x12 cab sim per channel and the output level.
//!
//! Controls
//!   Left knob   Dist   (distortion gain, ~12 to 44 dB)
//!   Mid knob    Tone   (DS-1 tone: CCW full/dark, noon scooped, CW bright/fizzy)
//!   Right knob  Level  (output; unity at 3 o'clock)
//!   Footswitch press  Verse <-> Chorus (quiet/loud; the verse sits ~6 dB under the chorus)
//!   Footswitch hold   Lead boost on/off (more gain, mids and level, for the solo)
//!   LED  Verse: light blue | Chorus: red | with lead boost: blue | magenta
//!
//! The sound lives in `nirvana` (built from `pedalkit` blocks set by its
//! `presets`); the entry points come from `pedalkit::endless_patch!`.
//! Host builds (`cargo sim`) render a demo riff through the same code; see `host`.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

/// Reusable blocks (effects, DSP, the Endless ABI); see its docs.
mod pedalkit;

/// The pedal itself.
mod nirvana;

endless_patch!(kit = crate::pedalkit, pedal = crate::nirvana::Patch);

#[cfg(not(target_os = "none"))]
mod host;

#[cfg(not(target_os = "none"))]
fn main() {
    host::main();
}
