//! /host: the host's phone. Shows everything (including answers and where
//! the Daily Double is) and has every control.
//!
//! The page is redrawn from the latest [`View`] whenever it changes. Buttons
//! only send messages; what happens is decided by the server, and the result
//! comes back as a new view.

use game::actions::Action;
use game::protocol::ClientMessage;
use game::state::{ClueStage, Player, PlayerId};
use game::view::{BoardView, ClueView, Role, View, ViewPhase};
use leptos::html;
use leptos::prelude::*;

use crate::connection::{Connection, connect};

#[component]
pub fn Host() -> impl IntoView {
    let conn = connect(Role::Host);

    let title = move || {
        conn.view
            .with(|view| view.as_ref().map(|v| v.game_name.clone()))
            .unwrap_or_default()
    };

    view! {
        <main class="host">
            <header class="topbar">
                <span class="title">{title}</span>
                <span class="status" class:offline=move || !conn.connected.get()>
                    {move || if conn.connected.get() { "connected" } else { "reconnecting…" }}
                </span>
                <button class="undo" on:click=move |_| conn.send(ClientMessage::Undo)>
                    "Undo"
                </button>
            </header>
            {move || match conn.view.get() {
                None => view! { <p class="hint">"Connecting…"</p> }.into_any(),
                Some(view) => page(conn, view),
            }}
        </main>
    }
}

/// The page for one view: the panel for the current phase, then the scores.
fn page(conn: Connection, view: View) -> AnyView {
    let View {
        players,
        phase,
        board_names,
        ..
    } = view;

    let in_setup = matches!(phase, ViewPhase::Setup);
    let main = match phase {
        ViewPhase::Setup => setup(conn, &players, &board_names),
        ViewPhase::Board(board) => board_panel(conn, board),
        ViewPhase::Clue { clue, .. } => clue_panel(conn, &players, clue),
        ViewPhase::BoardEnded => board_ended(conn, &board_names),
    };
    let scores = (!in_setup).then(|| scores(conn, players));

    view! { {main} {scores} }.into_any()
}

// --- Setup ---

fn setup(conn: Connection, players: &[Player], board_names: &[String]) -> AnyView {
    let input: NodeRef<html::Input> = NodeRef::new();
    let add = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let Some(field) = input.get() else {
            return;
        };
        let name = field.value();
        if !name.trim().is_empty() {
            conn.action(Action::AddPlayer { name });
            field.set_value("");
        }
    };

    let can_start = !players.is_empty();
    let players = players
        .iter()
        .map(|player| {
            let id = player.id;
            view! {
                <li>
                    <span class="name">{player.name.clone()}</span>
                    <button class="remove" on:click=move |_| conn.action(Action::RemovePlayer(id))>
                        "✕"
                    </button>
                </li>
            }
        })
        .collect_view();

    view! {
        <section class="panel">
            <h2>"Players"</h2>
            <form class="add-player" on:submit=add>
                <input node_ref=input type="text" placeholder="Name" autocomplete="off" />
                <button type="submit">"Add"</button>
            </form>
            <ul class="list">{players}</ul>
        </section>
        <section class="panel">
            <h2>"Start"</h2>
            {start_buttons(conn, board_names, can_start)}
            {(!can_start).then(|| view! { <p class="hint">"Add a player first."</p> })}
        </section>
    }
    .into_any()
}

/// One button per board. The server picks the Daily Double.
fn start_buttons(conn: Connection, board_names: &[String], enabled: bool) -> AnyView {
    let buttons = board_names
        .iter()
        .enumerate()
        .map(|(board, name)| {
            view! {
                <button
                    class="big"
                    disabled={!enabled}
                    on:click=move |_| conn.send(ClientMessage::StartBoard { board })
                >
                    {format!("Start {name}")}
                </button>
            }
        })
        .collect_view();
    view! { <div class="stack">{buttons}</div> }.into_any()
}

// --- Board ---

fn board_panel(conn: Connection, board: BoardView) -> AnyView {
    let columns = board.categories.len();
    let categories = board
        .categories
        .into_iter()
        .map(|category| {
            let tiles = category
                .tiles
                .into_iter()
                .map(|tile| {
                    let id = tile.id;
                    view! {
                        <button
                            class="tile"
                            class:used=tile.used
                            class:dd=tile.daily_double
                            disabled=tile.used
                            on:click=move |_| conn.action(Action::SelectClue(id))
                        >
                            {(!tile.used).then_some(tile.value)}
                        </button>
                    }
                })
                .collect_view();
            view! {
                <div class="column">
                    <div class="category">{category.name}</div>
                    {tiles}
                </div>
            }
        })
        .collect_view();

    view! {
        <section class="panel">
            <h2>{board.name}</h2>
            <div class="mini-board" style=format!("--columns: {columns}")>{categories}</div>
            <p class="hint">"★ = Daily Double (only you can see this)"</p>
            <button class="secondary" on:click=move |_| conn.action(Action::EndBoard)>
                "End board"
            </button>
        </section>
    }
    .into_any()
}

fn board_ended(conn: Connection, board_names: &[String]) -> AnyView {
    view! {
        <section class="panel">
            <h2>"Board finished"</h2>
            <p class="hint">"Start another board. The final round comes later."</p>
            {start_buttons(conn, board_names, true)}
        </section>
    }
    .into_any()
}

// --- Open clue ---

fn clue_panel(conn: Connection, players: &[Player], clue: ClueView) -> AnyView {
    let name = |id: PlayerId| {
        players
            .iter()
            .find(|p| p.id == id)
            .map_or_else(|| format!("Player {}", id.0), |p| p.name.clone())
    };
    let points = if clue.daily_double {
        clue.value * 2
    } else {
        clue.value
    };

    let controls = match clue.stage {
        ClueStage::Selected => view! {
            <button class="big primary" on:click=move |_| conn.action(Action::Flip)>
                "Flip card"
            </button>
        }
        .into_any(),
        ClueStage::Open { buzzes } => {
            open_controls(conn, players, &buzzes, &clue.answered_wrong, name)
        }
        ClueStage::Revealed => view! {
            <button class="big primary" on:click=move |_| conn.action(Action::CloseClue)>
                "Back to board"
            </button>
        }
        .into_any(),
    };

    view! {
        <section class="panel clue">
            <div class="clue-meta">
                <span>{clue.category}</span>
                <span class="points">{points}</span>
                {clue.daily_double.then(|| view! { <span class="badge">"Daily Double ×2"</span> })}
            </div>
            <p class="question">{clue.question}</p>
            <p class="answer">"Answer: " <strong>{clue.answer}</strong></p>
        </section>
        <section class="panel">{controls}</section>
    }
    .into_any()
}

/// While the buzzers are open: the queue, ✓/✗ for whoever is first, buttons
/// to enter a buzz by hand (phase 1, no player phones), and "nobody".
fn open_controls(
    conn: Connection,
    players: &[Player],
    buzzes: &[PlayerId],
    answered_wrong: &std::collections::BTreeSet<PlayerId>,
    name: impl Fn(PlayerId) -> String,
) -> AnyView {
    let judge = buzzes.first().copied().map(|first| {
        let who = name(first);
        view! {
            <p class="answering">{format!("{who} answers")}</p>
            <div class="judge">
                <button class="correct" on:click=move |_| conn.action(Action::Correct(first))>
                    "✓ Correct"
                </button>
                <button class="wrong" on:click=move |_| conn.action(Action::Wrong(first))>
                    "✗ Wrong"
                </button>
            </div>
        }
    });

    let queue = buzzes
        .iter()
        .skip(1)
        .map(|&id| view! { <li>{name(id)}</li> })
        .collect_view();

    let can_buzz = players
        .iter()
        .filter(|p| !buzzes.contains(&p.id) && !answered_wrong.contains(&p.id))
        .map(|player| {
            let id = player.id;
            view! {
                <button on:click=move |_| conn.action(Action::Buzz(id))>
                    {player.name.clone()}
                </button>
            }
        })
        .collect_view();

    let wrong = answered_wrong.iter().map(|&id| name(id)).collect::<Vec<_>>();

    view! {
        {judge}
        {(buzzes.len() > 1).then(|| view! { <p class="label">"Next up"</p> <ol class="list">{queue}</ol> })}
        <p class="label">"Who buzzed?"</p>
        <div class="buzz-buttons">{can_buzz}</div>
        {(!wrong.is_empty()).then(|| view! { <p class="hint">{format!("Wrong: {}", wrong.join(", "))}</p> })}
        <button class="secondary" on:click=move |_| conn.action(Action::NobodyGotIt)>
            "Nobody: reveal answer"
        </button>
    }
    .into_any()
}

// --- Scores ---

fn scores(conn: Connection, players: Vec<Player>) -> AnyView {
    let rows = players
        .into_iter()
        .map(|player| {
            let id = player.id;
            let adjust = move |delta| {
                conn.action(Action::AdjustScore {
                    player_id: id,
                    delta,
                })
            };
            view! {
                <li>
                    <span class="name">{player.name}</span>
                    <button class="adjust" on:click=move |_| adjust(-1)>"−"</button>
                    <span class="score" class:negative={player.score < 0}>{player.score}</span>
                    <button class="adjust" on:click=move |_| adjust(1)>"+"</button>
                </li>
            }
        })
        .collect_view();

    view! {
        <section class="panel">
            <h2>"Scores"</h2>
            <ul class="list scores">{rows}</ul>
        </section>
    }
    .into_any()
}
