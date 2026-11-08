//! Messages from clients to the server, over the `/ws` WebSocket.
//!
//! Shared by the server (which reads them) and the frontend (which sends
//! them), so both sides always agree on the format. The other direction,
//! server to client, is a [`View`](crate::view::View).

use serde::{Deserialize, Serialize};

use crate::actions::Action;

/// What the host phone can send, as JSON:
/// `{"Action": {"AddPlayer": {"name": "Anna"}}}`, `"Undo"` or
/// `{"StartBoard": {"board": 0}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ClientMessage {
    /// A game action, applied through the rules.
    Action(Action),
    /// Restore the state from before the last accepted change.
    Undo,
    /// Start this board; the server picks the Daily Double at random
    /// (see `Game::daily_double_candidates`).
    StartBoard { board: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The JSON format is a contract with the frontend; pin it down.
    #[test]
    fn json_format() {
        let cases = [
            (
                ClientMessage::Action(Action::AddPlayer {
                    name: "Anna".into(),
                }),
                r#"{"Action":{"AddPlayer":{"name":"Anna"}}}"#,
            ),
            (ClientMessage::Action(Action::Flip), r#"{"Action":"Flip"}"#),
            (ClientMessage::Undo, r#""Undo""#),
            (
                ClientMessage::StartBoard { board: 1 },
                r#"{"StartBoard":{"board":1}}"#,
            ),
        ];
        for (message, json) in cases {
            assert_eq!(serde_json::to_string(&message).unwrap(), json);
            assert_eq!(
                serde_json::from_str::<ClientMessage>(json).unwrap(),
                message
            );
        }
    }
}
