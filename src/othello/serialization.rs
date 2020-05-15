use std::fmt;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};

use crate::othello::*;

impl Serialize for BoardStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // String implements FromIterator<char>, str does not
        let st : String = self.board
            .iter()
            .map(piece_to_char)
            .collect();
        serializer.serialize_str(st.as_str())
    }
}

struct BoardVisitor;

impl<'de> Visitor<'de> for BoardVisitor {
    type Value = BoardStruct;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an 8x8 Othello board in TJHSST notation (length 100)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value.len() != 100 {
            return Err(E::custom(format!("board string has incorrect size!")));
        }

        let pcs : Vec<Piece> = value
            .chars()
            .map(char_to_piece)
            .collect();
        if pcs.len() != 100 {
            return Err(E::custom(format!("something went wonky; board is not length 100 somewhere")));
        }
        
        let mut board : Board = [Piece::OUTER; 100];
        board.copy_from_slice(pcs.as_slice());
        Ok(BoardStruct {board: board})
    }
}

impl<'de> Deserialize<'de> for BoardStruct {
    fn deserialize<D>(deserializer: D) -> Result<BoardStruct, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(BoardVisitor)
    }
}
