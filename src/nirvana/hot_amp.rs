//! The chorus voice's amp: a hot preamp (tanh, 2x oversampled) into an amp
//! EQ with a little low-mid dip, and the lead boost's mid peak.

use super::presets::{LEAD_MID_DB, PREAMP_DB};
use crate::pedalkit::dsp::{Biquad, db, tanh};
use crate::pedalkit::helpers::Clip2x;

pub struct HotAmp {
    pre_hp: Biquad,
    clip: Clip2x,
    bass: Biquad,
    mid: Biquad,
    treble: Biquad,
    lead_mid: Biquad,
}

impl HotAmp {
    pub const NEW: Self = Self {
        pre_hp: Biquad::NEW,
        clip: Clip2x::NEW,
        bass: Biquad::NEW,
        mid: Biquad::NEW,
        treble: Biquad::NEW,
        lead_mid: Biquad::NEW,
    };

    pub fn init(&mut self) {
        self.pre_hp.highpass(110.0, 0.707);
        self.bass.peak(110.0, 0.9, 3.0);
        self.mid.peak(500.0, 0.9, -3.0);
        self.treble.peak(3200.0, 0.9, 3.0);
        self.lead_mid.peak(900.0, 0.8, 0.0);
    }

    /// The lead boost's 900 Hz mid push, 0 (off) or 1 (on).
    pub fn lead(&mut self, lead: f32) {
        self.lead_mid.peak(900.0, 0.8, LEAD_MID_DB * lead);
    }

    #[inline]
    pub fn process(&mut self, y: f32) -> f32 {
        let u = self.pre_hp.process(y) * db(PREAMP_DB);
        let y = self.clip.process(u, tanh);
        let y = self.treble.process(self.mid.process(self.bass.process(y)));
        self.lead_mid.process(y)
    }
}
