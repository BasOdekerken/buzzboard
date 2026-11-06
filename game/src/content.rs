use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Clue {
    pub value: u32,
    pub question: String,
    pub answer: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub struct ClueId {
    pub board: usize,
    pub category: usize,
    pub row: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub name: String,
    pub clues: Vec<Clue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Board {
    pub name: String,
    pub categories: Vec<Category>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub name: String,
    pub boards: Vec<Board>,
    // pub fast_money:
}

impl Game {
    pub fn clue(&self, id: ClueId) -> Option<&Clue> {
        self.boards
            .get(id.board)?
            .categories
            .get(id.category)?
            .clues
            .get(id.row)
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
            row: 1
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
            ClueId { board: 9, ..valid},
            ClueId { category: 9, ..valid},
            ClueId { row: 9, ..valid}
        ];

        for id in cases {
            assert_eq!(game.clue(id), None, "expected None for {id:?}");
        }
    }
}
