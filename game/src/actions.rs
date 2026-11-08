//! Actions: everything that can change the [`GameState`](crate::state::GameState).
//!
//! Clients (the host phone, later the player phones) send actions to the
//! server, which applies them with [`GameState::apply`](crate::state::GameState::apply). Undo is not an action: the
//! server handles it by restoring a previous state.
//!
//! An action that doesn't fit the current phase or stage (a late buzz, a
//! double tap on ✗) is ignored rather than treated as a bug.

use serde::{Deserialize, Serialize};

use crate::{content::ClueId, state::PlayerId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    // --- Setup ---
    /// Adds a player with score 0. Setup only.
    AddPlayer { name: String },
    /// Removes a player. Setup only; to fix a typo, remove and add again.
    RemovePlayer(PlayerId),

    // --- Any time ---
    /// Manual score correction by the host. `delta` may be negative.
    AdjustScore { player_id: PlayerId, delta: i32 },

    // --- Boards ---
    /// Starts the board the host chose (index into `Game::boards`), from
    /// `Setup` (needs at least one player) or `BoardEnded`. Any board may be
    /// started, including one already played. Resets used clues; scores carry
    /// over. The server picks `daily_double` at random on that board, so the
    /// rules stay pure; it must lie on `board`.
    StartBoard { board: usize, daily_double: ClueId },
    /// Ends the current board, even with tiles left. Only from `Board`
    /// (close an open clue first).
    EndBoard,

    // --- Clues ---
    /// Opens a clue: the card zooms in on the TV showing its value
    /// (`Selected` stage). Only from `Board`, and only for unused clues.
    SelectClue(ClueId),
    /// Flips the card: shows the question and opens the buzzers (`Open`).
    Flip,
    /// A player buzzes in and joins the end of the queue. Sent by a player's
    /// phone, or by the host tapping a name in phase 1. Ignored outside `Open`,
    /// for players already in the queue, and for players who already answered
    /// this clue wrong.
    Buzz(PlayerId),
    /// The player answered correctly: +value (×2 for the Daily Double), and
    /// the clue moves to `Revealed`. Ignored unless this player is first in
    /// the queue.
    Correct(PlayerId),
    /// The player answered wrong: −value (×2 for the Daily Double), they
    /// can't buzz again on this clue, and the next player in the queue
    /// answers. The clue stays open, even if nobody is left; the host ends it
    /// with `NobodyGotIt`. Ignored unless this player is first in the queue.
    Wrong(PlayerId),
    /// Nobody (left) knows it: reveal the answer with no score change.
    NobodyGotIt,
    /// Closes a revealed clue: marks it used and returns to the board.
    CloseClue,
}
