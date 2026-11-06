//! Game content: the boards, categories and clues loaded from a game file
//! (see `games/sample.json`).
//!
//! Content is static during play. Call [`Game::validate`] once after loading;
//! the rest of the game code may assume the content is valid.

use serde::{Deserialize, Serialize};

/// A single tile on the board.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Clue {
    /// Points awarded for a correct answer (and lost for a wrong one).
    pub value: u32,
    /// Shown on /display and read out by the host.
    pub question: String,
    /// Only shown to the host.
    pub answer: String,
}

/// Position of a clue in a [`Game`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy, PartialOrd, Ord, Eq)]
pub struct ClueId {
    /// Index into [`Game::boards`].
    pub board: usize,
    /// Index into [`Board::categories`].
    pub category: usize,
    /// Index into [`Category::clues`], top (cheapest) to bottom.
    pub row: usize,
}

/// One column on a board.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub name: String,
    /// Ordered top to bottom.
    pub clues: Vec<Clue>,
}

/// One round of play: a grid of categories (columns) and clues (rows).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Board {
    pub name: String,
    /// Ordered left to right.
    pub categories: Vec<Category>,
}

/// A complete game file.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub name: String,
    /// Played in order. [`Game::validate`] requires 1 or 2.
    pub boards: Vec<Board>,
    // pub fast_money:
}

/// Why [`Game::validate`] rejected a game.
#[derive(Debug, PartialEq)]
pub enum ContentError {
    /// The game name is empty or only whitespace.
    EmptyGameName,
    /// The game must have 1 or 2 boards; `found` is how many it has.
    InvalidBoardCount { found: usize },
    /// A board has no categories.
    EmptyBoard { board: usize },
    /// A category has no clues.
    EmptyCategory { board: usize, category: usize },
    /// A category has a different number of clues than the first category
    /// on its board. `expected` is taken from that first category.
    UnevenCategory {
        board: usize,
        category: usize,
        expected: usize,
        found: usize,
    },
    /// A clue's question is empty or only whitespace.
    EmptyQuestion(ClueId),
    /// A clue's answer is empty or only whitespace.
    EmptyAnswer(ClueId),
}

impl Game {
    /// Looks up a clue by position. Returns `None` if any index is out of range.
    pub fn clue(&self, id: ClueId) -> Option<&Clue> {
        self.boards
            .get(id.board)?
            .categories
            .get(id.category)?
            .clues
            .get(id.row)
    }

    /// Checks that the content is playable. Stops at the first problem found.
    ///
    /// Checks run in order: game name, board count, then per board: has
    /// categories, and per category: has clues, same clue count as the first.
    pub fn validate(&self) -> Result<(), ContentError> {
        if self.name.trim().is_empty() {
            return Err(ContentError::EmptyGameName);
        }

        match self.boards.len() {
            1 | 2 => {}
            n => return Err(ContentError::InvalidBoardCount { found: n }),
        }

        for (b, board) in self.boards.iter().enumerate() {
            if board.categories.is_empty() {
                return Err(ContentError::EmptyBoard { board: b });
            }
            // Safe to index: we just checked `categories` is not empty.
            let expected = board.categories[0].clues.len();
            for (c, category) in board.categories.iter().enumerate() {
                if category.clues.is_empty() {
                    return Err(ContentError::EmptyCategory {
                        board: b,
                        category: c,
                    });
                }
                if category.clues.len() != expected {
                    return Err(ContentError::UnevenCategory {
                        board: b,
                        category: c,
                        expected,
                        found: category.clues.len(),
                    });
                }
                for (r, clue) in category.clues.iter().enumerate() {
                    let clue_id = ClueId {
                        board: b,
                        category: c,
                        row: r,
                    };
                    if clue.question.trim().is_empty() {
                        return Err(ContentError::EmptyQuestion(clue_id));
                    }

                    if clue.answer.trim().is_empty() {
                        return Err(ContentError::EmptyAnswer(clue_id));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Game {
        serde_json::from_str(include_str!("../../games/sample.json")).unwrap()
    }

    #[test]
    fn sample_loads() {
        let game = sample();
        assert_eq!(game.boards.len(), 2);
    }

    #[test]
    fn clue_in_range_returns_some() {
        let game: Game = sample();
        let id = ClueId {
            board: 1,
            category: 1,
            row: 1,
        };
        assert!(game.clue(id).is_some())
    }

    #[test]
    fn clue_out_of_range_returns_none() {
        let game: Game = sample();
        let valid = ClueId {
            board: 0,
            category: 0,
            row: 0,
        };

        let cases = [
            ClueId { board: 9, ..valid },
            ClueId {
                category: 9,
                ..valid
            },
            ClueId { row: 9, ..valid },
        ];

        for id in cases {
            assert_eq!(game.clue(id), None, "expected None for {id:?}");
        }
    }

    #[test]
    fn sample_is_valid() {
        assert_eq!(sample().validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_broken_content() {
        let cases: Vec<(fn(&mut Game), ContentError)> = vec![
            (|g| g.name = "   ".into(), ContentError::EmptyGameName),
            (
                |g| g.boards.clear(),
                ContentError::InvalidBoardCount { found: 0 },
            ),
            (
                |g| g.boards.push(g.boards[0].clone()),
                ContentError::InvalidBoardCount { found: 3 },
            ),
            (
                |g| g.boards[1].categories.clear(),
                ContentError::EmptyBoard { board: 1 },
            ),
            (
                |g| g.boards[0].categories[2].clues.clear(),
                ContentError::EmptyCategory {
                    board: 0,
                    category: 2,
                },
            ),
            (
                |g| {
                    g.boards[1].categories[3].clues.pop();
                },
                ContentError::UnevenCategory {
                    board: 1,
                    category: 3,
                    expected: 4,
                    found: 3,
                },
            ),
            // First category is the odd one out: it sets `expected`,
            // so the error is reported on category 1.
            (
                |g| {
                    g.boards[0].categories[0].clues.pop();
                },
                ContentError::UnevenCategory {
                    board: 0,
                    category: 1,
                    expected: 3,
                    found: 4,
                },
            ),
            (
                |g| g.boards[1].categories[4].clues[2].question = "".into(),
                ContentError::EmptyQuestion(ClueId {
                    board: 1,
                    category: 4,
                    row: 2,
                }),
            ),
            (
                |g| g.boards[0].categories[1].clues[3].answer = " \t ".into(),
                ContentError::EmptyAnswer(ClueId {
                    board: 0,
                    category: 1,
                    row: 3,
                }),
            ),
        ];

        for (break_it, expected) in cases {
            let mut game = sample();
            break_it(&mut game);
            assert_eq!(game.validate(), Err(expected), "case failed");
        }
    }
}
