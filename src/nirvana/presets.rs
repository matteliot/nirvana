//! Nirvana's settings: every number that shapes the sound.

use crate::pedalkit::clean::{Attack, CleanSettings};
use crate::pedalkit::eq::Eq;
use crate::pedalkit::gate::GateSettings;

/// Distortion gain (dB) at knob 0, and the extra at knob 1.
pub const DIST_MIN_DB: f32 = 12.0;
pub const DIST_RANGE_DB: f32 = 32.0;
/// Second (preamp) stage gain, dB.
pub const PREAMP_DB: f32 = 10.0;
/// Lead boost: extra distortion, mid push and level (dB).
pub const LEAD_GAIN_DB: f32 = 6.0;
pub const LEAD_MID_DB: f32 = 4.0;
pub const LEAD_LEVEL_DB: f32 = 2.0;

/// The verse sits this far under the chorus (dB): quiet/loud. Plus the clean
/// amp's makeup gain (dB) so the verse lands VERSE_LEVEL_DB under the chorus.
pub const VERSE_LEVEL_DB: f32 = -6.0;
pub const CLEAN_MAKEUP_DB: f32 = 29.0;

/// Verse: the clean amp (drive into its gentle saturation).
pub const CLEAN: CleanSettings = CleanSettings { hp_hz: 90.0, drive: 1.8, attack: Attack::PerSample(0.05), release: 0.05 };

/// Small Clone-style chorus: triangle LFO rate (Hz), centre delay and
/// sweep (samples), wet mix (the original is a fixed 50/50 blend).
pub const CLONE_RATE: f32 = 0.9;
pub const CLONE_BASE: f32 = 260.0;
pub const CLONE_DEPTH: f32 = 110.0;
pub const CLONE_MIX: f32 = 0.5;

/// Double-track: the second "take" lags by this much (samples, ~17 ms) and
/// drifts in time with two slow LFOs (Hz, samples of drift).
pub const DOUBLE_DELAY: f32 = 820.0;
pub const DOUBLE_DRIFT: f32 = 22.0;
pub const DOUBLE_RATE_A: f32 = 0.31;
pub const DOUBLE_RATE_B: f32 = 0.53;

/// Noise gate on the input envelope.
pub const GATE: GateSettings =
    GateSettings { open: 0.0012, close: 0.0006, attack: 0.001, release: 0.08, fade_in: 0.002, fade_out: 0.05 };

/// Output trim: unity Level puts the chorus riff around -14 dB RMS.
pub const OUT_TRIM: f32 = 0.3;

/// Each channel's 4x12.
pub const CAB: [Eq; 6] = [
    Eq::Hp(80.0, 0.707),
    Eq::Peak(110.0, 1.2, 3.0),
    Eq::Peak(2200.0, 1.5, 2.5),
    Eq::Peak(4500.0, 2.0, -3.0),
    Eq::Lp(5600.0, 0.75),
    Eq::Lp(7800.0, 0.6),
];
