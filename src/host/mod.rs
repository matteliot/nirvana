//! Host-only demo: `cargo sim [OUT.wav]` plays `score` on the modelled
//! guitar through the patch (see `pedalkit::host`).

mod score;

use crate::entry::ENTRY;
use crate::pedalkit::host::{self, Options};

pub fn main() {
    let out = host::out_path("target/render/nirvana_demo.wav");
    let (events, total) = score::score();
    let o = Options { first: "chorus (loud power chords)", input_trim: 0.6, working: 16 };
    let r = host::render(&ENTRY, events, total, o);
    host::finish(&ENTRY, r, &out);
}
