extern crate actix;

use self::actix::prelude::*;
use bounded_set::BoundedSet;
use std::collections::HashSet;
use std::io;
use std::sync::mpsc::channel;
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

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct Peer {}

pub struct Inspect;

impl Message for Inspect {
    type Result = Result<Views, io::Error>;
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
        let (tx, rx) = channel();
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

#[derive(Debug, Eq, PartialEq)]
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

impl Handler<Inspect> for HyParViewActor {
    type Result = Result<Views, io::Error>;

    fn handle(&mut self, _msg: Inspect, _ctx: &mut Context<Self>) -> Self::Result {
        println!("Received inspect");
        Ok(Views::from_hyparview(self))
    }
}

#[cfg(test)]
mod test_join;

#[cfg(test)]
mod test_passive_view;

#[cfg(test)]
mod test_active_view;

#[cfg(test)]
mod test_loquat;
