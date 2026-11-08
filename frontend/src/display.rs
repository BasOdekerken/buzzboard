//! /display: the TV. A pure renderer: it draws the [`View`] for
//! [`Role::Display`] and never sends anything.
//!
//! Layout: the current phase fills the screen, with a score bar along the
//! bottom once the game has started. All sizes scale with the screen so the
//! text is readable across the room.

use game::state::{ClueStage, Player, PlayerId};
use game::view::{BoardView, ClueView, Role, View, ViewPhase};
use leptos::prelude::*;

use crate::connection::connect;

#[component]
pub fn Display() -> impl IntoView {
    let conn = connect(Role::Display);

    view! {
        <main class="display">
            {move || match conn.view.get() {
                None => view! { <div class="splash"><p>"Connecting…"</p></div> }.into_any(),
                Some(view) => screen(view),
            }}
            <div class="offline-dot" class:visible=move || !conn.connected.get()></div>
        </main>
    }
}

/// The whole screen for one view.
fn screen(view: View) -> AnyView {
    let View {
        game_name,
        players,
        phase,
        ..
    } = view;

    // The score bar is hidden before the game starts, and after a board,
    // where the ranking already shows the scores.
    let show_scores = matches!(phase, ViewPhase::Board(_) | ViewPhase::Clue { .. });
    let main = match phase {
        ViewPhase::Setup => setup(&game_name, &players),
        ViewPhase::Board(board) => board_grid(board),
        ViewPhase::Clue { clue, .. } => clue_card(&players, clue),
        ViewPhase::BoardEnded => board_ended(&players),
    };
    let score_bar = show_scores.then(|| score_bar(&players));

    view! {
        <div class="stage">{main}</div>
        {score_bar}
    }
    .into_any()
}

// --- Setup ---

fn setup(game_name: &str, players: &[Player]) -> AnyView {
    let names = players
        .iter()
        .map(|p| view! { <li>{p.name.clone()}</li> })
        .collect_view();
    view! {
        <div class="splash">
            <h1>{game_name.to_string()}</h1>
            {if players.is_empty() {
                view! { <p class="sub">"Waiting for players…"</p> }.into_any()
            } else {
                view! { <ul class="joined">{names}</ul> }.into_any()
            }}
        </div>
    }
    .into_any()
}

// --- Board ---

fn board_grid(board: BoardView) -> AnyView {
    let columns = board.categories.len();
    let rows = board
        .categories
        .first()
        .map_or(0, |category| category.tiles.len());

    // Laid out row by row (all headers, then row 0 of every category, ...)
    // so a single CSS grid keeps every row the same height.
    let headers = board
        .categories
        .iter()
        .map(|category| view! { <div class="header">{category.name.clone()}</div> })
        .collect_view();
    let tiles = (0..rows)
        .flat_map(|row| board.categories.iter().map(move |category| &category.tiles[row]))
        .map(|tile| {
            let value = (!tile.used).then_some(tile.value);
            view! { <div class="tile" class:used=tile.used>{value}</div> }
        })
        .collect_view();

    view! {
        <div class="board" style=format!("--columns: {columns}")>
            {headers}
            {tiles}
        </div>
    }
    .into_any()
}

fn board_ended(players: &[Player]) -> AnyView {
    // Highest score first; the score bar below keeps the join order.
    let mut ranking: Vec<&Player> = players.iter().collect();
    ranking.sort_by_key(|p| std::cmp::Reverse(p.score));
    let rows = ranking
        .into_iter()
        .map(|p| {
            view! {
                <li>
                    <span class="name">{p.name.clone()}</span>
                    <span class="score" class:negative={p.score < 0}>{p.score}</span>
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="splash">
            <h1>"Board finished"</h1>
            <ol class="ranking">{rows}</ol>
        </div>
    }
    .into_any()
}

// --- Open clue ---

fn clue_card(players: &[Player], clue: ClueView) -> AnyView {
    let name = |id: PlayerId| {
        players
            .iter()
            .find(|p| p.id == id)
            .map_or_else(String::new, |p| p.name.clone())
    };

    let body = match (&clue.stage, clue.question, clue.answer) {
        // Not flipped yet: only the value, or the Daily Double splash.
        (ClueStage::Selected, _, _) => {
            if clue.daily_double {
                view! {
                    <div class="daily-double">
                        <span>"Daily"</span>
                        <span>"Double"</span>
                    </div>
                }
                .into_any()
            } else {
                view! { <div class="value">{clue.value}</div> }.into_any()
            }
        }
        (ClueStage::Open { .. }, Some(question), _) => {
            view! { <div class="question">{question}</div> }.into_any()
        }
        (ClueStage::Revealed, Some(question), Some(answer)) => view! {
            <div class="question small">{question}</div>
            <div class="answer">{answer}</div>
        }
        .into_any(),
        // The view always fills these in for the matching stage.
        _ => ().into_any(),
    };

    let footer = match &clue.stage {
        ClueStage::Open { buzzes } => {
            let answering = buzzes.first().map(|&id| {
                view! { <span class="answering">{format!("{} answers", name(id))}</span> }
            });
            let wrong = clue
                .answered_wrong
                .iter()
                .map(|&id| view! { <span class="wrong">{name(id)}</span> })
                .collect_view();
            Some(view! { <div class="clue-footer">{answering} {wrong}</div> })
        }
        _ => None,
    };

    let points = if clue.daily_double {
        clue.value * 2
    } else {
        clue.value
    };

    view! {
        <div class="card" class:dd=clue.daily_double>
            <div class="card-head">
                <span>{clue.category}</span>
                <span>{points}</span>
            </div>
            <div class="card-body">{body}</div>
            {footer}
        </div>
    }
    .into_any()
}

// --- Score bar ---

fn score_bar(players: &[Player]) -> AnyView {
    let items = players
        .iter()
        .map(|p| {
            view! {
                <div class="player">
                    <span class="name">{p.name.clone()}</span>
                    <span class="score" class:negative={p.score < 0}>{p.score}</span>
                </div>
            }
        })
        .collect_view();
    view! { <footer class="score-bar">{items}</footer> }.into_any()
}
