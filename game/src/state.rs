//! Game state: everything that changes during a game night.
//!
//! The server owns one [`GameState`], changes it only through the reducer,
//! saves it to disk after every change and sends role-filtered views of it
//! to the clients. Static content (boards, clues) lives in [`crate::content`].

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::content::ClueId;

/// Stable identifier for a player. Not an index into [`GameState::players`],
/// so it stays valid if a player is removed.
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct PlayerId(u32);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    /// Can go negative: a wrong answer subtracts the clue's value.
    pub score: i32,
}

/// Where we are in the game night.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub enum Phase {
    /// Entering player names, before the first board.
    Setup,
    /// The board is showing; the host opens the next clue.
    Board,
    /// A clue is open on the TV.
    Clue(ActiveClue),
}

/// The clue that is currently open, and how far it has progressed.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub struct ActiveClue {
    pub id: ClueId,
    pub stage: ClueStage,
    /// Players who answered this clue wrong; they can't buzz again on it.
    /// Kept outside [`ClueStage`] so it survives every stage of the clue.
    pub locked_out: BTreeSet<PlayerId>
}

/// The steps of a single clue: `Selected` → `Open` → `Revealed`.
/// A Daily Double goes through the same steps; only its scoring differs.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub enum ClueStage {
    /// The card is zoomed in on the TV, showing only its value (or the
    /// Daily Double animation). The question is hidden and buzzers are locked.
    Selected,
    /// The card is flipped: the question is visible and buzzers are live.
    /// `buzzes` is in arrival order; the first player in it is answering.
    /// A wrong answer removes them from here and adds them to
    /// [`ActiveClue::locked_out`].
    Open { buzzes: Vec<PlayerId> },
    /// The answer is shown on the TV. The next action closes the clue
    /// and returns to the board.
    Revealed
}

/// The complete state of a game night. This is what gets saved to disk.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub struct GameState {
    /// In join order; this is also the order of the score bar.
    pub players: Vec<Player>,
    pub phase: Phase,
    /// Index into [`crate::content::Game::boards`] of the board being played.
    pub board_index: usize,
    /// Clues that have been played and closed; shown as empty tiles.
    pub used_clues: BTreeSet<ClueId>,
    /// The Daily Double on the current board. `None` until the board starts
    /// (the server picks it then). Never sent to /display or /play before
    /// that clue is selected.
    pub daily_double: Option<ClueId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state in the middle of a clue, so every type is exercised.
    fn mid_clue_state() -> GameState {
        let clue = ClueId {
            board: 0,
            category: 2,
            row: 1,
        };
        GameState {
            players: vec![
                Player {
                    id: PlayerId(0),
                    name: "Anna".into(),
                    score: 3,
                },
                Player {
                    id: PlayerId(1),
                    name: "Bram".into(),
                    score: -2,
                },
                Player {
                    id: PlayerId(2),
                    name: "Cas".into(),
                    score: 0,
                },
            ],
            phase: Phase::Clue(ActiveClue {
                id: clue,
                stage: ClueStage::Open {
                    buzzes: vec![PlayerId(2), PlayerId(0)],
                },
                locked_out: BTreeSet::from([PlayerId(1)]),
            }),
            board_index: 0,
            used_clues: BTreeSet::from([
                ClueId {
                    board: 0,
                    category: 0,
                    row: 0,
                },
                ClueId {
                    board: 0,
                    category: 4,
                    row: 3,
                },
            ]),
            daily_double: Some(clue),
        }
    }

    #[test]
    fn state_survives_json_round_trip() {
        let state = mid_clue_state();
        let json = serde_json::to_string(&state).unwrap();
        let back: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
    }

    #[test]
    fn empty_setup_state_survives_json_round_trip() {
        let state = GameState {
            players: vec![],
            phase: Phase::Setup,
            board_index: 0,
            used_clues: BTreeSet::new(),
            daily_double: None,
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
    }
}
