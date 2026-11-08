//! The buzzboard server: owns the game state and serves the pages.
//!
//! Usage: `cargo run -p server [-- path/to/game.json]`
//! Without an argument it loads `games/sample.json`.
//!
//! On startup it loads and validates the game, resumes the saved state for
//! that game if there is one (`data/<name>.state.json`), and then serves the
//! frontend plus the `/ws` WebSocket on port 3000.

use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::routing::get;
use game::content::Game;
use game::state::GameState;
use server::session::{Session, Shared};
use server::ws::ws_handler;
use tokio::sync::watch::{self};
use tower_http::services::{ServeDir, ServeFile};

/// Also the port `frontend/Trunk.toml` proxies `/ws` to during development.
const PORT: u16 = 3000;

/// Paths relative to the workspace root, so `cargo run` works from any folder.
const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

#[tokio::main]
async fn main() -> ExitCode {
    // First argument, or the sample game.
    let game_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(WORKSPACE).join("games/sample.json"));

    let game = match load_game(&game_path) {
        Ok(game) => game,
        Err(message) => {
            eprintln!("Can't load {}: {message}", game_path.display());
            return ExitCode::FAILURE;
        }
    };

    // One save file per game file: games/foo.json → data/foo.state.json.
    // Can't really fail (the file was just read), but no unwrap.
    let Some(stem) = game_path.file_stem() else {
        eprintln!("Can't make a save file name from {}", game_path.display());
        return ExitCode::FAILURE;
    };

    let save_path = Path::new(WORKSPACE)
        .join("data")
        .join(format!("{}.state.json", stem.to_string_lossy()));

    // A broken save file stops the server instead of starting fresh: the
    // first change would overwrite it, and it may still be fixable by hand.
    let state = match load_state(&save_path) {
        Ok(state) => state,
        Err(message) => {
            eprintln!("Can't load saved game {}: {message}", save_path.display());
            eprintln!("Fix or delete that file to start over.");
            return ExitCode::FAILURE;
        }
    };

    if save_path.exists() {
        println!("Resuming saved game ({} players)", state.players.len());
    } else {
        println!("New game, saving to {}", save_path.display());
    }

    println!("Loaded \"{}\" ({} board(s))", game.name, game.boards.len());

    // The initial receiver is dropped; each connection subscribes its own.
    let (changes, _) = watch::channel(());

    let session: Shared = Arc::new(Mutex::new(Session {
        game: game,
        state: state,
        undo: VecDeque::new(),
        changes: changes,
        save_path: save_path,
    }));

    let app = router(session);

    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, PORT));
    // 0.0.0.0: reachable from the TV and phones, not only this laptop.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("Can't listen on port {PORT}: {err}");
            return ExitCode::FAILURE;
        }
    };

    let host = lan_ip().map_or_else(|| "localhost".to_string(), |ip| ip.to_string());
    println!("TV:   http://{host}:{PORT}/display");
    println!("Host: http://{host}:{PORT}/host");

    if let Err(err) = axum::serve(listener, app).await {
        eprintln!("Server error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Reads, parses and validates a game file.
fn load_game(path: &Path) -> Result<Game, String> {
    let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let game: Game = serde_json::from_str(&text).map_err(|err| err.to_string())?;
    game.validate()
        .map_err(|err| format!("invalid content: {err:?}"))?;
    Ok(game)
}

/// Serves the built frontend (`trunk build` output). Every path that isn't a
/// file gets `index.html`, so the frontend router handles /display and /host.
fn router(session: Shared) -> Router {
    let dist = Path::new(WORKSPACE).join("frontend/dist");
    let index = ServeFile::new(dist.join("index.html"));
    Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(dist).fallback(index))
        .with_state(session)
}

/// This machine's address on the local network, for the URLs printed at
/// startup. Connecting a UDP socket sends nothing; it only makes the OS pick
/// the outgoing interface, whose address we then read.
fn lan_ip() -> Option<std::net::IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 168, 1, 1), 80)).ok()?;
    Some(socket.local_addr().ok()?.ip())
}

/// Loads the saved state, or a fresh one if there is no save file yet.
/// The undo history is not saved, so it always starts empty.
fn load_state(path: &Path) -> Result<GameState, String> {
    if !path.exists() {
        return Ok(GameState::new());
    }

    let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let state: GameState = serde_json::from_str(&text).map_err(|err| err.to_string())?;

    Ok(state)
}
