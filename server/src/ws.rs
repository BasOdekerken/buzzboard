//! WebSocket connections: one task per connected screen.
//!
//! A client connects to `/ws?role=host` or `/ws?role=display`. It receives
//! its [`View`](game::view::View) right away and again after every change.
//! Only the host may send messages; anything the display sends is dropped.

use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use game::{
    actions::Action,
    view::{Role, view_for},
};
use rand::seq::IndexedRandom;
use serde::Deserialize;

use crate::session::{Session, Shared};

/// Query string of `/ws`. An unknown role is rejected by axum with a 400.
#[derive(Debug, Deserialize)]
pub struct WsParams {
    role: Role,
}

/// What the host phone can send, as JSON:
/// `{"Action": {"AddPlayer": {"name": "Anna"}}}` or `"Undo"`.
#[derive(Deserialize)]
enum ClientMessage {
    /// A game action, applied through the rules.
    Action(Action),
    /// Restore the state from before the last accepted change.
    Undo,
    /// Start this board; the server picks the Daily Double at random
    /// (see `Game::daily_double_candidates`).
    StartBoard { board: usize },
}

/// How many states undo can go back.
const UNDO_DEPTH: usize = 20;

/// Upgrades `/ws` to a WebSocket and starts a task for the connection.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(session): State<Shared>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, session, params.role))
}

/// Runs one connection until the client leaves or the server stops.
async fn handle_socket(mut socket: WebSocket, session: Shared, role: Role) {
    // Subscribe before sending the first view, so a change that happens in
    // between is not missed. The lock is held only inside this block.
    let mut changes = {
        let Ok(guard) = session.lock() else { return };
        guard.changes.subscribe()
    };

    if !send_view(&mut socket, &session, &role).await {
        return;
    }

    // Wait for whichever comes first: a message from this client, or a
    // change made by anyone (this client included).
    loop {
        tokio::select! {
            message = socket.recv() => match message {
                Some(Ok(Message::Text(text))) => {
                    if role == Role::Host {
                        handle_message(&session, text.as_str());
                    }
                }
                // Pings, binary frames: nothing to do.
                Some(Ok(_)) => {}
                // Closed, errored or gone: end this connection.
                _ => break,
            },
            result = changes.changed() => {
                // Err only when the sender is gone, i.e. the server stops.
                if result.is_err() { break };
                if !send_view(&mut socket, &session, &role).await { break }
            }
        }
    }
}

/// Sends the client its view of the current state.
/// Returns `false` if the client is gone.
async fn send_view(socket: &mut WebSocket, session: &Shared, role: &Role) -> bool {
    // Build the JSON while holding the lock, then release it before the
    // `.await` on send: a slow client must not block everyone else.
    let json = {
        let Ok(guard) = session.lock() else {
            return false;
        };
        let view = view_for(&guard.game, &guard.state, *role);
        let Ok(message) = serde_json::to_string(&view) else {
            return false;
        };
        message
    };
    socket.send(Message::Text(json.into())).await.is_ok()
}

/// Handles one message from the host. Not `async` on purpose: it only
/// locks, changes and signals, so the lock can't be held across an `.await`.
fn handle_message(session: &Shared, text: &str) {
    let Ok(message) = serde_json::from_str::<ClientMessage>(text) else {
        eprintln!("Ignoring unreadable message: {text}");
        return;
    };
    let Ok(mut guard) = session.lock() else {
        return;
    };
    // Borrow through the guard once, so `state` and `game` can be used as
    // separate fields below (`apply` needs `&mut state` and `&game`).
    let session = &mut *guard;

    match message {
        ClientMessage::Action(action) => apply(session, action),
        ClientMessage::Undo => {
            // Empty history: nothing to undo, nothing to send.
            if let Some(previous) = session.undo.pop_back() {
                session.state = previous;
                session.changed();
            }
        }
        ClientMessage::StartBoard { board } => {
            let candidates = session.game.daily_double_candidates(board);
            // No candidates: the board doesn't exist. Nothing to do.
            let Some(&daily_double) = candidates.choose(&mut rand::rng()) else {
                return;
            };
            apply(
                session,
                Action::StartBoard {
                    board,
                    daily_double,
                },
            );
        }
    }
}

/// Applies an action through the rules. If it was accepted: keep the old
/// state for undo (capped at `UNDO_DEPTH`), save, and notify everyone.
/// Ignored actions leave no undo step and send nothing.
fn apply(session: &mut Session, action: Action) {
    let before = session.state.clone();
    if session.state.apply(&session.game, action) {
        session.undo.push_back(before);
        if session.undo.len() > UNDO_DEPTH {
            session.undo.pop_front();
        }
        session.changed();
    }
}
