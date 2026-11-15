//! The whole patch. Two voices run all the time and the footswitch
//! crossfades between them:
//!   Verse   clean amp, light compression, Small Clone-style chorus (stereo)
//!   Chorus  DS-1-style distortion -> hot preamp -> amp EQ, double-tracked:
//!           the right channel is a second, late and drifting "take"
//! then a 4x12 cab sim per channel and the output level.

use super::double_track::DoubleTrack;
use super::ds1::Ds1;
use super::hot_amp::HotAmp;
use super::presets::*;
use super::small_clone::SmallClone;
use crate::pedalkit::abi::Color;
use crate::pedalkit::clean::CleanAmp;
use crate::pedalkit::dsp::{Biquad, db};
use crate::pedalkit::eq::EqChain;
use crate::pedalkit::gate::Gate;
use crate::pedalkit::helpers::{Smooth, limit};
use crate::pedalkit::pedal::{Param, Pedal};
use crate::pedalkit::pool::Pool;

/// Knob indices.
const DIST: usize = 0;
const TONE: usize = 1;
const LEVEL: usize = 2;

pub struct Patch {
    knobs: [f32; 3],
    loud: bool,
    lead: bool,
    gain: Smooth,
    out: Smooth,
    tone: Smooth,
    mix: Smooth,
    hp_in: Biquad,
    gate: Gate,
    ds1: Ds1,
    amp: HotAmp,
    double: DoubleTrack,
    clean: CleanAmp,
    clone: SmallClone,
    cab_l: EqChain<6>,
    cab_r: EqChain<6>,
}

impl Patch {
    fn retarget(&mut self) {
        let lead = if self.lead { 1.0 } else { 0.0 };
        self.gain.set(db(DIST_MIN_DB + DIST_RANGE_DB * self.knobs[DIST] + LEAD_GAIN_DB * lead));
        // Square law: 0 at minimum, unity at 3 o'clock, +5 dB at maximum.
        let v = self.knobs[LEVEL] / 0.75;
        self.out.set(v * v * OUT_TRIM * db(LEAD_LEVEL_DB * lead));
        self.tone.set(self.knobs[TONE]);
        self.mix.on(self.loud);
        self.amp.lead(lead);
    }
}

impl Pedal for Patch {
    const PARAMS: [Param; 3] = [
        // Distortion gain, ~12 to 44 dB.
        Param { name: "Dist", default: 0.8 },
        // DS-1 tone: CCW full/dark, noon scooped, CW bright/fizzy.
        Param { name: "Tone", default: 0.45 },
        // Output; unity at 3 o'clock.
        Param { name: "Level", default: 0.75 },
    ];
    const POOL: usize = 512 + 1024;

    const NEW: Self = Self {
        knobs: [Self::PARAMS[0].default, Self::PARAMS[1].default, Self::PARAMS[2].default],
        // The patch starts in Chorus.
        loud: true,
        lead: false,
        gain: Smooth::new(1.0),
        out: Smooth::new(0.0),
        tone: Smooth::new(Self::PARAMS[TONE].default),
        mix: Smooth::new(1.0),
        hp_in: Biquad::NEW,
        gate: Gate::NEW,
        ds1: Ds1::NEW,
        amp: HotAmp::NEW,
        double: DoubleTrack::NEW,
        clean: CleanAmp::NEW,
        clone: SmallClone::NEW,
        cab_l: EqChain::NEW,
        cab_r: EqChain::NEW,
    };

    fn init(&mut self, pool: &mut Pool) {
        self.hp_in.highpass(80.0, 0.707);
        self.gate.init(&GATE);
        self.ds1.init();
        self.amp.init();
        self.double.init(pool);
        self.clean.init(&CLEAN);
        self.clone.init(pool);
        self.cab_l.init(&CAB);
        self.cab_r.init(&CAB);
        for s in [&mut self.gain, &mut self.out, &mut self.tone] {
            s.time(0.02);
        }
        self.mix.time(0.012);
        self.retarget();
        self.gain.snap();
        self.out.snap();
        self.tone.snap();
        self.mix.snap();
    }

    fn set_knob(&mut self, idx: usize, v: f32) {
        self.knobs[idx] = v;
        self.retarget();
    }

    /// Footswitch press: Verse <-> Chorus.
    fn press(&mut self) {
        self.loud = !self.loud;
        self.retarget();
    }

    /// Footswitch hold: lead boost on/off.
    fn hold(&mut self) {
        self.lead = !self.lead;
        self.retarget();
    }

    #[inline]
    fn tick(&mut self, x: f32, _: f32) -> (f32, f32) {
        let g = self.gain.tick();
        let out = self.out.tick();
        let t = self.tone.tick();
        let m = self.mix.tick();
        let x = self.hp_in.process(x);
        let gate = self.gate.process(x);
        // Both voices always run, so switching is a click-free crossfade.
        let d = self.amp.process(self.ds1.process(x, g, t)) * gate;
        let dr = self.double.process(d);
        let (cl, cr) = self.clone.process(self.clean.process(x));
        let quiet = db(VERSE_LEVEL_DB + CLEAN_MAKEUP_DB);
        let l = cl * quiet + (d - cl * quiet) * m;
        let r = cr * quiet + (dr - cr * quiet) * m;
        let l = self.cab_l.process(l);
        let r = self.cab_r.process(r);
        (limit(l * out), limit(r * out))
    }

    fn led(&self) -> Color {
        match (self.loud, self.lead) {
            (false, false) => Color::LightBlue,
            (false, true) => Color::Blue,
            (true, false) => Color::Red,
            (true, true) => Color::Magenta,
        }
    }
}
