extern crate actix;
extern crate bytes;
extern crate futures;
//extern crate futures_channel;
//extern crate futures_core;
extern crate libp2p_core;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;
extern crate tokio_timer;

use self::actix::prelude::*;
use self::actix::Recipient;
use bounded_set::BoundedSet;
use std::collections::HashSet;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use util::logged::*;

mod config;
pub use self::config::*;

mod peer;
pub use self::peer::*;

mod message;
pub use self::message::*;

mod views;
pub use self::views::*;

pub mod libp2p;

type ViewsRecipient = Recipient<Views>;

type HpvRecipient = Recipient<HpvMsg>;

pub struct HyParViewActor {
    config: Config,
    active_view: BoundedSet<Peer>,
    passive_view: BoundedSet<Peer>,
    // TODO: This should be replaced by `Environment` once implemented
    out: Sender<Peer>,
    shuffle_id: u32,
    shuffling: bool, // true if a request is dispatched, but no reply received
    offer: HashSet<Peer>,
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
            shuffle_id: 0,
            shuffling: false,
            offer: HashSet::default(),
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

    fn set_shuffling(&mut self, id: u32, offer: HashSet<Peer>) -> &mut Self {
        self.shuffling = true;
        self.shuffle_id = id;
        self.offer = offer;
        self
    }

    fn apply_capacity_config(&mut self) {
        self.active_view
            .set_capacity(self.config.max_active_view_size);
        self.passive_view
            .set_capacity(self.config.max_passive_view_size);
    }
}

impl Actor for HyParViewActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(
            self.config.shuffle_interval,
            |hpv: &mut HyParViewActor, ctx: &mut Self::Context| {
                let self_peer: Peer = ctx.address().recipient().into();
                hpv.initiate_shuffle(self_peer);
            },
        );
    }
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
                joining,
                forwarder,
                ttl,
            } => self.handle_forward_join(self_peer, joining, forwarder, ttl),
            HpvMsg::Neighbour { peer, prio } => self.handle_neighbour(self_peer, peer, prio),
            HpvMsg::NeighbourReply { peer, accepted } => {
                self.handle_neighbour_reply(self_peer, peer, accepted)
            }
            HpvMsg::Shuffle {
                id,
                origin,
                exchange,
                ttl,
            } => self.handle_shuffle(id, origin, exchange, ttl),
            HpvMsg::ShuffleReply(id, ps) => self.handle_shuffle_reply(id, ps),
            HpvMsg::Disconnect(p) => self.handle_disconnect(self_peer, &p),
        };
        // Satisfy actix contract
        Ok(())
    }
}

impl HyParViewActor {
    pub fn handle_inspect(&self, v: ViewsRecipient) {
        v.do_send(Views::from_hyparview(self))
            .log_error("Inspection requested, but failed to forward current view!");
    }

    pub fn handle_init_join(&mut self, self_recipient: Peer, bootstrap: Peer) {
        bootstrap
            .recipient
            .do_send(HpvMsg::Join(self_recipient.into()))
            .log_error("Failed to dispatch Join request to bootstrap node");
    }

    pub fn handle_join(&mut self, self_peer: Peer, new_peer: Peer) {
        self.publish_peer(new_peer.clone());

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

    pub fn handle_forward_join(
        &mut self,
        self_peer: Peer,
        new_peer: Peer,
        forwarder: Peer,
        ttl: usize,
    ) {
        self.publish_peer(new_peer.clone());

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
                        let msg = HpvMsg::ForwardJoin {
                            joining: new_peer.clone(),
                            forwarder: self_peer.clone(),
                            ttl: ttl - 1,
                        };
                        p.recipient
                            .do_send(msg)
                            .log_error("Failed to forward join to random peer");
                    });
                } else {
                    // If we cannot forward, it's better to expand our active view
                    self.add_node_to_active_view(self_peer, new_peer);
                }
                self.active_view.insert(forwarder);
            }
        }
    }

    pub fn handle_disconnect(&mut self, self_peer: Peer, remove: &Peer) {
        if self.active_view.contains(remove) {
            self.active_view.remove(remove);
        }

        self.promote_random_peer(self_peer);
    }

    pub fn promote_random_peer(&mut self, self_peer: Peer) {
        match self.passive_view.sample_one().cloned() {
            Some(candidate) => {
                candidate
                    .recipient
                    .do_send(HpvMsg::Neighbour {
                        peer: self_peer,
                        prio: self.active_view.len() == 0,
                    })
                    .log_error("Failed to promote Neighbour");
                self.passive_view.remove(&candidate);
                self.promote_peer(candidate);
            }
            None => {}
        }
    }

    pub fn drop_random_active_peer(&mut self, self_peer: &Peer) {
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

    pub fn promote_peer(&mut self, new_peer: Peer) {
        self.active_view.insert(new_peer);
        // TODO: Connect to the peer / Start watching the peer for disconnect
    }

    pub fn add_node_to_active_view(&mut self, self_peer: Peer, new_peer: Peer) {
        if new_peer != self_peer && !self.active_view.contains(&new_peer) {
            if self.active_view.is_full() {
                self.drop_random_active_peer(&self_peer);
            }
            self.promote_peer(new_peer);
        }
    }

    pub fn add_node_to_passive_view(&mut self, self_peer: Peer, new_peer: Peer) {
        if new_peer != self_peer
            && !self.active_view.contains(&new_peer)
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

    pub fn handle_neighbour(&mut self, self_peer: Peer, neighbour: Peer, prio: bool) {
        self.publish_peer(neighbour.clone());

        if prio && self.active_view.is_full() {
            self.drop_random_active_peer(&self_peer);
        }

        if self.active_view.is_full() {
            neighbour
                .recipient
                .do_send(HpvMsg::NeighbourReply {
                    peer: self_peer,
                    accepted: false,
                })
                .log_error("Failed to send neighbour rejection message");
        } else {
            neighbour
                .recipient
                .do_send(HpvMsg::NeighbourReply {
                    peer: self_peer,
                    accepted: true,
                })
                .log_error("Failed to send neighbour acceptance message");

            self.promote_peer(neighbour.clone());
            self.passive_view.remove(&neighbour);
        }
    }

    pub fn handle_neighbour_reply(&mut self, self_peer: Peer, neighbour: Peer, accepted: bool) {
        self.publish_peer(neighbour.clone());

        if !accepted {
            self.handle_disconnect(self_peer, &neighbour);
            self.passive_view.insert(neighbour);
        }
    }

    pub fn initiate_shuffle(&mut self, self_peer: Peer) {
        match self.active_view.sample_one() {
            Some(shuffle_target) => {
                // Clone the active-view to sample of it, without the shuffle target...?
                // TODO: Improve!
                let mut clone = self.active_view.clone();
                let active_part = {
                    clone.remove(&shuffle_target);
                    clone.sample(self.config.shuffle_active)
                };
                let passive_part = self.passive_view.sample(self.config.shuffle_passive);
                let shuffle_request = HpvMsg::Shuffle {
                    id: self.shuffle_id,
                    origin: self_peer.clone(),
                    exchange: active_part
                        .union(&passive_part)
                        .map(|e| (**e).clone())
                        .collect(),
                    ttl: self.config.shuffle_rwl,
                };

                self.shuffle_id += 1;

                shuffle_target
                    .recipient
                    .do_send(shuffle_request)
                    .log_error("Failed to initate shuffle request");
            }
            None => {}
        }

        if !self.active_view.is_full() {
            self.promote_random_peer(self_peer);
        }
    }

    pub fn handle_shuffle(&mut self, id: u32, origin: Peer, exchange: HashSet<Peer>, ttl: usize) {
        self.publish_peers(exchange.clone());

        if ttl == 1 || self.active_view.len() <= 1 {
            // construct a response with candidates from our passive view
            let mut passive_fragment = self.passive_view.clone();
            exchange.iter().for_each(|p| {
                passive_fragment.remove(p);
            });
            passive_fragment.remove(&origin);
            let sample: HashSet<Peer> = passive_fragment
                .sample(exchange.len() + 1)
                .iter()
                .map(|x| (**x).clone())
                .collect();

            origin
                .recipient
                .do_send(HpvMsg::ShuffleReply(id, sample.clone()))
                .log_error("Failed to reply to shuffle request");

            let mut all_peers = exchange;
            if !self.active_view.contains(&origin) {
                all_peers.insert(origin);
            }
            self.passive_view.bounded_union(&all_peers, &sample)
        } else {
            // FIXME: structural sharing would really start to be beneficial...
            let forward_message = HpvMsg::Shuffle {
                id: self.shuffle_id,
                origin: origin.clone(),
                exchange: exchange,
                ttl: ttl - 1,
            };
            self.shuffle_id += 1;
            let mut active_fragment = self.active_view.clone();
            active_fragment.remove(&origin);
            match active_fragment.sample_one() {
                Some(target) => target
                    .recipient
                    .do_send(forward_message)
                    .log_error("Failed to propagate Shuffle request"),
                _ => {}
            };
        }
    }

    pub fn handle_shuffle_reply(&mut self, shuffle_reply_id: u32, exchange: HashSet<Peer>) {
        self.publish_peers(exchange.clone());

        if self.shuffling && self.shuffle_id == shuffle_reply_id {
            self.passive_view.bounded_union(&exchange, &self.offer);
            self.offer = HashSet::default();
        } else if self.shuffle_id < shuffle_reply_id {
            println!("[WARN] Received shuffle reply with id ({}). This exceeds largest dispatched shuffle request id ({})", shuffle_reply_id, self.shuffle_id);
        } else if self.shuffle_id > shuffle_reply_id {
            println!(
                "[INFO] Received reply ({}) to old shuffle request (current={}), ignoring",
                shuffle_reply_id, self.shuffle_id
            );
        } else {
            println!("[INFO] Received duplicate shuffle reply")
        }
    }

    pub fn publish_peer(&self, peer: Peer) {
        self.publish_peers(hashset!{peer});
    }

    pub fn publish_peers(&self, peers: HashSet<Peer>) {
        peers
            .iter()
            .filter(|p| !self.active_view.contains(p))
            .filter(|p| !self.passive_view.contains(p))
            .cloned()
            .for_each(|p| {
                self.out.send(p).log_error("Failed to publish peer");
            });
    }
}

#[cfg(test)]
mod test;
