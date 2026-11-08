//! Tests for [`view_for`]: what each role may see.
//!
//! The most important property: the TV never receives an answer before the
//! clue is revealed, and never learns where the Daily Double is.

use super::*;
use crate::actions::Action;
use crate::content::Game;
use crate::state::GameState;

fn game() -> Game {
    serde_json::from_str(include_str!("../../../games/sample.json")).unwrap()
}

fn clue(board: usize, category: usize, row: usize) -> ClueId {
    ClueId {
        board,
        category,
        row,
    }
}

// Sample facts used below:
// - board 0 is "Board 1"; its categories are Animals, Colours, Food, Music, Weather
// - (0, 1, 2) is Colours, value 3: "Banana" / "Yellow"
// - (0, 4, 3) is Weather, value 4: "Thunder" / "Lightning" (used as the Daily Double)
const DAILY_DOUBLE: ClueId = ClueId {
    board: 0,
    category: 4,
    row: 3,
};

/// Applies actions in order, asserting each one is accepted.
fn play(actions: &[Action]) -> GameState {
    let game = game();
    let mut state = GameState::new();
    for action in actions {
        assert!(state.apply(&game, action.clone()), "{action:?} was ignored");
    }
    state
}

fn players() -> Vec<Action> {
    vec![
        Action::AddPlayer {
            name: "Anna".into(),
        },
        Action::AddPlayer {
            name: "Bram".into(),
        },
    ]
}

/// Anna and Bram, board 0 started with the Daily Double at (0, 4, 3).
fn on_board() -> Vec<Action> {
    let mut actions = players();
    actions.push(Action::StartBoard {
        board: 0,
        daily_double: DAILY_DOUBLE,
    });
    actions
}

/// `on_board` plus `more`.
fn on_board_then(more: &[Action]) -> GameState {
    let mut actions = on_board();
    actions.extend_from_slice(more);
    play(&actions)
}

fn view(state: &GameState, role: Role) -> View {
    view_for(&game(), state, role)
}

fn board_of(view: &View) -> &BoardView {
    match &view.phase {
        ViewPhase::Board(board) | ViewPhase::Clue { board, .. } => board,
        other => panic!("expected a board, got {other:?}"),
    }
}

fn clue_of(view: &View) -> &ClueView {
    match &view.phase {
        ViewPhase::Clue { clue, .. } => clue,
        other => panic!("expected an open clue, got {other:?}"),
    }
}

fn tile(view: &View, category: usize, row: usize) -> &TileView {
    &board_of(view).categories[category].tiles[row]
}

// --- Setup / BoardEnded ---

#[test]
fn setup_shows_name_and_players() {
    let state = play(&players());
    for role in [Role::Host, Role::Display] {
        let view = view(&state, role);
        assert_eq!(view.game_name, "Game night");
        assert_eq!(view.board_names, ["Board 1", "Board 2"]);
        assert_eq!(view.players, state.players);
        assert_eq!(view.phase, ViewPhase::Setup);
    }
}

#[test]
fn board_ended_has_no_board() {
    let state = on_board_then(&[Action::EndBoard]);
    for role in [Role::Host, Role::Display] {
        assert_eq!(view(&state, role).phase, ViewPhase::BoardEnded);
    }
}

#[test]
fn scores_are_shown_to_everyone() {
    let state = on_board_then(&[Action::AdjustScore {
        player_id: PlayerId(1),
        delta: 5,
    }]);
    for role in [Role::Host, Role::Display] {
        let scores: Vec<i32> = view(&state, role).players.iter().map(|p| p.score).collect();
        assert_eq!(scores, [0, 5]);
    }
}

// --- Board ---

#[test]
fn board_lists_categories_and_tiles_in_order() {
    let state = play(&on_board());
    let view = view(&state, Role::Display);
    let board = board_of(&view);

    assert_eq!(board.name, "Board 1");
    let names: Vec<&str> = board.categories.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Animals", "Colours", "Food", "Music", "Weather"]);

    for (c, category) in board.categories.iter().enumerate() {
        let values: Vec<u32> = category.tiles.iter().map(|t| t.value).collect();
        assert_eq!(values, [1, 2, 3, 4], "values of category {c}");
        for (r, tile) in category.tiles.iter().enumerate() {
            assert_eq!(tile.id, clue(0, c, r));
            assert!(!tile.used);
        }
    }
}

#[test]
fn board_shows_the_board_being_played() {
    let state = play(&[
        players()[0].clone(),
        Action::StartBoard {
            board: 1,
            daily_double: clue(1, 0, 0),
        },
    ]);
    let view = view(&state, Role::Display);
    assert_eq!(board_of(&view).name, "Board 2");
    assert_eq!(tile(&view, 2, 1).id, clue(1, 2, 1));
}

#[test]
fn played_clues_are_marked_used() {
    let state = on_board_then(&[
        Action::SelectClue(clue(0, 1, 2)),
        Action::Flip,
        Action::NobodyGotIt,
        Action::CloseClue,
    ]);
    for role in [Role::Host, Role::Display] {
        let view = view(&state, role);
        for (c, category) in board_of(&view).categories.iter().enumerate() {
            for (r, tile) in category.tiles.iter().enumerate() {
                assert_eq!(tile.used, (c, r) == (1, 2), "tile ({c}, {r})");
            }
        }
    }
}

#[test]
fn host_sees_where_the_daily_double_is() {
    let state = play(&on_board());
    let view = view(&state, Role::Host);
    for (c, category) in board_of(&view).categories.iter().enumerate() {
        for (r, tile) in category.tiles.iter().enumerate() {
            assert_eq!(tile.daily_double, (c, r) == (4, 3), "tile ({c}, {r})");
        }
    }
}

#[test]
fn display_never_sees_where_the_daily_double_is() {
    let states = [
        play(&on_board()),
        on_board_then(&[Action::SelectClue(clue(0, 1, 2))]),
        on_board_then(&[Action::SelectClue(DAILY_DOUBLE)]),
    ];
    for state in states {
        let view = view(&state, Role::Display);
        let flagged = board_of(&view)
            .categories
            .iter()
            .flat_map(|c| &c.tiles)
            .any(|t| t.daily_double);
        assert!(!flagged, "board tile flagged as Daily Double for Display");
    }
}

// --- Open clue ---

#[test]
fn open_clue_keeps_the_board_behind_it() {
    let state = on_board_then(&[Action::SelectClue(clue(0, 1, 2))]);
    let view = view(&state, Role::Display);
    assert_eq!(board_of(&view).name, "Board 1");
}

#[test]
fn clue_view_shows_category_value_and_stage() {
    let state = on_board_then(&[Action::SelectClue(clue(0, 1, 2))]);
    for role in [Role::Host, Role::Display] {
        let view = view(&state, role);
        let clue = clue_of(&view);
        assert_eq!(clue.category, "Colours");
        assert_eq!(clue.value, 3);
        assert!(!clue.daily_double);
        assert_eq!(clue.stage, ClueStage::Selected);
        assert!(clue.answered_wrong.is_empty());
    }
}

#[test]
fn daily_double_is_announced_to_everyone_once_selected() {
    let state = on_board_then(&[Action::SelectClue(DAILY_DOUBLE)]);
    for role in [Role::Host, Role::Display] {
        let view = view(&state, role);
        let clue = clue_of(&view);
        assert!(clue.daily_double);
        assert_eq!(clue.value, 4, "value is the normal value, not doubled");
    }
}

#[test]
fn clue_view_shows_buzz_queue_and_wrong_answers() {
    let state = on_board_then(&[
        Action::SelectClue(clue(0, 1, 2)),
        Action::Flip,
        Action::Buzz(PlayerId(0)),
        Action::Buzz(PlayerId(1)),
        Action::Wrong(PlayerId(0)),
    ]);
    for role in [Role::Host, Role::Display] {
        let view = view(&state, role);
        let clue = clue_of(&view);
        assert_eq!(
            clue.stage,
            ClueStage::Open {
                buzzes: vec![PlayerId(1)]
            }
        );
        assert_eq!(clue.answered_wrong, BTreeSet::from([PlayerId(0)]));
    }
}

// --- Question and answer per role and stage ---

/// The clue (0, 1, 2) at each stage, in order.
fn each_stage() -> Vec<(&'static str, GameState)> {
    let select = Action::SelectClue(clue(0, 1, 2));
    vec![
        ("Selected", on_board_then(std::slice::from_ref(&select))),
        ("Open", on_board_then(&[select.clone(), Action::Flip])),
        (
            "Revealed",
            on_board_then(&[select, Action::Flip, Action::NobodyGotIt]),
        ),
    ]
}

#[test]
fn host_always_sees_question_and_answer() {
    for (stage, state) in each_stage() {
        let view = view(&state, Role::Host);
        let clue = clue_of(&view);
        assert_eq!(clue.question.as_deref(), Some("Banana"), "at {stage}");
        assert_eq!(clue.answer.as_deref(), Some("Yellow"), "at {stage}");
    }
}

#[test]
fn display_sees_question_from_flip_and_answer_only_when_revealed() {
    let expected = [
        ("Selected", None, None),
        ("Open", Some("Banana"), None),
        ("Revealed", Some("Banana"), Some("Yellow")),
    ];
    for ((stage, state), (name, question, answer)) in each_stage().into_iter().zip(expected) {
        assert_eq!(stage, name);
        let view = view(&state, Role::Display);
        let clue = clue_of(&view);
        assert_eq!(clue.question.as_deref(), question, "question at {stage}");
        assert_eq!(clue.answer.as_deref(), answer, "answer at {stage}");
    }
}

/// The strongest check: serialize the Display view exactly as it would go
/// over the WebSocket, and make sure no answer text appears in it, except
/// the answer of a clue that has been revealed.
#[test]
fn display_json_never_contains_an_unrevealed_answer() {
    let game = game();
    let all_answers: Vec<&str> = game
        .boards
        .iter()
        .flat_map(|b| &b.categories)
        .flat_map(|c| &c.clues)
        .map(|clue| clue.answer.as_str())
        .collect();

    let select = Action::SelectClue(clue(0, 1, 2));
    let moments = [
        ("setup", play(&players()), None),
        ("board", play(&on_board()), None),
        (
            "selected",
            on_board_then(std::slice::from_ref(&select)),
            None,
        ),
        ("open", on_board_then(&[select.clone(), Action::Flip]), None),
        (
            "after a wrong answer",
            on_board_then(&[
                select.clone(),
                Action::Flip,
                Action::Buzz(PlayerId(0)),
                Action::Wrong(PlayerId(0)),
            ]),
            None,
        ),
        (
            "revealed",
            on_board_then(&[select.clone(), Action::Flip, Action::NobodyGotIt]),
            Some("Yellow"),
        ),
        (
            "back on the board",
            on_board_then(&[select, Action::Flip, Action::NobodyGotIt, Action::CloseClue]),
            None,
        ),
        (
            "daily double selected",
            on_board_then(&[Action::SelectClue(DAILY_DOUBLE)]),
            None,
        ),
    ];

    for (moment, state, revealed) in moments {
        let json = serde_json::to_string(&view_for(&game, &state, Role::Display)).unwrap();
        for answer in &all_answers {
            let quoted = format!("\"{answer}\"");
            let allowed = revealed == Some(*answer);
            assert!(
                allowed || !json.contains(&quoted),
                "Display JSON contains answer {answer:?} at moment '{moment}'"
            );
        }
    }
}
