//! Window Snapping (Magnetic Windows) & Group Movement
//! Inspired by Winamp: automatically snaps windows to screen edges / taskbar
//! and to each other when close, with linked movement when bonded.
//!
//! Uses a Generalized Directed Dock Tree (Forest) architecture:
//! - Priority: Main Window (Lv 1) > Mini Player (Lv 2) > Equalizer (Lv 3) > Visualizer (Lv 4)
//! - Each Slave window has at most ONE Master (parent) it is bonded to.
//! - Master movement automatically propagates down the entire subtree recursively.
//! - Dragging a Slave window detaches it if pulled away (> 24px) without pulling Master.
//! - Window HWND discovery uses exact Win32 window title matching to prevent viewport ID collisions.

pub mod types;
pub mod geometry;
pub mod opacity;
pub mod cluster;
pub mod animation;
pub mod subclass;
#[cfg(test)]
mod tests;

pub use types::*;
pub use geometry::*;
pub use opacity::*;
pub use cluster::*;
pub use animation::*;
pub use subclass::*;
#[cfg(test)]
pub use tests::*;
