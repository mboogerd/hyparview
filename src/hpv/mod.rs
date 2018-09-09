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
        Display::fmt(&self, f)
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
            HpvMsg::Disconnect(p) => write!(f, "Disconnect({})", p),
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
        self.config = config;
        self.apply_capacity_config();
    }

    pub fn change_config<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Config) -> (),
    {
        f(&mut self.config);
        self.apply_capacity_config();
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

    fn apply_capacity_config(&mut self) {
        self.active_view
            .set_capacity(self.config.max_active_view_size);
        self.passive_view
            .set_capacity(self.config.max_passive_view_size);
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
        let self_peer: Peer = ctx.address().recipient().into();
        match msg {
            HpvMsg::Inspect(v) => self.handle_inspect(v),
            HpvMsg::InitiateJoin(v) => self.handle_init_join(self_peer, v),
            HpvMsg::Join(p) => self.handle_join(self_peer, p),
            HpvMsg::ForwardJoin {
                joining: p,
                forwarder: f,
                ttl,
            } => self.handle_forward_join(self_peer, p, f, ttl),
            HpvMsg::Disconnect(p) => self.handle_disconnect(p),
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

    fn handle_init_join(&mut self, self_recipient: Peer, bootstrap: Peer) {
        bootstrap
            .recipient
            .do_send(HpvMsg::Join(self_recipient.into()))
            .log_error("Failed to dispatch Join request to bootstrap node");
    }

    fn handle_join(&mut self, self_peer: Peer, new_peer: Peer) {
        if !self.active_view.contains(&new_peer) && self.active_view.is_full() {
            self.drop_random_active_peer(&self_peer);
        }
        self.active_view.for_each(|p| {
            p.recipient
                .do_send(HpvMsg::ForwardJoin {
                    joining: new_peer.clone(),
                    forwarder: self_peer.clone(),
                    ttl: self.config.active_rwl,
                })
                .log_error("Failed to forward join");
        });
        self.promote_peer(new_peer);
    }

    fn handle_forward_join(
        &mut self,
        self_peer: Peer,
        new_peer: Peer,
        forwarder: Peer,
        ttl: usize,
    ) {
        if ttl == 0 || self.active_view.len() == 0 {
            self.add_node_to_active_view(self_peer, new_peer);
        } else {
            if ttl == self.config.passive_rwl {
                self.add_node_to_passive_view(self_peer.clone(), new_peer.clone());
            }

            if ttl > 0 {
                // set of candidates to foward excludes the one who forwarded
                // TODO: Verify whether remove+insert is actually safe. A non-active node could have considered us active?
                self.active_view.remove(&forwarder);
                if self.active_view.len() > 0 {
                    self.active_view.sample_one().iter().for_each(|p| {
                        p.recipient
                            .do_send(HpvMsg::ForwardJoin {
                                joining: new_peer.clone(),
                                forwarder: self_peer.clone(),
                                ttl: ttl - 1,
                            })
                            .log_error("Failed to forward join to random peer");
                    });
                }
                self.active_view.insert(forwarder);
            }
        }
    }

    fn handle_disconnect(&mut self, new_peer: Peer) {
        self.active_view.for_each(|p| {
            p.recipient
                .do_send(HpvMsg::Join(new_peer.clone()))
                .log_error("Failed to forward join");
        });
    }

    fn drop_random_active_peer(&mut self, self_peer: &Peer) {
        // FIXME: Shouldn't need clone???
        match self.active_view.sample_one().cloned() {
            Some(node) => {
                node.recipient
                    .do_send(HpvMsg::Disconnect(self_peer.clone()))
                    .log_error("Failed to send Disconnect");
                self.active_view.remove(&node);
                self.passive_view.insert(node);
            }
            None => {
                println!("Wanted to drop random active peer, but none found");
            }
        }
    }

    fn promote_peer(&mut self, new_peer: Peer) {
        self.active_view.insert(new_peer);
        // TODO: Connect to the peer / Start watching the peer for disconnect
    }

    fn add_node_to_active_view(&mut self, self_peer: Peer, new_peer: Peer) {
        if new_peer != self_peer && !self.active_view.contains(&new_peer) {
            if self.active_view.is_full() {
                self.drop_random_active_peer(&self_peer);
            }
            self.promote_peer(new_peer);
        }
    }

    fn add_node_to_passive_view(&mut self, self_peer: Peer, new_peer: Peer) {
        if new_peer != self_peer && !self.active_view.contains(&new_peer)
            && !self.passive_view.contains(&new_peer)
        {
            if self.passive_view.is_full() {
                // This is safe for any view with positive capacity
                // FIXME: Shouldn't need clone???
                let remove = self.passive_view.sample_one().cloned().unwrap();
                self.passive_view.remove(&remove);
            }
            self.passive_view.insert(new_peer);
        }
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
