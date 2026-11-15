//! The `cargo sim` demo: its score (the rendering itself is `pedalkit::host`).
//!
//! Plays an original power-chord riff at 117 bpm (strums plus muted ghost
//! strums) on a modelled electric guitar (see `guitar`: exact tuning, pick and
//! pickup, strums, vibrato, human timing), drives the patch's entry
//! points the way the firmware does (128-sample blocks, footswitch events at
//! set times) and writes a stereo float WAV. Prints per-section level stats
//! and fails on NaN or clipping.

use crate::pedalkit::guitar::Note;
use crate::pedalkit::host::{Ev, Score};

const E2: f32 = 82.41;
const G2: f32 = 98.0;
const A2: f32 = 110.0;
const C3: f32 = 130.81;
const E3: f32 = 164.81;
const G3: f32 = 196.0;
const A3: f32 = 220.0;
const C4: f32 = 261.63;
const D4: f32 = 293.66;
const E4: f32 = 329.63;

enum Local {
    /// Play these notes, strummed this many seconds apart, releasing what's still ringing.
    Play(Vec<Note>, f32),
    Press,
    Hold,
}

fn power(root: f32, vel: f32) -> Vec<Note> {
    [root, root * 1.4983, root * 2.0].iter().map(|f| Note::new(*f).vel(vel)).collect()
}

/// A muted "chk" strum: the fretting hand lies across the strings, so they barely sound.
fn ghost() -> Vec<Note> {
    [E2 * 1.03, A2 * 1.03, E3 * 1.03].iter().map(|f| Note::new(*f).vel(0.5).pm().t60(0.04)).collect()
}

fn local_score() -> (Vec<(f64, Local, &'static str)>, f64) {
    let mut ev = vec![];
    let e = 60.0 / 117.0 / 2.0; // eighth note
    let mut t = 0.1;
    // Small, repeatable timing slop: nobody plays exactly on the grid.
    let mut seed: u32 = 0x1357_9bdf;
    let mut slop = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f64 / u32::MAX as f64 * 2.0 - 1.0) * 0.006
    };
    let loud_riff = |ev: &mut Vec<(f64, Local, &'static str)>, t: &mut f64, slop: &mut dyn FnMut() -> f64| {
        for _ in 0..2 {
            for root in [E2, G2, A2, C3] {
                // hit, hit, ghost, hit
                for k in 0..4 {
                    let x = if k == 2 { Local::Play(ghost(), 0.005) } else { Local::Play(power(root, if k == 0 { 0.95 } else { 0.85 }), 0.008) };
                    ev.push((*t + slop(), x, ""));
                    *t += e;
                }
            }
        }
    };
    loud_riff(&mut ev, &mut t, &mut slop);
    ev.push((t, Local::Press, "verse (clean + chorus pedal)"));
    t += 0.05;
    for _ in 0..2 {
        for pair in [[E3, A3], [E3, A3], [G3, C4], [G3, C4]] {
            ev.push((t + slop().abs(), Local::Play(pair.iter().map(|f| Note::new(*f).vel(0.7)).collect(), 0.01), ""));
            t += 2.0 * e;
        }
    }
    ev.push((t, Local::Press, "chorus again (loud)"));
    loud_riff(&mut ev, &mut t, &mut slop);
    ev.push((t, Local::Hold, "lead boost (solo line)"));
    for (n, len) in [(A3, 2.0), (C4, 2.0), (D4, 2.0), (E4, 4.0), (D4, 2.0), (C4, 2.0), (A3, 8.0)] {
        let note = Note::new(n).vel(0.9);
        let note = if len >= 4.0 { note.vib() } else { note };
        ev.push((t + slop(), Local::Play(vec![note], 0.0), ""));
        t += len * e;
    }
    (ev, t + 0.5)
}

/// The score as pedalkit events.
pub fn score() -> (Score, f64) {
    let (ev, total) = local_score();
    (ev.into_iter().map(|(t, e, l)| (t as f64, e.into(), l)).collect(), total as f64)
}

impl From<Local> for Ev {
    fn from(e: Local) -> Ev {
        match e {
            Local::Play(notes, strum) => Ev::Play(notes, strum, false),
            Local::Press => Ev::Press,
            Local::Hold => Ev::Hold,
        }
    }
}
