use crate::othello::*;

impl From<&Piece> for Player {
    fn from (pc: &Piece) -> Player {
        match pc {
            Piece::BLACK => Player::Black,
            Piece::WHITE => Player::White,
            Piece::EMPTY => Player::Unknown,
            Piece::OUTER => Player::Unknown,
        }
    }
}

impl From<Piece> for Player {
    fn from (pc: Piece) -> Player {
        match pc {
            Piece::BLACK => Player::Black,
            Piece::WHITE => Player::White,
            Piece::EMPTY => Player::Unknown,
            Piece::OUTER => Player::Unknown,
        }
    }
}

impl From<&Piece> for char {
    fn from (pc: &Piece) -> char {
        match pc {
            Piece::BLACK => '@',
            Piece::WHITE => 'o',
            Piece::EMPTY => '.',
            Piece::OUTER => '?',
        }
    }
}

impl From<Piece> for char {
    fn from (pc: Piece) -> char {
        match pc {
            Piece::BLACK => '@',
            Piece::WHITE => 'o',
            Piece::EMPTY => '.',
            Piece::OUTER => '?',
        }
    }
}

impl From<char> for Piece {
    fn from (c: char) -> Piece {
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
}

impl From<&Player> for Piece {
    fn from (pl: &Player) -> Piece {
        match pl {
            Player::Black => Piece::BLACK,
            Player::White => Piece::WHITE,
            Player::Unknown => Piece::OUTER,
        }
    }
}

impl From<Player> for Piece {
    fn from (pl: Player) -> Piece {
        match pl {
            Player::Black => Piece::BLACK,
            Player::White => Piece::WHITE,
            Player::Unknown => Piece::OUTER,
        }
    }
}
