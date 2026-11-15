//! The second "take" of a double-tracked guitar: late, drifting in time and
//! slightly darker, like the layered *Nevermind* guitars.

use super::presets::{DOUBLE_DELAY, DOUBLE_DRIFT, DOUBLE_RATE_A, DOUBLE_RATE_B};
use crate::pedalkit::dsp::{Biquad, SR, sin};
use crate::pedalkit::helpers::Phase;
use crate::pedalkit::pool::{Line, Pool};
use core::f32::consts::TAU;

pub struct DoubleTrack {
    line: Line,
    a: Phase,
    b: Phase,
    lp: Biquad,
}

impl DoubleTrack {
    pub const NEW: Self = Self { line: Line::NEW, a: Phase::new(0.0), b: Phase::new(0.25), lp: Biquad::NEW };

    pub fn init(&mut self, pool: &mut Pool) {
        self.line.take(pool, 1024);
        self.lp.lowpass(7000.0, 0.707);
    }

    #[inline]
    pub fn process(&mut self, d: f32) -> f32 {
        self.line.push(d);
        let pa = self.a.step(DOUBLE_RATE_A / SR);
        let pb = self.b.step(DOUBLE_RATE_B / SR);
        let drift = 0.6 * sin(TAU * pa) + 0.4 * sin(TAU * pb);
        self.lp.process(self.line.read(DOUBLE_DELAY + DOUBLE_DRIFT * drift))
    }
}
