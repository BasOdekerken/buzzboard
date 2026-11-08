//! Views: what each screen is allowed to see.
//!
//! Clients never get the [`Game`] (it holds every answer) or the raw
//! [`GameState`] (it holds the Daily Double location). Instead the server
//! sends each client a [`View`] built by [`view_for`] for its [`Role`]. A
//! view is self-contained: a page draws it without needing anything else.
//!
//! A view is data, not layout. The TV and the host phone get the same type;
//! each page decides how to draw it, and only the host page has buttons.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{
    content::{ClueId, Game},
    state::{ActiveClue, ClueStage, GameState, Phase, Player, PlayerId},
};

/// Who is looking. Player phones (phase 2) will get their own small view.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The host phone (/host): sees everything, including answers and
    /// where the Daily Double is.
    Host,
    /// The TV (/display): only what the room may see right now.
    Display,
}

/// Everything one screen needs to draw the game.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct View {
    pub game_name: String,
    /// Same for every role: names and scores are public.
    pub players: Vec<Player>,
    pub phase: ViewPhase,
}

/// Like [`Phase`], but carrying what is needed to draw it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ViewPhase {
    Setup,
    Board(BoardView),
    /// The board stays included, so the TV can keep it behind the card.
    Clue {
        board: BoardView,
        clue: ClueView,
    },
    BoardEnded,
}

/// The board being played.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BoardView {
    pub name: String,
    /// Left to right.
    pub categories: Vec<CategoryView>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CategoryView {
    pub name: String,
    /// Top (cheapest) to bottom.
    pub tiles: Vec<TileView>,
}

/// One tile on the board.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TileView {
    /// What the host page sends in `SelectClue` when this tile is tapped.
    pub id: ClueId,
    pub value: u32,
    /// Already played: drawn as an empty tile.
    pub used: bool,
    /// Host only. Always `false` for [`Role::Display`], so the TV can't
    /// tell where the Daily Double is.
    pub daily_double: bool,
}

/// The clue that is open.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClueView {
    /// Category name.
    pub category: String,
    /// The clue's normal value; scoring doubles it for the Daily Double.
    pub value: u32,
    /// For every role, as soon as the clue is selected (the TV plays the
    /// Daily Double animation then).
    pub daily_double: bool,
    /// Includes the buzz queue when `Open`.
    pub stage: ClueStage,
    pub answered_wrong: BTreeSet<PlayerId>,
    /// Host: always. Display: from the flip on (`Open`, `Revealed`).
    pub question: Option<String>,
    /// Host: always. Display: only when `Revealed`.
    pub answer: Option<String>,
}

/// Builds the view of `state` that `role` may see.
pub fn view_for(game: &Game, state: &GameState, role: Role) -> View {
    let phase = match &state.phase {
        Phase::Setup => ViewPhase::Setup,
        Phase::BoardEnded => ViewPhase::BoardEnded,
        Phase::Board => ViewPhase::Board(board_view(game, state, &role)),
        Phase::Clue(active) => ViewPhase::Clue {
            board: board_view(game, state, &role),
            clue: clue_view(game, state, active, &role),
        },
    };

    View {
        game_name: game.name.clone(),
        phase,
        players: state.players.clone(),
    }
}

fn board_view(game: &Game, state: &GameState, role: &Role) -> BoardView {
    // Safe to index: `StartBoard` only accepts boards that exist.
    let board = &game.boards[state.board_index];

    let categories = board
        .categories
        .iter()
        .enumerate()
        .map(|(c, category)| {
            let tiles = category
                .clues
                .iter()
                .enumerate()
                .map(|(r, clue)| {
                    let id = ClueId {
                        board: state.board_index,
                        category: c,
                        row: r,
                    };
                    TileView {
                        id,
                        value: clue.value,
                        used: state.used_clues.contains(&id),
                        daily_double: *role == Role::Host && state.daily_double == Some(id),
                    }
                })
                .collect();
            CategoryView {
                name: category.name.clone(),
                tiles,
            }
        })
        .collect();

    BoardView {
        name: board.name.clone(),
        categories,
    }
}

fn clue_view(game: &Game, state: &GameState, active: &ActiveClue, role: &Role) -> ClueView {
    // Safe to index: `SelectClue` only accepts clues that exist.
    let category = &game.boards[active.id.board].categories[active.id.category];
    let clue = &category.clues[active.id.row];

    let host = *role == Role::Host;
    let flipped = active.stage != ClueStage::Selected;
    let revealed = active.stage == ClueStage::Revealed;

    ClueView {
        category: category.name.clone(),
        value: clue.value,
        daily_double: state.daily_double == Some(active.id),
        stage: active.stage.clone(),
        answered_wrong: active.answered_wrong.clone(),
        question: (host || flipped).then(|| clue.question.clone()),
        answer: (host || revealed).then(|| clue.answer.clone()),
    }
}

#[cfg(test)]
mod tests;
