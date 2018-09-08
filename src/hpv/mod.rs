extern crate actix;
extern crate futures_channel;
extern crate futures_core;

use self::actix::prelude::*;
use self::actix::Recipient;
use bounded_set::BoundedSet;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

#[derive(Clone)]
pub struct Config {
    max_active_view_size: usize,
    max_passive_view_size: usize,
    active_rwl: usize,
    passive_rwl: usize,
    shuffle_rwl: usize,
    shuffle_active: usize,
    shuffle_passive: usize,
    shuffle_interval: Duration,
}

impl Config {
    pub fn default() -> Config {
        Config {
            max_active_view_size: 4,
            max_passive_view_size: 4,
            active_rwl: 3,
            passive_rwl: 2,
            shuffle_rwl: 1,
            shuffle_active: 2,
            shuffle_passive: 2,
            shuffle_interval: Duration::from_secs(30),
        }
    }
}

// Dynamic address
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct Peer {
    recipient: HpvRecipient,
}

impl Into<Peer> for HpvRecipient {
    fn into(self) -> Peer {
        Peer { recipient: self }
    }
}

impl fmt::Debug for Peer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hasher = DefaultHasher::new();
        self.recipient.hash(&mut hasher);
        write!(f, "Peer {}", hasher.finish())
    }
}
impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hasher = DefaultHasher::new();
        self.recipient.hash(&mut hasher);
        write!(f, "Peer {}", hasher.finish())
    }
}

impl Message for Views {
    type Result = Result<(), io::Error>;
}

type ViewsRecipient = Recipient<Views>;

type HpvRecipient = Recipient<HpvMsg>;

#[derive(Eq, PartialEq)]
pub enum HpvMsg {
    Inspect(ViewsRecipient),
    InitiateJoin(Peer),
    Join(Peer),
}

impl fmt::Debug for HpvMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HpvMsg::Inspect(_) => write!(f, "Inspect"),
            HpvMsg::InitiateJoin(p) => write!(f, "InitiateJoin({})", p),
            HpvMsg::Join(p) => write!(f, "Join({})", p),
        }
    }
}

impl Message for HpvMsg {
    type Result = Result<(), io::Error>;
}

pub struct HyParViewActor {
    config: Config,
    active_view: BoundedSet<Peer>,
    passive_view: BoundedSet<Peer>,
    // TODO: This should be replaced by `Environment` once implemented
    out: Sender<Peer>,
}

impl HyParViewActor {
    pub fn default() -> (Receiver<Peer>, HyParViewActor) {
        let (tx, rx) = mpsc::channel();
        let config = Config::default();
        let hpv = HyParViewActor {
            config: config.clone(),
            active_view: BoundedSet::new(config.max_active_view_size),
            passive_view: BoundedSet::new(config.max_passive_view_size),
            out: tx,
        };
        (rx, hpv)
    }

    pub fn set_config(&mut self, config: Config) {
        assert!(config.max_active_view_size >= self.active_view.capacity);
        assert!(config.max_passive_view_size >= self.passive_view.capacity);
        self.config = config;
    }

    pub fn add_passive_node(&mut self, p: Peer) -> &mut Self {
        self.passive_view.insert(p);
        self
    }

    pub fn add_active_node(&mut self, p: Peer) -> &mut Self {
        self.active_view.insert(p);
        self
    }

    pub fn add_passive_view(&mut self, peers: HashSet<Peer>) -> &mut Self {
        peers.iter().cloned().for_each(|p| {
            self.passive_view.insert(p);
        });
        self
    }

    pub fn add_active_view(&mut self, peers: HashSet<Peer>) -> &mut Self {
        peers.iter().cloned().for_each(|p| {
            self.active_view.insert(p);
        });
        self
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct Views {
    active_view: BoundedSet<Peer>,
    passive_view: BoundedSet<Peer>,
}

impl Views {
    pub fn from_hyparview(actor: &HyParViewActor) -> Views {
        Views {
            active_view: actor.active_view.clone(),
            passive_view: actor.passive_view.clone(),
        }
    }
}

impl Actor for HyParViewActor {
    type Context = Context<Self>;
}

impl Handler<HpvMsg> for HyParViewActor {
    type Result = Result<(), io::Error>;

    fn handle(&mut self, msg: HpvMsg, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            HpvMsg::Inspect(v) => self.handle_inspect(v),
            HpvMsg::InitiateJoin(v) => {
                self.handle_init_join(ctx.address().recipient(), v.recipient)
            }
            HpvMsg::Join(v) => self.handle_join(v.recipient),
        };
        // Satisfy actix contract
        Ok(())
    }
}

impl HyParViewActor {
    fn handle_inspect(&self, v: ViewsRecipient) {
        v.do_send(Views::from_hyparview(self))
            .log_error("Inspection requested, but failed to forward current view!");
    }

    fn handle_init_join(&mut self, self_recipient: HpvRecipient, bootstrap: HpvRecipient) {
        bootstrap
            .do_send(HpvMsg::Join(self_recipient.into()))
            .log_error("Failed to dispatch Join request to bootstrap node");
    }

    fn handle_join(&mut self, new_peer: HpvRecipient) {
        self.active_view.for_each(|p| {
            p.recipient
                .do_send(HpvMsg::Join(new_peer.clone().into()))
                .log_error("Failed to forward join");
        });
    }
}

trait Logged {
    fn log_error(&self, msg: &str);
}

impl<V, E: Display> Logged for Result<V, E> {
    fn log_error(&self, msg: &str) {
        match self {
            Ok(_) => (),
            Err(e) => println!("[Error] Description: '{}'. Cause: '{}'", msg, e),
        }
    }
}

#[cfg(test)]
mod test;
