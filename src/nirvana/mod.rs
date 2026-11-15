//! Nirvana's *Smells Like Teen Spirit* tone: the pedal (`Patch`), its blocks
//! and its settings (`presets`). Self-contained apart from `crate::pedalkit`,
//! so superPedal carries an identical copy.

mod double_track;
mod ds1;
mod hot_amp;
mod patch;
mod presets;
mod small_clone;

pub use patch::Patch;
