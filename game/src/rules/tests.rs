//! Tests for the game rules ([`GameState::apply`]). One section per action.

use super::*;
use std::collections::BTreeSet;

use crate::content::ClueId;
use crate::state::{ActiveClue, ClueStage, Phase, Player, PlayerId};

fn game() -> Game {
    serde_json::from_str(include_str!("../../../games/sample.json")).unwrap()
}

/// A fresh Setup state with the given players added, in order.
fn setup_with(names: &[&str]) -> GameState {
    let game = game();
    let mut state = GameState::new();
    for name in names {
        assert!(state.apply(
            &game,
            Action::AddPlayer {
                name: name.to_string()
            }
        ));
    }
    state
}

fn names(state: &GameState) -> Vec<&str> {
    state.players.iter().map(|p| p.name.as_str()).collect()
}

// --- GameState::new ---

#[test]
fn new_state_is_empty_setup() {
    let state = GameState::new();
    assert_eq!(state.phase, Phase::Setup);
    assert!(state.players.is_empty());
    assert!(state.used_clues.is_empty());
    assert_eq!(state.daily_double, None);
    assert_eq!(state.board_index, 0);
}

// --- AddPlayer ---

#[test]
fn add_player_starts_at_zero_with_first_id_zero() {
    let state = setup_with(&["Anna"]);
    assert_eq!(
        state.players,
        vec![Player {
            id: PlayerId(0),
            name: "Anna".into(),
            score: 0,
        }]
    );
}

#[test]
fn add_player_appends_in_order_with_increasing_ids() {
    let state = setup_with(&["Anna", "Bram", "Cas"]);
    assert_eq!(names(&state), ["Anna", "Bram", "Cas"]);
    let ids: Vec<PlayerId> = state.players.iter().map(|p| p.id).collect();
    assert_eq!(ids, [PlayerId(0), PlayerId(1), PlayerId(2)]);
}

#[test]
fn add_player_trims_name() {
    let state = setup_with(&["  Anna \t"]);
    assert_eq!(names(&state), ["Anna"]);
}

#[test]
fn add_player_ignores_blank_name() {
    let game = game();
    let mut state = setup_with(&["Anna"]);
    let before = state.clone();
    assert!(!state.apply(&game, Action::AddPlayer { name: "   ".into() }));
    assert_eq!(state, before);
}

#[test]
fn add_player_ignores_duplicate_name() {
    let game = game();
    let mut state = setup_with(&["Anna"]);
    let before = state.clone();
    assert!(!state.apply(
        &game,
        Action::AddPlayer {
            name: " Anna ".into()
        }
    ));
    assert_eq!(state, before);
}

#[test]
fn add_player_ignored_outside_setup() {
    let game = game();
    let mut state = setup_with(&["Anna"]);
    state.phase = Phase::Board;
    let before = state.clone();
    assert!(!state.apply(
        &game,
        Action::AddPlayer {
            name: "Bram".into()
        }
    ));
    assert_eq!(state, before);
}

// --- RemovePlayer ---

#[test]
fn remove_player_keeps_order_and_ids_of_others() {
    let game = game();
    let mut state = setup_with(&["Anna", "Bram", "Cas"]);
    assert!(state.apply(&game, Action::RemovePlayer(PlayerId(1))));
    assert_eq!(names(&state), ["Anna", "Cas"]);
    assert_eq!(state.players[1].id, PlayerId(2));
}

#[test]
fn new_id_is_highest_existing_plus_one() {
    let game = game();
    let mut state = setup_with(&["Anna", "Bram", "Cas"]);
    state.apply(&game, Action::RemovePlayer(PlayerId(0)));
    state.apply(
        &game,
        Action::AddPlayer {
            name: "Dirk".into(),
        },
    );
    assert_eq!(state.players.last().unwrap().id, PlayerId(3));
}

#[test]
fn remove_and_re_add_fixes_a_typo() {
    let game = game();
    let mut state = setup_with(&["Anna", "Barm"]);
    state.apply(&game, Action::RemovePlayer(PlayerId(1)));
    state.apply(
        &game,
        Action::AddPlayer {
            name: "Bram".into(),
        },
    );
    assert_eq!(names(&state), ["Anna", "Bram"]);
}

#[test]
fn remove_unknown_player_is_ignored() {
    let game = game();
    let mut state = setup_with(&["Anna"]);
    let before = state.clone();
    assert!(!state.apply(&game, Action::RemovePlayer(PlayerId(7))));
    assert_eq!(state, before);
}

#[test]
fn remove_player_ignored_outside_setup() {
    let game = game();
    let mut state = setup_with(&["Anna"]);
    state.phase = Phase::Board;
    let before = state.clone();
    assert!(!state.apply(&game, Action::RemovePlayer(PlayerId(0))));
    assert_eq!(state, before);
}

/// Asserts that `action` is ignored: returns false and changes nothing.
fn assert_ignored(state: &mut GameState, action: Action) {
    let before = state.clone();
    assert!(
        !state.apply(&game(), action.clone()),
        "expected {action:?} to be ignored"
    );
    assert_eq!(
        *state, before,
        "{action:?} changed the state but returned false"
    );
}

fn clue(board: usize, category: usize, row: usize) -> ClueId {
    ClueId {
        board,
        category,
        row,
    }
}

fn start(board: usize, daily_double: ClueId) -> Action {
    Action::StartBoard {
        board,
        daily_double,
    }
}

/// Players Anna and Bram, at the given phase.
fn in_phase(phase: Phase) -> GameState {
    let mut state = setup_with(&["Anna", "Bram"]);
    state.phase = phase;
    state
}

fn open_clue_phase() -> Phase {
    Phase::Clue(ActiveClue {
        id: clue(0, 0, 0),
        stage: ClueStage::Selected,
        answered_wrong: BTreeSet::new(),
    })
}

// --- StartBoard ---

#[test]
fn start_board_from_setup() {
    let mut state = setup_with(&["Anna"]);
    assert!(state.apply(&game(), start(0, clue(0, 3, 2))));
    assert_eq!(state.phase, Phase::Board);
    assert_eq!(state.board_index, 0);
    assert_eq!(state.daily_double, Some(clue(0, 3, 2)));
    assert!(state.used_clues.is_empty());
}

#[test]
fn start_board_can_begin_with_second_board() {
    let mut state = setup_with(&["Anna"]);
    assert!(state.apply(&game(), start(1, clue(1, 0, 0))));
    assert_eq!(state.board_index, 1);
    assert_eq!(state.daily_double, Some(clue(1, 0, 0)));
}

#[test]
fn start_board_needs_a_player() {
    let mut state = GameState::new();
    assert_ignored(&mut state, start(0, clue(0, 0, 0)));
}

#[test]
fn start_board_after_board_ended_resets_board_but_keeps_scores() {
    let mut state = in_phase(Phase::BoardEnded);
    state.board_index = 0;
    state.players[0].score = 5;
    state.players[1].score = -2;
    state.used_clues = BTreeSet::from([clue(0, 0, 0), clue(0, 1, 2)]);
    state.daily_double = Some(clue(0, 4, 3));

    assert!(state.apply(&game(), start(1, clue(1, 2, 1))));

    assert_eq!(state.phase, Phase::Board);
    assert_eq!(state.board_index, 1);
    assert!(state.used_clues.is_empty());
    assert_eq!(state.daily_double, Some(clue(1, 2, 1)));
    let scores: Vec<i32> = state.players.iter().map(|p| p.score).collect();
    assert_eq!(scores, [5, -2]);
}

#[test]
fn start_board_can_replay_a_board() {
    let mut state = in_phase(Phase::BoardEnded);
    state.board_index = 0;
    assert!(state.apply(&game(), start(0, clue(0, 1, 1))));
    assert_eq!(state.board_index, 0);
    assert_eq!(state.phase, Phase::Board);
}

#[test]
fn start_board_ignores_board_that_does_not_exist() {
    // The sample has 2 boards: 0 and 1.
    let mut state = in_phase(Phase::BoardEnded);
    assert_ignored(&mut state, start(2, clue(2, 0, 0)));
}

#[test]
fn start_board_ignores_daily_double_on_other_board() {
    let mut state = in_phase(Phase::Setup);
    assert_ignored(&mut state, start(0, clue(1, 0, 0)));
}

#[test]
fn start_board_ignores_daily_double_out_of_range() {
    // Each board has 5 categories × 4 clues.
    for dd in [clue(0, 5, 0), clue(0, 0, 4)] {
        let mut state = in_phase(Phase::Setup);
        assert_ignored(&mut state, start(0, dd));
    }
}

#[test]
fn start_board_ignored_during_a_board() {
    for phase in [Phase::Board, open_clue_phase()] {
        let mut state = in_phase(phase);
        assert_ignored(&mut state, start(1, clue(1, 0, 0)));
    }
}

// --- EndBoard ---

#[test]
fn end_board_from_board_only_changes_phase() {
    let mut state = in_phase(Phase::Board);
    state.board_index = 1;
    state.players[0].score = 3;
    state.used_clues = BTreeSet::from([clue(1, 0, 0)]);
    state.daily_double = Some(clue(1, 4, 2));
    let mut expected = state.clone();
    expected.phase = Phase::BoardEnded;

    assert!(state.apply(&game(), Action::EndBoard));
    assert_eq!(state, expected);
}

#[test]
fn end_board_ignored_outside_board() {
    for phase in [Phase::Setup, Phase::BoardEnded, open_clue_phase()] {
        let mut state = in_phase(phase);
        assert_ignored(&mut state, Action::EndBoard);
    }
}

// --- SelectClue ---

/// Players Anna and Bram, playing the given board, with a Daily Double on it.
fn on_board(board: usize) -> GameState {
    let mut state = in_phase(Phase::Board);
    state.board_index = board;
    state.daily_double = Some(clue(board, 4, 3));
    state
}

#[test]
fn select_clue_opens_it_in_selected_stage() {
    let mut state = on_board(1);
    state.used_clues = BTreeSet::from([clue(1, 0, 0)]);
    let mut expected = state.clone();
    expected.phase = Phase::Clue(ActiveClue {
        id: clue(1, 2, 3),
        stage: ClueStage::Selected,
        answered_wrong: BTreeSet::new(),
    });

    assert!(state.apply(&game(), Action::SelectClue(clue(1, 2, 3))));
    assert_eq!(state, expected);
}

#[test]
fn select_clue_daily_double_opens_like_any_other() {
    // The TV shows the animation based on `daily_double`; the state
    // itself treats the Daily Double like a normal clue.
    let mut state = on_board(0);
    assert!(state.apply(&game(), Action::SelectClue(clue(0, 4, 3))));
    assert_eq!(
        state.phase,
        Phase::Clue(ActiveClue {
            id: clue(0, 4, 3),
            stage: ClueStage::Selected,
            answered_wrong: BTreeSet::new(),
        })
    );
}

#[test]
fn select_clue_ignores_clue_on_other_board() {
    let mut state = on_board(0);
    assert_ignored(&mut state, Action::SelectClue(clue(1, 0, 0)));
}

#[test]
fn select_clue_ignores_clue_out_of_range() {
    for id in [clue(0, 5, 0), clue(0, 0, 4)] {
        let mut state = on_board(0);
        assert_ignored(&mut state, Action::SelectClue(id));
    }
}

#[test]
fn select_clue_ignores_used_clue() {
    let mut state = on_board(0);
    state.used_clues = BTreeSet::from([clue(0, 2, 1)]);
    assert_ignored(&mut state, Action::SelectClue(clue(0, 2, 1)));
}

#[test]
fn select_clue_ignored_outside_board() {
    for phase in [Phase::Setup, Phase::BoardEnded, open_clue_phase()] {
        let mut state = in_phase(phase);
        assert_ignored(&mut state, Action::SelectClue(clue(0, 1, 1)));
    }
}

// --- Flip ---

/// Players Anna and Bram on board 0, with clue (0, 1, 2) open at `stage`.
fn clue_open_at(stage: ClueStage) -> GameState {
    let mut state = on_board(0);
    state.phase = Phase::Clue(ActiveClue {
        id: clue(0, 1, 2),
        stage,
        answered_wrong: BTreeSet::new(),
    });
    state
}

#[test]
fn flip_opens_buzzers_with_empty_queue() {
    let mut state = clue_open_at(ClueStage::Selected);
    let mut expected = state.clone();
    expected.phase = Phase::Clue(ActiveClue {
        id: clue(0, 1, 2),
        stage: ClueStage::Open { buzzes: vec![] },
        answered_wrong: BTreeSet::new(),
    });

    assert!(state.apply(&game(), Action::Flip));
    assert_eq!(state, expected);
}

#[test]
fn flip_works_for_daily_double() {
    let mut state = on_board(0);
    assert!(state.apply(&game(), Action::SelectClue(clue(0, 4, 3))));
    assert!(state.apply(&game(), Action::Flip));
    assert!(matches!(
        state.phase,
        Phase::Clue(ActiveClue {
            stage: ClueStage::Open { .. },
            ..
        })
    ));
}

#[test]
fn flip_ignored_when_already_flipped() {
    let open = ClueStage::Open {
        buzzes: vec![PlayerId(1)],
    };
    for stage in [open, ClueStage::Revealed] {
        let mut state = clue_open_at(stage);
        assert_ignored(&mut state, Action::Flip);
    }
}

#[test]
fn flip_ignored_without_open_clue() {
    for phase in [Phase::Setup, Phase::Board, Phase::BoardEnded] {
        let mut state = in_phase(phase);
        assert_ignored(&mut state, Action::Flip);
    }
}

// --- Buzz ---

/// Players Anna (0) and Bram (1) plus Cas (2), on board 0, with clue (0, 1, 2)
/// flipped and the given buzz queue and players who already answered wrong.
fn buzzing(buzzes: &[u32], answered_wrong: &[u32]) -> GameState {
    let mut state = clue_open_at(ClueStage::Open {
        buzzes: buzzes.iter().map(|&n| PlayerId(n)).collect(),
    });
    state.players.push(Player {
        id: PlayerId(2),
        name: "Cas".into(),
        score: 0,
    });
    if let Phase::Clue(active) = &mut state.phase {
        active.answered_wrong = answered_wrong.iter().map(|&n| PlayerId(n)).collect();
    }
    state
}

#[test]
fn buzz_joins_empty_queue() {
    let mut state = buzzing(&[], &[]);
    assert!(state.apply(&game(), Action::Buzz(PlayerId(1))));
    assert_eq!(state, buzzing(&[1], &[]));
}

#[test]
fn buzz_joins_end_of_queue_while_someone_answers() {
    let mut state = buzzing(&[2], &[]);
    assert!(state.apply(&game(), Action::Buzz(PlayerId(0))));
    assert_eq!(state, buzzing(&[2, 0], &[]));
}

#[test]
fn buzz_ignored_when_already_in_queue() {
    for buzzes in [&[1][..], &[0, 1]] {
        let mut state = buzzing(buzzes, &[]);
        assert_ignored(&mut state, Action::Buzz(PlayerId(1)));
    }
}

#[test]
fn buzz_ignored_after_answering_wrong() {
    let mut state = buzzing(&[0], &[1]);
    assert_ignored(&mut state, Action::Buzz(PlayerId(1)));
}

#[test]
fn buzz_ignored_for_unknown_player() {
    let mut state = buzzing(&[], &[]);
    assert_ignored(&mut state, Action::Buzz(PlayerId(9)));
}

#[test]
fn buzz_ignored_before_flip_and_after_reveal() {
    for stage in [ClueStage::Selected, ClueStage::Revealed] {
        let mut state = clue_open_at(stage);
        assert_ignored(&mut state, Action::Buzz(PlayerId(0)));
    }
}

#[test]
fn buzz_ignored_without_open_clue() {
    for phase in [Phase::Setup, Phase::Board, Phase::BoardEnded] {
        let mut state = in_phase(phase);
        assert_ignored(&mut state, Action::Buzz(PlayerId(0)));
    }
}

// --- Correct / Wrong ---

// Clue (0, 1, 2) in the sample has value 3; the Daily Double is (0, 4, 3).

fn scores(state: &GameState) -> Vec<i32> {
    state.players.iter().map(|p| p.score).collect()
}

fn stage(state: &GameState) -> &ClueStage {
    match &state.phase {
        Phase::Clue(active) => &active.stage,
        other => panic!("expected an open clue, got {other:?}"),
    }
}

fn answered_wrong(state: &GameState) -> Vec<PlayerId> {
    match &state.phase {
        Phase::Clue(active) => active.answered_wrong.iter().copied().collect(),
        other => panic!("expected an open clue, got {other:?}"),
    }
}

/// Like `buzzing`, but with the Daily Double (0, 4, 3) open, value 4.
fn buzzing_daily_double(buzzes: &[u32]) -> GameState {
    let mut state = buzzing(buzzes, &[]);
    if let Phase::Clue(active) = &mut state.phase {
        active.id = clue(0, 4, 3);
    }
    state
}

#[test]
fn sample_values_used_by_these_tests() {
    let game = game();
    assert_eq!(game.clue(clue(0, 1, 2)).unwrap().value, 3);
    assert_eq!(game.clue(clue(0, 4, 3)).unwrap().value, 4);
}

#[test]
fn correct_adds_value_and_reveals() {
    let mut state = buzzing(&[1, 2], &[]);
    assert!(state.apply(&game(), Action::Correct(PlayerId(1))));
    assert_eq!(scores(&state), [0, 3, 0]);
    assert_eq!(stage(&state), &ClueStage::Revealed);
}

#[test]
fn correct_on_daily_double_adds_double() {
    let mut state = buzzing_daily_double(&[0]);
    assert!(state.apply(&game(), Action::Correct(PlayerId(0))));
    assert_eq!(scores(&state), [8, 0, 0]);
}

#[test]
fn correct_after_a_wrong_answer() {
    let mut state = buzzing(&[0, 1], &[]);
    assert!(state.apply(&game(), Action::Wrong(PlayerId(0))));
    assert!(state.apply(&game(), Action::Correct(PlayerId(1))));
    assert_eq!(scores(&state), [-3, 3, 0]);
    assert_eq!(stage(&state), &ClueStage::Revealed);
}

#[test]
fn wrong_subtracts_value_locks_out_and_next_answers() {
    let mut state = buzzing(&[1, 2], &[]);
    assert!(state.apply(&game(), Action::Wrong(PlayerId(1))));
    assert_eq!(scores(&state), [0, -3, 0]);
    assert_eq!(
        stage(&state),
        &ClueStage::Open {
            buzzes: vec![PlayerId(2)]
        }
    );
    assert_eq!(answered_wrong(&state), [PlayerId(1)]);
}

#[test]
fn wrong_on_daily_double_subtracts_double() {
    let mut state = buzzing_daily_double(&[2]);
    assert!(state.apply(&game(), Action::Wrong(PlayerId(2))));
    assert_eq!(scores(&state), [0, 0, -8]);
}

#[test]
fn wrong_with_empty_queue_stays_open_so_others_can_buzz() {
    let mut state = buzzing(&[0], &[]);
    assert!(state.apply(&game(), Action::Wrong(PlayerId(0))));
    assert_eq!(stage(&state), &ClueStage::Open { buzzes: vec![] });
    assert!(state.apply(&game(), Action::Buzz(PlayerId(1))));
}

#[test]
fn wrong_does_not_reveal_when_everyone_answered_wrong() {
    let mut state = buzzing(&[2], &[0, 1]);
    assert!(state.apply(&game(), Action::Wrong(PlayerId(2))));
    assert_eq!(stage(&state), &ClueStage::Open { buzzes: vec![] });
    assert_eq!(
        answered_wrong(&state),
        [PlayerId(0), PlayerId(1), PlayerId(2)]
    );
}

#[test]
fn wrong_twice_by_double_tap_only_counts_once() {
    let mut state = buzzing(&[0, 1], &[]);
    assert!(state.apply(&game(), Action::Wrong(PlayerId(0))));
    assert_ignored(&mut state, Action::Wrong(PlayerId(0)));
    assert_eq!(scores(&state), [-3, 0, 0]);
}

#[test]
fn judging_ignored_unless_player_is_first_in_queue() {
    for action in [Action::Correct(PlayerId(1)), Action::Wrong(PlayerId(1))] {
        // Bram is second, not first.
        let mut state = buzzing(&[0, 1], &[]);
        assert_ignored(&mut state, action.clone());
        // Nobody buzzed.
        let mut state = buzzing(&[], &[]);
        assert_ignored(&mut state, action);
    }
}

#[test]
fn judging_ignored_outside_open() {
    for action in [Action::Correct(PlayerId(0)), Action::Wrong(PlayerId(0))] {
        for stage in [ClueStage::Selected, ClueStage::Revealed] {
            let mut state = clue_open_at(stage);
            assert_ignored(&mut state, action.clone());
        }
        for phase in [Phase::Setup, Phase::Board, Phase::BoardEnded] {
            let mut state = in_phase(phase);
            assert_ignored(&mut state, action.clone());
        }
    }
}

// --- NobodyGotIt ---

#[test]
fn nobody_got_it_reveals_without_score_change() {
    for (buzzes, wrong) in [(&[][..], &[][..]), (&[], &[0, 1]), (&[2], &[0])] {
        let mut state = buzzing(buzzes, wrong);
        let before = scores(&state);
        assert!(state.apply(&game(), Action::NobodyGotIt));
        assert_eq!(stage(&state), &ClueStage::Revealed);
        assert_eq!(scores(&state), before);
    }
}

#[test]
fn nobody_got_it_ignored_outside_open() {
    for stage in [ClueStage::Selected, ClueStage::Revealed] {
        let mut state = clue_open_at(stage);
        assert_ignored(&mut state, Action::NobodyGotIt);
    }
    for phase in [Phase::Setup, Phase::Board, Phase::BoardEnded] {
        let mut state = in_phase(phase);
        assert_ignored(&mut state, Action::NobodyGotIt);
    }
}

// --- CloseClue ---

#[test]
fn close_clue_marks_used_and_returns_to_board() {
    let mut state = clue_open_at(ClueStage::Revealed);
    state.used_clues = BTreeSet::from([clue(0, 0, 0)]);
    let mut expected = state.clone();
    expected.phase = Phase::Board;
    expected.used_clues.insert(clue(0, 1, 2));

    assert!(state.apply(&game(), Action::CloseClue));
    assert_eq!(state, expected);
}

#[test]
fn closed_clue_can_not_be_selected_again() {
    let mut state = clue_open_at(ClueStage::Revealed);
    assert!(state.apply(&game(), Action::CloseClue));
    assert_ignored(&mut state, Action::SelectClue(clue(0, 1, 2)));
}

#[test]
fn close_clue_ignored_before_reveal() {
    let open = ClueStage::Open {
        buzzes: vec![PlayerId(0)],
    };
    for stage in [ClueStage::Selected, open] {
        let mut state = clue_open_at(stage);
        assert_ignored(&mut state, Action::CloseClue);
    }
    for phase in [Phase::Setup, Phase::Board, Phase::BoardEnded] {
        let mut state = in_phase(phase);
        assert_ignored(&mut state, Action::CloseClue);
    }
}

// --- AdjustScore ---

fn adjust(player_id: u32, delta: i32) -> Action {
    Action::AdjustScore {
        player_id: PlayerId(player_id),
        delta,
    }
}

#[test]
fn adjust_score_adds_and_subtracts() {
    let mut state = in_phase(Phase::Board);
    assert!(state.apply(&game(), adjust(1, 4)));
    assert!(state.apply(&game(), adjust(0, -2)));
    assert!(state.apply(&game(), adjust(1, -1)));
    assert_eq!(scores(&state), [-2, 3]);
}

#[test]
fn adjust_score_works_in_every_phase() {
    let phases = [
        Phase::Setup,
        Phase::Board,
        Phase::BoardEnded,
        open_clue_phase(),
    ];
    for phase in phases {
        let mut state = in_phase(phase);
        assert!(state.apply(&game(), adjust(0, 1)));
        assert_eq!(scores(&state), [1, 0]);
    }
}

#[test]
fn adjust_score_leaves_the_clue_alone() {
    let mut state = buzzing(&[1], &[0]);
    let mut expected = state.clone();
    expected.players[2].score = 5;
    assert!(state.apply(&game(), adjust(2, 5)));
    assert_eq!(state, expected);
}

#[test]
fn adjust_score_ignores_unknown_player() {
    let mut state = in_phase(Phase::Board);
    assert_ignored(&mut state, adjust(9, 1));
}

#[test]
fn adjust_score_by_zero_is_ignored() {
    // Nothing changes, so it must not create an undo step.
    let mut state = in_phase(Phase::Board);
    assert_ignored(&mut state, adjust(0, 0));
}
