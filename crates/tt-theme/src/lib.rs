//! tTorrent's design tokens (DESIGN.md is the spec; this crate is its source of
//! truth in code). Every colour, size and duration the window draws comes from
//! here, never from a literal in the UI.

pub mod color;
mod tokens;

pub use color::{Oklch, Rgba, contrast, mix, ok};
pub use tokens::*;
