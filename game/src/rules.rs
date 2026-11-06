//! The game rules: how each [`Action`] changes the [`GameState`].
//!
//! This is the only place the state changes. Every action goes through
//! [`GameState::apply`], which either applies it completely or ignores it.
//! What each action does, and when it is allowed, is documented on
//! [`Action`] itself; the tests in `rules/tests.rs` pin that down.
//!
//! No I/O and no randomness here (the server picks the Daily Double and
//! passes it in), so the same state and action always give the same result.

use std::collections::BTreeSet;

use crate::{
    actions::Action::{self},
    content::Game,
    state::{
        ActiveClue,
        ClueStage::{self},
        GameState, Phase, Player, PlayerId,
    },
};

impl GameState {
    /// An empty game in [`Phase::Setup`]: no players, nothing played.
    pub fn new() -> Self {
        GameState {
            players: vec![],
            phase: Phase::Setup,
            board_index: 0,
            used_clues: BTreeSet::new(),
            daily_double: None,
        }
    }

    /// Applies `action` and returns `true`, or ignores it and returns `false`.
    ///
    /// `false` means nothing changed, so the server skips the undo step, the
    /// save and the broadcast. Actions are ignored when they don't fit the
    /// current phase or stage; with phones on wifi that is normal (a late
    /// buzz, a double tap), not a bug.
    ///
    /// Every arm follows the same rule: **check everything first, then
    /// change.** An ignored action must never leave a half-changed state.
    pub fn apply(&mut self, game: &Game, action: Action) -> bool {
        match action {
            Action::AddPlayer { name } => {
                if self.phase != Phase::Setup {
                    return false;
                }

                let name = name.trim();

                if name.is_empty() {
                    return false;
                }

                if self.players.iter().any(|p| p.name == name) {
                    return false;
                }

                // Highest existing id + 1, or 0 for the first player. Ids are
                // never reused while their player exists.
                let next = self
                    .players
                    .iter()
                    .map(|p| p.id.0)
                    .max()
                    .map(|n| n + 1)
                    .unwrap_or(0);
                let id = PlayerId(next);
                let player = Player {
                    id,
                    name: name.to_string(),
                    score: 0,
                };
                self.players.push(player);
                true
            }
            Action::RemovePlayer(id) => {
                if self.phase != Phase::Setup {
                    return false;
                }

                let Some(index) = self.players.iter().position(|p| p.id == id) else {
                    return false;
                };

                self.players.remove(index);

                true
            }
            Action::StartBoard {
                daily_double,
                board,
            } => {
                match self.phase {
                    Phase::Setup if !self.players.is_empty() => {}
                    Phase::BoardEnded => {}
                    _ => return false,
                }

                if board >= game.boards.len() {
                    return false;
                }

                // The server picks the Daily Double; don't trust it blindly.
                // It must exist, and lie on the board being started.
                if daily_double.board != board || game.clue(daily_double).is_none() {
                    return false;
                }

                // Scores are deliberately left alone: they carry over.
                self.board_index = board;
                self.used_clues.clear();
                self.daily_double = Some(daily_double);
                self.phase = Phase::Board;

                true
            }
            Action::EndBoard => {
                if self.phase != Phase::Board {
                    return false;
                }

                self.phase = Phase::BoardEnded;

                true
            }
            Action::SelectClue(clue_id) => {
                if self.phase != Phase::Board {
                    return false;
                }

                // Only clues on the board being played, and only ones that exist.
                if clue_id.board != self.board_index || game.clue(clue_id).is_none() {
                    return false;
                }

                if self.used_clues.contains(&clue_id) {
                    return false;
                }

                // Not marked as used yet: a clue counts as played once it is
                // closed (`CloseClue`).
                self.phase = Phase::Clue(ActiveClue {
                    id: clue_id,
                    stage: ClueStage::Selected,
                    answered_wrong: BTreeSet::new(),
                });

                true
            }
            Action::Flip => {
                let Phase::Clue(active) = &mut self.phase else {
                    return false;
                };

                if active.stage != ClueStage::Selected {
                    return false;
                }

                active.stage = ClueStage::Open { buzzes: vec![] };

                true
            }
            Action::Buzz(player_id) => {
                let Phase::Clue(active) = &mut self.phase else {
                    return false;
                };

                let ClueStage::Open { buzzes } = &mut active.stage else {
                    return false;
                };

                if !self.players.iter().any(|p| p.id == player_id) {
                    return false;
                }

                if buzzes.contains(&player_id) {
                    return false;
                }

                if active.answered_wrong.contains(&player_id) {
                    return false;
                }

                buzzes.push(player_id);

                true
            }
            Action::Correct(player_id) => self.judge(game, player_id, true),
            Action::Wrong(player_id) => self.judge(game, player_id, false),
            Action::NobodyGotIt => {
                let Phase::Clue(active) = &mut self.phase else {
                    return false;
                };

                if !matches!(active.stage, ClueStage::Open { .. }) {
                    return false;
                }

                active.stage = ClueStage::Revealed;
                true
            }
            Action::CloseClue => {
                let Phase::Clue(active) = &self.phase else {
                    return false;
                };

                if active.stage != ClueStage::Revealed {
                    return false;
                }

                // Order matters: `active` borrows `self.phase`, so read its id
                // before the phase is replaced.
                self.used_clues.insert(active.id);
                self.phase = Phase::Board;

                true
            }
            Action::AdjustScore { player_id, delta } => {
                // A zero change would still create an undo step; ignore it.
                if delta == 0 {
                    return false;
                }

                let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) else {
                    return false;
                };

                player.score += delta;

                true
            }
        }
    }

    /// Shared by `Correct` (`correct == true`) and `Wrong`: judges the answer
    /// of the player at the front of the buzz queue.
    ///
    /// Only that player can be judged, which makes a double tap on ✓ or ✗
    /// harmless: the second tap names a player who is no longer first.
    /// Points are the clue's value, doubled for the Daily Double.
    fn judge(&mut self, game: &Game, player_id: PlayerId, correct: bool) -> bool {
        let Phase::Clue(active) = &mut self.phase else {
            return false;
        };

        let ClueStage::Open { buzzes } = &mut active.stage else {
            return false;
        };

        if buzzes.first() != Some(&player_id) {
            return false;
        }

        // Always found (`SelectClue` checked it), but this avoids an unwrap.
        let Some(clue) = game.clue(active.id) else {
            return false;
        };

        let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) else {
            return false;
        };

        let mut points = clue.value as i32;

        if self.daily_double == Some(active.id) {
            points *= 2;
        }

        if correct {
            player.score += points;
            active.stage = ClueStage::Revealed;
        } else {
            // The stage stays `Open`: the next in the queue answers, and
            // others may still buzz. Only the host ends it (`NobodyGotIt`).
            player.score -= points;
            active.answered_wrong.insert(player_id);
            buzzes.remove(0);
        }
        true
    }
}

/// Same as [`GameState::new`].
impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
