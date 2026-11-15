//! DS-1-style distortion: highs emphasised into a hard clipper (2x
//! oversampled), then its tone blend of a 230 Hz low-pass and a 1.3 kHz
//! high-pass (the famous mid scoop).

use crate::pedalkit::dsp::{Biquad, OnePole};
use crate::pedalkit::helpers::{Clip2x, hard};

pub struct Ds1 {
    emph: OnePole,
    clip: Clip2x,
    aa: Biquad,
    lo: OnePole,
    hi: OnePole,
}

impl Ds1 {
    pub const NEW: Self = Self { emph: OnePole::NEW, clip: Clip2x::NEW, aa: Biquad::NEW, lo: OnePole::NEW, hi: OnePole::NEW };

    pub fn init(&mut self) {
        self.emph.set(700.0);
        self.aa.lowpass(9000.0, 0.707);
        self.lo.set(230.0);
        self.hi.set(1300.0);
    }

    /// `g` distortion gain, `t` tone 0..1.
    #[inline]
    pub fn process(&mut self, x: f32, g: f32, t: f32) -> f32 {
        let x = x + 1.5 * (x - self.emph.process(x)); // ~+8 dB above 700 Hz
        let y = self.clip.process(x * g, hard);
        let y = self.aa.process(y);
        let lo = self.lo.process(y);
        let hi = y - self.hi.process(y);
        0.3 * y + 0.7 * (lo * (1.0 - t) + 1.6 * hi * t)
    }
}
