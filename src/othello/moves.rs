use crate::othello::*;

type Loc = usize;
type Bd = BoardStruct;

#[derive(Debug)]
pub struct IllegalMoveError {
    player: Player,
    square: Loc,
    board: Bd,
}

// Taken from https://stackoverflow.com/a/54035801
fn add (u: usize, i: i32) -> usize {
    if i.is_negative() {
        u - i.wrapping_abs() as u32 as usize
    } else {
        u + i as usize
    }
}

fn find_bracket(square: &Loc, player: &Player, board: &Bd, direction: &Direction) -> Option<Loc> {
    let dir = direction.value();
    let board = board.board;
    
    let mut bracket : Loc = add(square.clone(), dir);
    
    if board[bracket] == player.into() {
        return None;
    }

    let opp : Piece = player.opponent().into();
    while board[bracket] == opp {
        bracket = add(bracket, dir);
    }
    
    if board[bracket] == player.into() {
        return Some(bracket);
    } else {
        return None;
    }
}

pub fn is_legal(square: &Loc, player: &Player, board: &Bd) -> bool {
    if board.board[square.clone()] != Piece::EMPTY {
        return false;
    }

    Direction::iter().any(
        |d: &Direction| find_bracket(square, player, board, d).is_some()
    )
}

fn make_flips(square: &Loc, player: &Player, board: &mut Bd, direction: &Direction) -> () {
    let bracket = find_bracket(square, player, board, direction);
    let mut board = board.board;
    let dir = direction.value();

    match bracket {
        None => (),
        Some(endpoint) => {
            let mut flipping = add(square.clone(), dir);
            while flipping != endpoint {
                board[flipping] = player.into();
                flipping = add(flipping, dir);
            }
        }
    }
}

pub fn make_move(square: &Loc, player: &Player, board: &mut Bd) -> Result<(), IllegalMoveError> {
    if !is_legal(square, player, board) {
        return Err(IllegalMoveError {
            square: square.clone(),
            player: player.clone(),
            board: board.clone()
        });
    }
    board.board[square.clone()] = player.into();
    for d in Direction::iter() {
        make_flips(square, player, board, d);
    }
    Ok(())
}

pub fn legal_moves (player: &Player, board: &Bd) -> Vec<Loc> {
    LEGAL_SPACES
        .iter()
        .filter_map(|sq: &Loc| {
            if is_legal(sq, player, board) {
                Some(*sq)
            } else {
                None
            }
        })
        .collect()
}

pub fn any_legal_moves (player: &Player, board: &Bd) -> bool {
    LEGAL_SPACES
        .iter()
        .any(|sq: &Loc| {
            is_legal(sq, player, board)
        })
}

pub fn next_player (board: &Bd, prev_player: &Player) -> Option<Player> {
    let opp = prev_player.opponent();
    if any_legal_moves(&opp, board) {
        return Some(opp);
    } else if any_legal_moves(prev_player, board) {
        return Some(prev_player.clone());
    } else {
        return None
    }
}

pub fn score (player: &Player, board: &Bd) -> i32 {
    let mine : Piece = player.into();
    let theirs : Piece = player.opponent().into();
    board.board
        .iter()
        .map(|sq: &Piece| {
            if sq == &mine {
                1
            } else if sq == &theirs {
                -1
            } else {
                0
            }
        })
        .fold(0, |acc, x| acc + x)
}

pub fn winner (board: &Bd) -> Player {
    let diff_black = score(&Player::Black, board);
    if diff_black > 0 {
        Player::Black
    } else if diff_black < 0 {
        Player::White
    } else {
        Player::Unknown
    }
}

pub fn is_game_over(player: &Player, board: &Bd) -> bool {
    !any_legal_moves(player, board) && !any_legal_moves(&player.opponent(), board)
}
