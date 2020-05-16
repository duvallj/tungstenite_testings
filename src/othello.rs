use serde::{Serialize, Deserialize};
use std::slice::Iter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Piece {
    #[serde(rename = "@")]
    BLACK,
    #[serde(rename = "o")]
    WHITE,
    #[serde(rename = ".")]
    EMPTY,
    #[serde(rename = "?")]
    OUTER
}
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Player {
    #[serde(rename = "@")]
    Black,
    #[serde(rename = "o")]
    White,
    #[serde(rename = "?")]
    Unknown,
}

impl Player {
    fn opponent (&self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
            Player::Unknown => Player::Unknown,
        }
    }
}

pub const UP    : i32 = -10;
pub const DOWN  : i32 = 10;
pub const LEFT  : i32 = -1;
pub const RIGHT : i32 = 1;
pub const UP_RIGHT   : i32 = UP + RIGHT;
pub const UP_LEFT    : i32 = UP + LEFT;
pub const DOWN_RIGHT : i32 = DOWN + RIGHT;
pub const DOWN_LEFT  : i32 = DOWN + LEFT;

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    UpRight,
    UpLeft,
    DownRight,
    DownLeft,
}

const LEGAL_SPACES : [usize; 64] = [
    11, 12, 13, 14, 15, 16, 17, 18, 
    21, 22, 23, 24, 25, 26, 27, 28, 
    31, 32, 33, 34, 35, 36, 37, 38, 
    41, 42, 43, 44, 45, 46, 47, 48, 
    51, 52, 53, 54, 55, 56, 57, 58, 
    61, 62, 63, 64, 65, 66, 67, 68, 
    71, 72, 73, 74, 75, 76, 77, 78, 
    81, 82, 83, 84, 85, 86, 87, 88];

impl Direction {
    pub fn value(&self) -> i32 {
        match self {
            Direction::Up => UP,
            Direction::Down => DOWN,
            Direction::Left => LEFT,
            Direction::Right => RIGHT,
            Direction::UpRight => UP_RIGHT,
            Direction::UpLeft => UP_LEFT,
            Direction::DownRight => DOWN_RIGHT,
            Direction::DownLeft => DOWN_LEFT,
       }
    }

    pub fn iter() -> Iter<'static, Direction> {
        static DIRECTIONS : [Direction; 8] = [
            Direction::Up, Direction::UpLeft, Direction::Left, Direction::DownLeft, Direction::Down, Direction::DownRight, Direction::Right, Direction::UpRight
        ];
        DIRECTIONS.iter()
    }
}

pub type Board = [Piece; 100];

fn initial_board () -> Board {
    let mut board = [Piece::OUTER; 100];
    for x in 1..9 {
        for y in 1..9 {
            board[x+y*10] = Piece::EMPTY;
       }
    }
    board[44] = Piece::WHITE;
    board[45] = Piece::BLACK;
    board[54] = Piece::BLACK;
    board[55] = Piece::WHITE;

    board
}
// I can't believe they can't contantize the previous function :(
pub const INITIAL_BOARD : Board = 
[
Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::WHITE, Piece::BLACK, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::BLACK, Piece::WHITE, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::EMPTY, Piece::OUTER,
Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER, Piece::OUTER,
];


#[derive(Copy, Clone)]
pub struct BoardStruct {
    board: Board,
}
// whyy
impl std::fmt::Debug for BoardStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&&self.board[..], f)
    }
}

mod serialization;
mod moves;
mod conversions;
