use crate::protocol::{
    ClientRequest,
    PlayRequest, WatchRequest, ListRequest,
};
use http::{Uri};
use log::*;
// This needs to be included so we know which struct we return
use serde_urlencoded::de::Error;
// This *trait* needs to be included so we can construct new structs of the previous type
use serde::de::Error as SerdeError;

pub fn parse_uri(uri: Uri) -> Result<ClientRequest, Error> {
    let query: &str = match uri.query() {
        Some(s) => s,
        None => &"",
    };
    debug!("query received: {}", query);

    match uri.path() {
        "/play" => {
            let req : PlayRequest = serde_urlencoded::from_str(query)?;
            Ok(ClientRequest::Play(req))
        },
        "/watch" => {
            let req : WatchRequest = serde_urlencoded::from_str(query)?;
            Ok(ClientRequest::Watch(req))
        },
        "/list/games" => {
            let req : ListRequest = serde_urlencoded::from_str(query)?;
            Ok(ClientRequest::List(req))
        },
        other_path => Err(Error::custom(format!("Unknown path {}", other_path)))
    }
}
