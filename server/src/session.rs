//! The one shared game session: content, current state, undo history.
//!
//! Every WebSocket connection holds a [`Shared`] handle to the same session.
//! Lock it only briefly and never across an `.await`.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use game::{content::Game, state::GameState};
use serde_json::to_string_pretty;
use tokio::sync::watch;

/// Everything the server knows about the game night.
pub struct Session {
    /// The content (boards, clues). Read-only, validated at startup.
    pub game: Game,
    /// The current state. Change it only through `GameState::apply` or undo,
    /// and call [`Session::changed`] afterwards.
    pub state: GameState,
    /// Previous states, oldest first; undo pops from the back. Capped by the
    /// caller. Kept in memory only: a restart starts with empty history.
    pub undo: VecDeque<GameState>,
    /// "Something changed" signal. Each connection subscribes and sends its
    /// client a fresh view when it fires; the value itself carries nothing.
    pub changes: watch::Sender<()>,
    /// Where the state is saved: `data/<game file name>.state.json`.
    pub save_path: PathBuf,
}

/// The session as shared between connections.
pub type Shared = Arc<Mutex<Session>>;

impl Session {
    /// Call after every change to `state`: saves it to disk, then notifies
    /// every connection. Saving first means a client never sees a state that
    /// would be lost on a crash. A failed save is logged, not fatal: the game
    /// keeps going.
    pub fn changed(&self) {
        if let Err(err) = self.save() {
            eprintln!("Can't save {}: {err}", self.save_path.display());
        }
        self.changes.send_replace(());
    }

    /// Writes the state as pretty JSON (readable and fixable by hand).
    ///
    /// Writes to a temporary file first and then renames it over the real
    /// one. A rename is atomic, so a crash mid-write never leaves a
    /// half-written save file: it's either the old state or the new one.
    fn save(&self) -> std::io::Result<()> {
        let json = to_string_pretty(&self.state).map_err(std::io::Error::other)?;
        let tmp = self.save_path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.save_path)
    }
}
