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
//! Host builds (`cargo sim`) render a demo riff through the same code; see `host`.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg_attr(not(target_os = "none"), allow(dead_code))]
mod abi;
#[allow(dead_code)]
mod dsp;

use core::cell::UnsafeCell;
use core::f32::consts::TAU;

use abi::*;
use dsp::*;

// ---------------------------------------------------------------- voicing

/// Distortion gain (dB) at knob 0, and the extra at knob 1.
const DIST_MIN_DB: f32 = 12.0;
const DIST_RANGE_DB: f32 = 32.0;
/// Second (preamp) stage gain, dB.
const PREAMP_DB: f32 = 10.0;
/// Lead boost: extra distortion, mid push and level (dB).
const LEAD_GAIN_DB: f32 = 6.0;
const LEAD_MID_DB: f32 = 4.0;
const LEAD_LEVEL_DB: f32 = 2.0;

/// The verse sits this far under the chorus (dB): quiet/loud.
const VERSE_LEVEL_DB: f32 = -6.0;
/// Verse: drive into the clean amp's gentle saturation, and its makeup gain
/// (dB) so the verse lands VERSE_LEVEL_DB under the chorus.
const CLEAN_DRIVE: f32 = 1.8;
const CLEAN_MAKEUP_DB: f32 = 29.0;

/// Small Clone-style chorus: triangle LFO rate (Hz), centre delay and
/// sweep (samples), wet mix (the original is a fixed 50/50 blend).
const CLONE_RATE: f32 = 0.9;
const CLONE_BASE: f32 = 260.0;
const CLONE_DEPTH: f32 = 110.0;
const CLONE_MIX: f32 = 0.5;

/// Double-track: the second "take" lags by this much (samples, ~17 ms) and
/// drifts in time with two slow LFOs (Hz, samples of drift).
const DOUBLE_DELAY: f32 = 820.0;
const DOUBLE_DRIFT: f32 = 22.0;
const DOUBLE_RATE_A: f32 = 0.31;
const DOUBLE_RATE_B: f32 = 0.53;

/// Noise gate thresholds on the input envelope (linear), with hysteresis.
const GATE_OPEN: f32 = 0.0012;
const GATE_CLOSE: f32 = 0.0006;

/// Output trim: unity Level puts the chorus riff around -14 dB RMS.
const OUT_TRIM: f32 = 0.3;

// ---------------------------------------------------------------- state

/// The delay lines. They are all zero at rest, so they live in .bss and add
/// nothing to the patch file; `patch_init` zeroes them before use.
struct Bufs {
    clone_line: Delay<512>,
    dbl_line: Delay<1024>,
}

impl Bufs {
    const NEW: Self = Self { clone_line: Delay::NEW, dbl_line: Delay::NEW };
}

struct Nv {
    ready: bool,
    error: bool,
    // knobs, 0..1
    dist: f32,
    tone: f32,
    level: f32,
    loud: bool,
    lead: bool,
    // smoothed values and targets
    g_dist: f32,
    g_dist_t: f32,
    out_g: f32,
    out_t: f32,
    tone_s: f32,
    mix: f32,
    // filters
    hp_in: Biquad,
    ds_emph: OnePole,
    ds_aa: Biquad,
    ds_lo: OnePole,
    ds_hi: OnePole,
    pre_hp: Biquad,
    amp_bass: Biquad,
    amp_mid: Biquad,
    amp_treble: Biquad,
    lead_mid: Biquad,
    dbl_lp: Biquad,
    clean_hp: Biquad,
    cab: [[Biquad; 6]; 2],
    u1: f32,
    u2: f32,
    // dynamics
    env: f32,
    gate: f32,
    gate_open: bool,
    comp: f32,
    // modulation
    clone_ph: f32,
    dbl_ph_a: f32,
    dbl_ph_b: f32,
    // coefficients
    c_param: f32,
    c_mix: f32,
    c_att: f32,
    c_rel: f32,
    c_gate_open: f32,
    c_gate_close: f32,
    c_comp: f32,
}

impl Nv {
    const NEW: Self = Self {
        ready: false,
        error: false,
        dist: 0.8,
        tone: 0.45,
        level: 0.75,
        loud: true,
        lead: false,
        g_dist: 1.0,
        g_dist_t: 1.0,
        out_g: 0.0,
        out_t: 0.0,
        tone_s: 0.45,
        mix: 1.0,
        hp_in: Biquad::NEW,
        ds_emph: OnePole::NEW,
        ds_aa: Biquad::NEW,
        ds_lo: OnePole::NEW,
        ds_hi: OnePole::NEW,
        pre_hp: Biquad::NEW,
        amp_bass: Biquad::NEW,
        amp_mid: Biquad::NEW,
        amp_treble: Biquad::NEW,
        lead_mid: Biquad::NEW,
        dbl_lp: Biquad::NEW,
        clean_hp: Biquad::NEW,
        cab: [[Biquad::NEW; 6]; 2],
        u1: 0.0,
        u2: 0.0,
        env: 0.0,
        gate: 0.0,
        gate_open: false,
        comp: 1.0,
        clone_ph: 0.0,
        dbl_ph_a: 0.0,
        dbl_ph_b: 0.25,
        c_param: 1.0,
        c_mix: 1.0,
        c_att: 1.0,
        c_rel: 1.0,
        c_gate_open: 1.0,
        c_gate_close: 1.0,
        c_comp: 1.0,
    };

    fn init(&mut self) {
        self.hp_in.highpass(80.0, 0.707);
        // DS-1: highs emphasised into the clipper, then its tone blend of a
        // 230 Hz low-pass and a 1.3 kHz high-pass (the famous mid scoop).
        self.ds_emph.set(700.0);
        self.ds_aa.lowpass(9000.0, 0.707);
        self.ds_lo.set(230.0);
        self.ds_hi.set(1300.0);
        // Hot preamp into an amp EQ with a little low-mid dip.
        self.pre_hp.highpass(110.0, 0.707);
        self.amp_bass.peak(110.0, 0.9, 3.0);
        self.amp_mid.peak(500.0, 0.9, -3.0);
        self.amp_treble.peak(3200.0, 0.9, 3.0);
        self.lead_mid.peak(900.0, 0.8, 0.0);
        self.dbl_lp.lowpass(7000.0, 0.707); // the second take, a touch darker
        self.clean_hp.highpass(90.0, 0.707);
        for cab in self.cab.iter_mut() {
            cab[0].highpass(80.0, 0.707);
            cab[1].peak(110.0, 1.2, 3.0);
            cab[2].peak(2200.0, 1.5, 2.5);
            cab[3].peak(4500.0, 2.0, -3.0);
            cab[4].lowpass(5600.0, 0.75);
            cab[5].lowpass(7800.0, 0.6);
        }
        self.c_param = coef(0.02);
        self.c_mix = coef(0.012);
        self.c_att = coef(0.001);
        self.c_rel = coef(0.08);
        self.c_gate_open = coef(0.002);
        self.c_gate_close = coef(0.05);
        self.c_comp = coef(0.05);
        self.retarget();
        self.g_dist = self.g_dist_t;
        self.out_g = self.out_t;
        self.tone_s = self.tone;
        self.mix = if self.loud { 1.0 } else { 0.0 };
        self.ready = true;
    }

    fn retarget(&mut self) {
        let lead = if self.lead { 1.0 } else { 0.0 };
        self.g_dist_t = db(DIST_MIN_DB + DIST_RANGE_DB * self.dist + LEAD_GAIN_DB * lead);
        // Square law: 0 at minimum, unity at 3 o'clock, +5 dB at maximum.
        let v = self.level / 0.75;
        self.out_t = v * v * OUT_TRIM * db(LEAD_LEVEL_DB * lead);
        self.lead_mid.peak(900.0, 0.8, LEAD_MID_DB * lead);
    }

    /// Chorus voice: DS-1 -> preamp -> amp EQ (mono).
    #[inline]
    fn distortion(&mut self, x: f32) -> f32 {
        let x = x + 1.5 * (x - self.ds_emph.process(x)); // ~+8 dB above 700 Hz
        let u = x * self.g_dist;
        let y = 0.5 * (hard(0.5 * (u + self.u1)) + hard(u));
        self.u1 = u;
        let y = self.ds_aa.process(y);
        let lo = self.ds_lo.process(y);
        let hi = y - self.ds_hi.process(y);
        let t = self.tone_s;
        let y = 0.3 * y + 0.7 * (lo * (1.0 - t) + 1.6 * hi * t);
        let u = self.pre_hp.process(y) * db(PREAMP_DB);
        let y = 0.5 * (tanh(0.5 * (u + self.u2)) + tanh(u));
        self.u2 = u;
        let y = self.amp_treble.process(self.amp_mid.process(self.amp_bass.process(y)));
        self.lead_mid.process(y)
    }

    /// Verse voice: clean amp with gentle compression, then the chorus pedal (stereo).
    #[inline]
    fn clean(&mut self, bf: &mut Bufs, x: f32) -> (f32, f32) {
        let x = self.clean_hp.process(x);
        // Slow peak compressor, then a soft clean-amp edge.
        let a = x.abs() * CLEAN_DRIVE;
        let target = if a > 0.5 { 0.5 / a } else { 1.0 };
        self.comp += (target - self.comp) * if target < self.comp { 0.05 } else { self.c_comp };
        let y = tanh(x * CLEAN_DRIVE * self.comp) * 0.7;
        bf.clone_line.push(y);
        self.clone_ph += CLONE_RATE / SR;
        if self.clone_ph >= 1.0 {
            self.clone_ph -= 1.0;
        }
        // Triangle LFO; the right channel sweeps in the opposite direction.
        let tri = 4.0 * (self.clone_ph - 0.5).abs() - 1.0;
        let wl = bf.clone_line.read(CLONE_BASE + CLONE_DEPTH * tri);
        let wr = bf.clone_line.read(CLONE_BASE - CLONE_DEPTH * tri);
        (y + CLONE_MIX * (wl - y), y + CLONE_MIX * (wr - y))
    }

    /// The second "take": late, drifting and slightly darker.
    #[inline]
    fn double(&mut self, bf: &mut Bufs, d: f32) -> f32 {
        bf.dbl_line.push(d);
        self.dbl_ph_a += DOUBLE_RATE_A / SR;
        self.dbl_ph_b += DOUBLE_RATE_B / SR;
        if self.dbl_ph_a >= 1.0 {
            self.dbl_ph_a -= 1.0;
        }
        if self.dbl_ph_b >= 1.0 {
            self.dbl_ph_b -= 1.0;
        }
        let drift = 0.6 * sin(TAU * self.dbl_ph_a) + 0.4 * sin(TAU * self.dbl_ph_b);
        self.dbl_lp.process(bf.dbl_line.read(DOUBLE_DELAY + DOUBLE_DRIFT * drift))
    }

    #[inline]
    fn tick(&mut self, bf: &mut Bufs, x: f32) -> (f32, f32) {
        self.g_dist += (self.g_dist_t - self.g_dist) * self.c_param;
        self.out_g += (self.out_t - self.out_g) * self.c_param;
        self.tone_s += (self.tone - self.tone_s) * self.c_param;
        let mix_t = if self.loud { 1.0 } else { 0.0 };
        self.mix += (mix_t - self.mix) * self.c_mix;

        let x = self.hp_in.process(x);
        let a = x.abs();
        self.env += (a - self.env) * if a > self.env { self.c_att } else { self.c_rel };
        if self.env > GATE_OPEN {
            self.gate_open = true;
        } else if self.env < GATE_CLOSE {
            self.gate_open = false;
        }
        let (gt, gc) = if self.gate_open { (1.0, self.c_gate_open) } else { (0.0, self.c_gate_close) };
        self.gate += (gt - self.gate) * gc;

        // Both voices always run, so switching is a click-free crossfade.
        let d = self.distortion(x) * self.gate;
        let dr = self.double(bf, d);
        let (cl, cr) = self.clean(bf, x);
        let quiet = db(VERSE_LEVEL_DB + CLEAN_MAKEUP_DB);
        let m = self.mix;
        let mut l = cl * quiet + (d - cl * quiet) * m;
        let mut r = cr * quiet + (dr - cr * quiet) * m;
        for b in self.cab[0].iter_mut() {
            l = b.process(l);
        }
        for b in self.cab[1].iter_mut() {
            r = b.process(r);
        }
        (limit(l * self.out_g), limit(r * self.out_g))
    }

    fn led(&self) -> Color {
        if self.error {
            return Color::Red;
        }
        match (self.loud, self.lead) {
            (false, false) => Color::LightBlue,
            (false, true) => Color::Blue,
            (true, false) => Color::Red,
            (true, true) => Color::Magenta,
        }
    }
}

/// DS-1-style clipper: much harder knee than tanh (cubic, flat beyond |u| = 1.5).
#[inline]
fn hard(u: f32) -> f32 {
    let c = (u * (1.0 / 1.5)).clamp(-1.0, 1.0);
    1.5 * c - 0.5 * c * c * c
}

/// Output soft limiter, never reaching ±1.
#[inline]
fn limit(x: f32) -> f32 {
    tanh(x) * 0.98
}

/// All patch state. The firmware calls every entry point from the audio
/// thread (SDK: "no need to synchronize").
struct Global<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}
static STATE: Global<Nv> = Global(UnsafeCell::new(Nv::NEW));
static BUFS: Global<Bufs> = Global(UnsafeCell::new(Bufs::NEW));

#[allow(clippy::mut_from_ref)]
fn state() -> &'static mut Nv {
    // SAFETY: single-threaded access from the firmware's audio thread.
    unsafe { &mut *STATE.0.get() }
}

#[allow(clippy::mut_from_ref)]
fn bufs() -> &'static mut Bufs {
    // SAFETY: as `state`.
    unsafe { &mut *BUFS.0.get() }
}

/// Zeroes the delay lines in place. The loader should already have cleared
/// .bss, but nothing promises it, and a NaN left in a feedback loop never
/// decays. (In place: a zeroed copy would be a stack temporary.)
fn clear_bufs() {
    // SAFETY: `Bufs` is only f32 and usize fields, so all-zero bits are a
    // valid value; single-threaded as `state`.
    unsafe { core::ptr::write_bytes(BUFS.0.get(), 0, 1) }
}

// ---------------------------------------------------------------- entry points

const PARAM_DIST: i32 = 0;
const PARAM_TONE: i32 = 1;
const PARAM_LEVEL: i32 = 2;

unsafe extern "C" fn patch_init(env: *const PatchEnv) {
    if env.is_null() || unsafe { (*env).abi_version } != PATCH_ABI_VERSION as u32 {
        return;
    }
    clear_bufs();
    state().init();
}

unsafe extern "C" fn update_buffers(_: *const PatchEnv, left: *mut f32, right: *mut f32, samples: i32) -> f32 {
    let Ok(n) = usize::try_from(samples) else { return 0.0 };
    if left.is_null() || right.is_null() || n == 0 {
        return 0.0;
    }
    // SAFETY: the firmware passes two buffers of `samples` floats.
    let (l, r) = unsafe { (core::slice::from_raw_parts_mut(left, n), core::slice::from_raw_parts_mut(right, n)) };
    let (s, bf) = (state(), bufs());
    if !s.ready {
        return 0.0;
    }
    for (a, b) in l.iter_mut().zip(r.iter_mut()) {
        let (ol, or) = s.tick(bf, *a);
        *a = ol;
        *b = or;
    }
    0.0
}

unsafe extern "C" fn set_buffer(_: *const PatchEnv, buffer: *mut f32) {
    // All state is static; the working buffer isn't needed.
    if buffer.is_null() {
        state().error = true;
    }
}

unsafe extern "C" fn get_buffer_size(_: *const PatchEnv) -> i32 {
    WORKING_BUFFER_SIZE
}

unsafe extern "C" fn get_param_min(_: *const PatchEnv, _: i32) -> f32 {
    0.0
}

unsafe extern "C" fn get_param_max(_: *const PatchEnv, _: i32) -> f32 {
    1.0
}

unsafe extern "C" fn get_param_default(_: *const PatchEnv, idx: i32) -> f32 {
    match idx {
        PARAM_DIST => 0.8,
        PARAM_TONE => 0.45,
        _ => 0.75,
    }
}

unsafe extern "C" fn is_param_enabled(_: *const PatchEnv, idx: i32, source_id: i32) -> u8 {
    // Knobs only (source 0), like the SDK wrapper.
    (source_id == 0 && (0..PARAM_COUNT).contains(&idx)) as u8
}

unsafe fn write_c_str(s: &str, out: *mut u8, len: usize) {
    if out.is_null() || len == 0 {
        return;
    }
    let n = s.len().min(len - 1);
    // SAFETY: the firmware provides `len` writable bytes at `out`.
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), out, n);
        *out.add(n) = 0;
    }
}

unsafe extern "C" fn get_param_name(_: *const PatchEnv, idx: i32, out: *mut u8, len: usize) {
    let name = match idx {
        PARAM_DIST => "Dist",
        PARAM_TONE => "Tone",
        PARAM_LEVEL => "Level",
        _ => "",
    };
    unsafe { write_c_str(name, out, len) }
}

unsafe extern "C" fn get_param_unit(_: *const PatchEnv, _: i32, out: *mut u8, len: usize) {
    unsafe { write_c_str("", out, len) }
}

unsafe extern "C" fn set_param(_: *const PatchEnv, idx: i32, value: f32) {
    let s = state();
    let v = value.clamp(0.0, 1.0);
    match idx {
        PARAM_DIST => s.dist = v,
        PARAM_TONE => s.tone = v,
        PARAM_LEVEL => s.level = v,
        _ => return,
    }
    if s.ready {
        s.retarget();
    }
}

unsafe extern "C" fn special_action(_: *const PatchEnv, idx: i32) {
    let s = state();
    match idx {
        ACTION_FOOTSWITCH_PRESS => s.loud = !s.loud,
        ACTION_FOOTSWITCH_HOLD => s.lead = !s.lead,
        _ => return,
    }
    if s.ready {
        s.retarget();
    }
}

unsafe extern "C" fn get_state_idx(_: *const PatchEnv) -> i32 {
    state().led() as i32
}

// ---------------------------------------------------------------- header

#[cfg(target_os = "none")]
unsafe extern "C" {
    static __patch_bss_start: u8;
    static patch_image_size: u8;
    static patch_bss_size: u8;
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".patch_header")]
#[used]
pub static patch_header: PatchHeader = PatchHeader {
    magic: PATCH_MAGIC,
    abi_version: PATCH_ABI_VERSION,
    flags: PATCH_FLAG_NONE,
    init: patch_init,
    agent_update_buffers: update_buffers,
    agent_set_buffer: set_buffer,
    agent_get_buffer_size: get_buffer_size,
    agent_get_param_min: get_param_min,
    agent_get_param_max: get_param_max,
    agent_get_param_default: get_param_default,
    agent_is_param_enabled: is_param_enabled,
    agent_get_param_name: get_param_name,
    agent_get_param_unit: get_param_unit,
    agent_set_param: set_param,
    agent_special_action: special_action,
    agent_get_state_idx: get_state_idx,
    image_size: &raw const patch_image_size,
    bss_begin: &raw const __patch_bss_start,
    bss_size: &raw const patch_bss_size,
    reserved: [0; 16],
};

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
#[allow(dead_code)] // each demo uses only some of the playing techniques
mod guitar;
#[cfg(not(target_os = "none"))]
mod host;

#[cfg(not(target_os = "none"))]
fn main() {
    host::main();
}
