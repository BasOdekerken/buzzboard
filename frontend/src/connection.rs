//! The WebSocket connection to the server, shared by every page.
//!
//! [`connect`] opens `/ws?role=…` and keeps the latest [`View`] in a signal,
//! so a page just reads `connection.view` and redraws. If the connection
//! drops, it reconnects every second; the server sends the full view again on
//! connect, so nothing else is needed to catch up.

use std::time::Duration;

use game::actions::Action;
use game::protocol::ClientMessage;
use game::view::{Role, View};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, WebSocket};

/// How long to wait before reconnecting after the connection drops.
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// A handle to the connection. Cheap to copy into event handlers.
#[derive(Clone, Copy)]
pub struct Connection {
    /// The latest view from the server; `None` until the first one arrives.
    /// Kept while reconnecting, so the page doesn't go blank.
    pub view: ReadSignal<Option<View>>,
    /// Whether the WebSocket is currently open.
    pub connected: ReadSignal<bool>,
    socket: StoredValue<Option<WebSocket>, LocalStorage>,
}

impl Connection {
    /// Sends a message to the server. Dropped silently while disconnected:
    /// the host simply taps again once the connection is back.
    pub fn send(&self, message: ClientMessage) {
        let Ok(json) = serde_json::to_string(&message) else {
            return;
        };
        self.socket.with_value(|socket| {
            if let Some(socket) = socket {
                let _ = socket.send_with_str(&json);
            }
        });
    }

    /// Shorthand for sending a game action.
    pub fn action(&self, action: Action) {
        self.send(ClientMessage::Action(action));
    }
}

/// Opens the connection for `role` and keeps it open.
pub fn connect(role: Role) -> Connection {
    let (view, set_view) = signal(None);
    let (connected, set_connected) = signal(false);
    let socket = StoredValue::new_local(None);
    open(role, set_view, set_connected, socket);
    Connection {
        view,
        connected,
        socket,
    }
}

/// `ws://<same host>/ws?role=…`, so it works both behind `trunk serve`
/// (which proxies `/ws`) and when the server serves the pages itself.
fn url(role: Role) -> String {
    let location = window().location();
    let scheme = if location.protocol().as_deref() == Ok("https:") {
        "wss"
    } else {
        "ws"
    };
    let host = location.host().unwrap_or_default();
    let role = match role {
        Role::Host => "host",
        Role::Display => "display",
    };
    format!("{scheme}://{host}/ws?role={role}")
}

fn open(
    role: Role,
    set_view: WriteSignal<Option<View>>,
    set_connected: WriteSignal<bool>,
    socket: StoredValue<Option<WebSocket>, LocalStorage>,
) {
    let reconnect = move || {
        set_timeout(
            move || open(role, set_view, set_connected, socket),
            RECONNECT_DELAY,
        )
    };

    let Ok(ws) = WebSocket::new(&url(role)) else {
        reconnect();
        return;
    };

    // The callbacks live as long as the socket; `forget` hands them to the
    // browser. One small leak per reconnect, which is fine for this app.
    let on_open = Closure::<dyn FnMut()>::new(move || set_connected.set(true));
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let Some(text) = event.data().as_string() else {
            return;
        };
        match serde_json::from_str::<View>(&text) {
            Ok(view) => set_view.set(Some(view)),
            Err(err) => leptos::logging::warn!("Unreadable view from server: {err}"),
        }
    });
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let on_close = Closure::<dyn FnMut()>::new(move || {
        set_connected.set(false);
        socket.set_value(None);
        reconnect();
    });
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    on_close.forget();

    socket.set_value(Some(ws));
}
