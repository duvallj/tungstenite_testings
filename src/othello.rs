use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
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
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Player {
    #[serde(rename = "@")]
    Black,
    #[serde(rename = "o")]
    White,
    #[serde(rename = "?")]
    Unknown,
}

pub fn piece_to_player (pc: &Piece) -> Player {
    match pc {
        Piece::BLACK => Player::Black,
        Piece::WHITE => Player::White,
        Piece::EMPTY => Player::Unknown,
        Piece::OUTER => Player::Unknown,
    }
}

pub fn piece_to_char (pc: &Piece) -> char {
    match pc {
        Piece::BLACK => '@',
        Piece::WHITE => 'o',
        Piece::EMPTY => '.',
        Piece::OUTER => '?',
    }
}

pub fn char_to_piece (c: char) -> Piece {
    match c {
        '@' => Piece::BLACK,
        'o' => Piece::WHITE,
        '.' => Piece::EMPTY,
        '?' => Piece::OUTER,
        // Fail "gracefully," i.e. undetectably and this will most definitely
        // come to haunt me later oops
        _ => Piece::OUTER,
    }
}

pub fn player_to_piece (pl: &Player) -> Piece {
    match pl {
        Player::Black => Piece::BLACK,
        Player::White => Piece::WHITE,
        Player::Unknown => Piece::OUTER,
    }
}

pub fn opponent (pl: &Player) -> Player {
    match pl {
        Player::Black => Player::White,
        Player::White => Player::Black,
        Player::Unknown => Player::Unknown,
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
pub const DIRECTIONS : [i32; 8] = 
    [UP, UP_RIGHT, RIGHT, DOWN_RIGHT, DOWN, DOWN_LEFT, LEFT, UP_LEFT];

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

mod serialization;
mod moves;
