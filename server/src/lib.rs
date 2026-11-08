//! Server library: the shared game session and the WebSocket handling.
//! `main.rs` wires these into an axum app.

pub mod session;
pub mod ws;
