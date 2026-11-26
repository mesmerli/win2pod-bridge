//! # win2pod-ipc
//!
//! Inter-process communication library and protocol definitions for Win2Pod Bridge.
//! Provides clean, zero-copy, zero-GPL message types and transport mechanisms
//! over Windows Named Pipes.

pub mod protocol;
#[cfg(windows)]
pub mod transport;

pub use protocol::*;
#[cfg(windows)]
pub use transport::*;
