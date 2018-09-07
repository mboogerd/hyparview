extern crate actix;
extern crate futures_channel;
extern crate futures_core;

use self::actix::prelude::*;
use self::actix::Recipient;
use bounded_set::BoundedSet;
use std::collections::HashSet;
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
    recipient: Recipient<HpvMsg>,
}

impl Message for Views {
    type Result = Result<(), io::Error>;
}

type ViewsRecipient = Recipient<Views>;

pub enum HpvMsg {
    Inspect(ViewsRecipient),
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

    pub fn add_passive_view(&mut self, peers: HashSet<Peer>) {
        peers.iter().cloned().for_each(|p| {
            self.passive_view.insert(p);
        });
    }

    pub fn add_active_view(&mut self, peers: HashSet<Peer>) {
        peers.iter().cloned().for_each(|p| {
            self.active_view.insert(p);
        });
    }
}

#[derive(Eq, PartialEq)]
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

    fn handle(&mut self, msg: HpvMsg, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            HpvMsg::Inspect(v) => {
                let result = v.do_send(Views::from_hyparview(self));
                if result.is_err() {
                    // TODO: Improve error logging
                    println!("Inspection requested, but failed to forward current view!")
                }
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod test;
