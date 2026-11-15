//! Small Clone-style chorus: a triangle LFO sweeping a short delay, mixed
//! 50/50; the right channel sweeps the opposite way.

use super::presets::{CLONE_BASE, CLONE_DEPTH, CLONE_MIX, CLONE_RATE};
use crate::pedalkit::dsp::SR;
use crate::pedalkit::helpers::Phase;
use crate::pedalkit::pool::{Line, Pool};

pub struct SmallClone {
    line: Line,
    ph: Phase,
}

impl SmallClone {
    pub const NEW: Self = Self { line: Line::NEW, ph: Phase::new(0.0) };

    pub fn init(&mut self, pool: &mut Pool) {
        self.line.take(pool, 512);
    }

    #[inline]
    pub fn process(&mut self, y: f32) -> (f32, f32) {
        self.line.push(y);
        let p = self.ph.step(CLONE_RATE / SR);
        let tri = 4.0 * (p - 0.5).abs() - 1.0;
        let wl = self.line.read(CLONE_BASE + CLONE_DEPTH * tri);
        let wr = self.line.read(CLONE_BASE - CLONE_DEPTH * tri);
        (y + CLONE_MIX * (wl - y), y + CLONE_MIX * (wr - y))
    }
}
