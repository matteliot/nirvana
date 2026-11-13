//! Host-only demo renderer: `cargo sim [OUT.wav]`.
//!
//! Plays an original power-chord riff at 117 bpm (strums plus muted ghost
//! strums) on a modelled electric guitar (see `guitar`: exact tuning, pick and
//! pickup, strums, vibrato, human timing), drives the patch's entry
//! points the way the firmware does (128-sample blocks, footswitch events at
//! set times) and writes a stereo float WAV. Prints per-section level stats
//! and fails on NaN or clipping.

use crate::guitar::{Note, Player};
use crate::*;

const N: usize = 128;
const SRU: usize = 48_000;
/// The modelled guitar into this patch, matched to the level the patch was voiced for.
const INPUT_TRIM: f32 = 0.6;

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

enum Ev {
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

fn score() -> (Vec<(f64, Ev, &'static str)>, f64) {
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
    let loud_riff = |ev: &mut Vec<(f64, Ev, &'static str)>, t: &mut f64, slop: &mut dyn FnMut() -> f64| {
        for _ in 0..2 {
            for root in [E2, G2, A2, C3] {
                // hit, hit, ghost, hit
                for k in 0..4 {
                    let x = if k == 2 { Ev::Play(ghost(), 0.005) } else { Ev::Play(power(root, if k == 0 { 0.95 } else { 0.85 }), 0.008) };
                    ev.push((*t + slop(), x, ""));
                    *t += e;
                }
            }
        }
    };
    loud_riff(&mut ev, &mut t, &mut slop);
    ev.push((t, Ev::Press, "verse (clean + chorus pedal)"));
    t += 0.05;
    for _ in 0..2 {
        for pair in [[E3, A3], [E3, A3], [G3, C4], [G3, C4]] {
            ev.push((t + slop().abs(), Ev::Play(pair.iter().map(|f| Note::new(*f).vel(0.7)).collect(), 0.01), ""));
            t += 2.0 * e;
        }
    }
    ev.push((t, Ev::Press, "chorus again (loud)"));
    loud_riff(&mut ev, &mut t, &mut slop);
    ev.push((t, Ev::Hold, "lead boost (solo line)"));
    for (n, len) in [(A3, 2.0), (C4, 2.0), (D4, 2.0), (E4, 4.0), (D4, 2.0), (C4, 2.0), (A3, 8.0)] {
        let note = Note::new(n).vel(0.9);
        let note = if len >= 4.0 { note.vib() } else { note };
        ev.push((t + slop(), Ev::Play(vec![note], 0.0), ""));
        t += len * e;
    }
    (ev, t + 0.5)
}

pub fn main() {
    let out = std::env::args().nth(1).unwrap_or("target/render/nirvana_demo.wav".into());
    let env = PatchEnv { abi_version: PATCH_ABI_VERSION as u32, user_ctx: core::ptr::null_mut() };
    let mut work = vec![0.0f32; 16];
    unsafe {
        patch_init(&env);
        set_buffer(&env, work.as_mut_ptr());
        for i in 0..PARAM_COUNT {
            set_param(&env, i, get_param_default(&env, i));
        }
    }

    let (mut events, total_s) = score();
    events.sort_by(|a, b| b.0.total_cmp(&a.0));
    let total = (total_s * SRU as f64) as usize;
    let mut guitar = Player::new();

    let mut lo = vec![];
    let mut ro = vec![];
    let mut sections: Vec<(usize, &str)> = vec![(0, "chorus (loud power chords)")];
    let mut t = 0;
    while t < total {
        while events.last().is_some_and(|(et, _, _)| (*et * SRU as f64) as usize <= t) {
            let (_, e, label) = events.pop().unwrap();
            match e {
                Ev::Press => unsafe { special_action(&env, ACTION_FOOTSWITCH_PRESS) },
                Ev::Hold => unsafe { special_action(&env, ACTION_FOOTSWITCH_HOLD) },
                Ev::Play(notes, strum) => guitar.play(&notes, strum, false),
            }
            if !label.is_empty() {
                sections.push((t, label));
            }
        }
        let mut l = [0.0f32; N];
        let mut r = [0.0f32; N];
        for x in l.iter_mut() {
            *x = guitar.tick() * INPUT_TRIM;
        }
        unsafe { update_buffers(&env, l.as_mut_ptr(), r.as_mut_ptr(), N as i32) };
        lo.extend_from_slice(&l);
        ro.extend_from_slice(&r);
        t += N;
    }

    let mut bad = false;
    sections.push((lo.len(), ""));
    println!("{:<30} {:>7} {:>8} {:>8} {:>8}", "section", "peak", "rms dB", "L-R dB", "L/R corr");
    for w in sections.windows(2) {
        let (a, name) = w[0];
        let b = w[1].0;
        let (l, r) = (&lo[a..b], &ro[a..b]);
        let peak = l.iter().chain(r).fold(0.0f32, |m, v| m.max(v.abs()));
        let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        let side: Vec<f32> = l.iter().zip(r).map(|(a, b)| a - b).collect();
        let dot: f64 = l.iter().zip(r).map(|(a, b)| *a as f64 * *b as f64).sum();
        let corr = dot / (rms(l) * rms(r) * l.len() as f64).max(1e-12);
        let to_db = |v: f64| 20.0 * v.max(1e-9).log10();
        println!("{name:<30} {peak:>7.3} {:>8.1} {:>8.1} {corr:>8.2}", to_db(rms(l)), to_db(rms(&side)));
        if l.iter().chain(r).any(|v| !v.is_finite()) || peak >= 1.0 {
            bad = true;
        }
    }
    println!("final LED colour index: {}", unsafe { get_state_idx(&env) });

    if let Some(dir) = std::path::Path::new(&out).parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    write_wav(&out, &lo, &ro);
    println!("wrote {out} ({:.1} s)", lo.len() as f64 / SRU as f64);
    if bad {
        eprintln!("error: NaN or full-scale output");
        std::process::exit(1);
    }
    let _ = (get_buffer_size, get_param_min, get_param_max, is_param_enabled, get_param_name, get_param_unit);
}

fn write_wav(path: &str, l: &[f32], r: &[f32]) {
    let bytes = (l.len() * 8) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + bytes).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&(48_000u32 * 8).to_le_bytes());
    b.extend_from_slice(&8u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&bytes.to_le_bytes());
    for (a, c) in l.iter().zip(r) {
        b.extend_from_slice(&a.to_le_bytes());
        b.extend_from_slice(&c.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}
