use super::actix::Message;
use super::{HyParViewActor, Peer};
use bounded_set::BoundedSet;
use std::io;

#[derive(Eq, PartialEq, Debug)]
pub struct Views {
    pub active_view: BoundedSet<Peer>,
    pub passive_view: BoundedSet<Peer>,
}

impl Message for Views {
    type Result = Result<(), io::Error>;
}

impl Views {
    pub fn from_hyparview(actor: &HyParViewActor) -> Views {
        Views {
            active_view: actor.active_view.clone(),
            passive_view: actor.passive_view.clone(),
        }
    }
}
