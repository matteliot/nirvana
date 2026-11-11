//! Host-only demo renderer: `cargo sim [OUT.wav]`.
//!
//! Synthesises a Karplus-Strong "guitar" playing an original power-chord riff
//! at 117 bpm (strums plus muted ghost strums), drives the patch's entry
//! points the way the firmware does (128-sample blocks, footswitch events at
//! set times) and writes a stereo float WAV. Prints per-section level stats
//! and fails on NaN or clipping.

use crate::abi::*;
use crate::*;

const N: usize = 128;
const SRU: usize = 48_000;

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
    /// Pluck these notes with this decay per period; `mute` stops whatever
    /// is still ringing first.
    Notes(Vec<f32>, f32, bool),
    /// A muted "chk" strum.
    Ghost,
    Press,
    Hold,
}

struct Ks {
    buf: Vec<f32>,
    i: usize,
    damp: f32,
}

impl Ks {
    fn tick(&mut self) -> f32 {
        let n = self.buf.len();
        let a = self.buf[self.i];
        let b = self.buf[(self.i + 1) % n];
        self.buf[self.i] = self.damp * 0.5 * (a + b);
        self.i = (self.i + 1) % n;
        a
    }
}

fn power(root: f32) -> Vec<f32> {
    vec![root, root * 1.4983, root * 2.0]
}

fn score() -> (Vec<(f64, Ev, &'static str)>, f64) {
    let mut ev = vec![];
    let e = 60.0 / 117.0 / 2.0; // eighth note
    let mut t = 0.1;
    let loud_riff = |ev: &mut Vec<(f64, Ev, &'static str)>, t: &mut f64| {
        for _ in 0..2 {
            for root in [E2, G2, A2, C3] {
                // hit, hit, ghost, hit
                for k in 0..4 {
                    let x = if k == 2 { Ev::Ghost } else { Ev::Notes(power(root), 0.996, true) };
                    ev.push((*t, x, ""));
                    *t += e;
                }
            }
        }
    };
    loud_riff(&mut ev, &mut t);
    ev.push((t, Ev::Press, "verse (clean + chorus pedal)"));
    t += 0.05;
    for _ in 0..2 {
        for pair in [[E3, A3], [E3, A3], [G3, C4], [G3, C4]] {
            ev.push((t, Ev::Notes(pair.to_vec(), 0.998, true), ""));
            t += 2.0 * e;
        }
    }
    ev.push((t, Ev::Press, "chorus again (loud)"));
    loud_riff(&mut ev, &mut t);
    ev.push((t, Ev::Hold, "lead boost (solo line)"));
    for (n, len) in [(A3, 2.0), (C4, 2.0), (D4, 2.0), (E4, 4.0), (D4, 2.0), (C4, 2.0), (A3, 8.0)] {
        ev.push((t, Ev::Notes(vec![n], 0.998, true), ""));
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
    events.reverse();
    let total = (total_s * SRU as f64) as usize;
    let mut voices: Vec<Ks> = vec![];
    let mut seed: u32 = 0x1234_5678;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    };
    let mut pluck = |f: f32, amp: f32, damp: f32| {
        let n = (SRU as f32 / f).round() as usize;
        let mut prev = 0.0;
        let buf = (0..n)
            .map(|_| {
                prev = 0.5 * prev + 0.5 * rnd();
                prev * amp
            })
            .collect();
        Ks { buf, i: 0, damp }
    };

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
                Ev::Notes(notes, damp, mute) => {
                    if mute {
                        voices.iter_mut().for_each(|v| v.damp = 0.6);
                    }
                    for f in notes {
                        voices.push(pluck(f, 0.12, damp));
                    }
                }
                Ev::Ghost => {
                    voices.iter_mut().for_each(|v| v.damp = 0.6);
                    for f in [E2, A2, E3] {
                        voices.push(pluck(f * 1.03, 0.08, 0.55));
                    }
                }
            }
            voices.retain(|v| v.damp > 0.9 || v.buf.iter().any(|x| x.abs() > 1e-4));
            if !label.is_empty() {
                sections.push((t, label));
            }
        }
        let mut l = [0.0f32; N];
        let mut r = [0.0f32; N];
        for x in l.iter_mut() {
            *x = voices.iter_mut().map(Ks::tick).sum();
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
