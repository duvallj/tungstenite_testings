use crate::protocol::*;

impl From<Room> for ExternalRoom {
    fn from(r: Room) -> Self {
        ExternalRoom {
            black: r.black_name,
            white: r.white_name,
            timelimit: r.timelimit,
        }
    }
}

impl From<&Room> for ExternalRoom {
    fn from(r: &Room) -> Self {
        ExternalRoom {
            black: r.black_name.clone(),
            white: r.white_name.clone(),
            timelimit: r.timelimit,
        }
    }
}

impl PlayRequest {
    pub fn to_room(self, id: &Id) -> Room {
        Room {
            id: id.clone(),
            black_name: self.black,
            white_name: self.white,
            timelimit: self.t,
            watching: Vec::new(),
        }
    }
}

impl From<WatchRequest> for Id {
    fn from(wrq: WatchRequest) -> Self {
        wrq.watching
    }
}

impl From<&WatchRequest> for Id {
    fn from(wrq: &WatchRequest) -> Self {
        wrq.watching.clone()
    }
}
