use super::actix::Message;
use super::{Peer, ViewsRecipient};
use std::collections::HashSet;
use std::fmt;
use std::io;

#[derive(Eq, PartialEq, Clone)]
pub enum HpvMsg {
    Inspect(ViewsRecipient),
    InitiateJoin(Peer),
    Join(Peer),
    ForwardJoin {
        joining: Peer,
        forwarder: Peer,
        ttl: usize,
    },
    Neighbour {
        peer: Peer,
        prio: bool,
    },
    NeighbourReply {
        peer: Peer,
        accepted: bool,
    },
    Shuffle {
        id: u32,
        origin: Peer,
        exchange: HashSet<Peer>,
        ttl: usize,
    },
    ShuffleReply(u32, HashSet<Peer>),
    Disconnect(Peer),
}

impl fmt::Debug for HpvMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HpvMsg::Inspect(_) => write!(f, "Inspect"),
            HpvMsg::InitiateJoin(p) => write!(f, "InitiateJoin({})", p),
            HpvMsg::Join(p) => write!(f, "Join({})", p),
            // FIXME: Somehow cannot be destructured without a fmt macro error...?
            HpvMsg::ForwardJoin { .. } => write!(f, "ForwardJoin()"),
            // FIXME: Somehow cannot be destructured without a fmt macro error...?
            HpvMsg::Neighbour { .. } => write!(f, "Neighbour()"),
            HpvMsg::NeighbourReply { .. } => write!(f, "NeighbourReply()"),
            HpvMsg::Shuffle { .. } => write!(f, "Shuffle()"),
            HpvMsg::ShuffleReply(_, _) => write!(f, "ShuffleReply()"),
            HpvMsg::Disconnect(p) => write!(f, "Disconnect({})", p),
        }
    }
}

impl Message for HpvMsg {
    type Result = Result<(), io::Error>;
}
