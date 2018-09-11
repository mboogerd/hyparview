extern crate actix;
extern crate futures;
extern crate futures_channel;

use self::actix::prelude::*;
use super::*;
use hpv::HpvMsg;
use std::collections::HashSet;

const SHUFFLE_INTERVAL: Duration = TIMEOUT;

#[test]
fn periodically_initiate_shuffle() {
    let _ = System::new("test");
    let (ap, actv_probe) = mock_hpv_peer();

    let (_, hpv) = start_hyparview(|x| {
        x.add_active_node(actv_probe.clone())
            .change_config(|c| c.shuffle_interval = SHUFFLE_INTERVAL);
    });

    ap.expect_msg(
        SHUFFLE_INTERVAL + TIMEOUT,
        HpvMsg::Shuffle {
            id: 0,
            origin: hpv.clone().recipient().into(),
            exchange: HashSet::new(),
            ttl: Config::default().shuffle_rwl,
        },
    );
    ap.expect_no_msg(SHUFFLE_INTERVAL / 2);
    ap.expect_msg(
        SHUFFLE_INTERVAL,
        HpvMsg::Shuffle {
            id: 1,
            origin: hpv.clone().recipient().into(),
            exchange: HashSet::new(),
            ttl: Config::default().shuffle_rwl,
        },
    );
}

#[test]
fn include_passive_and_active_peers_without_target_in_shuffle() {
    let _ = System::new("test");

    let (shuffle_receiver, active_probe) = mock_hpv_peer();
    let ((_, pp1), (_, pp2), (_, pp3)) = (mock_hpv_peer(), mock_hpv_peer(), mock_hpv_peer());
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(active_probe.clone())
            .add_passive_node(pp1.clone())
            .add_passive_node(pp2.clone())
            .add_passive_node(pp3.clone())
            .change_config(|c| {
                c.shuffle_interval = SHUFFLE_INTERVAL;
                c.shuffle_active = 1; // not enough active peers available, make sure the shuffle target is not included
                c.shuffle_passive = 2; // can choose a subset
            });
    });

    hpv.initiate_shuffle(mock_self.clone());
    let shuffle = shuffle_receiver.recv_msg(TIMEOUT);

    match shuffle {
        HpvMsg::Shuffle {
            id: _,
            origin,
            exchange,
            ttl,
        } => {
            assert_eq!(origin, mock_self.clone());
            assert_eq!(ttl, Config::default().shuffle_rwl);

            assert_eq!(exchange.len(), 2);
            assert!(
                exchange.contains(&pp1) && exchange.contains(&pp2)
                    || exchange.contains(&pp1) && exchange.contains(&pp3)
                    || exchange.contains(&pp2) && exchange.contains(&pp3)
            );
        }
        _ => panic!("Did not receive expected shuffle message"),
    }
}

#[test]
fn walk_random_if_ttl_positive_and_activeviewsize_gt1() {
    let _ = System::new("test");

    let (shuffle_receiver, active_probe) = mock_hpv_peer();
    let (_, shuffle_initiator) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(active_probe.clone())
            .add_active_node(shuffle_initiator.clone())
            .change_config(|c| {
                c.shuffle_interval = SHUFFLE_INTERVAL;
                c.shuffle_active = 2; // not enough active peers available, make sure the shuffle target is not included
            });
    });

    hpv.handle_shuffle(0, shuffle_initiator.clone(), HashSet::new(), 2);
    shuffle_receiver.expect_msg(
        TIMEOUT,
        HpvMsg::Shuffle {
            id: 0,
            origin: shuffle_initiator,
            exchange: HashSet::new(),
            ttl: 1,
        },
    );
}

#[test]
fn handle_shuffle_when_ttl0() {
    let _ = System::new("test");

    let (_, active_probe) = mock_hpv_peer();
    let (reply_recv, shuffle_initiator) = mock_hpv_peer();
    let (_, pasv_peer) = mock_hpv_peer();
    let (_, shuffled_peer) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(active_probe.clone())
            .add_active_node(shuffle_initiator.clone())
            .add_passive_node(pasv_peer.clone())
            .add_passive_node(shuffled_peer.clone());
    });

    hpv.handle_shuffle(
        0,
        shuffle_initiator.clone(),
        hashset!{shuffled_peer.clone()},
        1,
    );

    reply_recv.expect_msg(
        TIMEOUT,
        HpvMsg::ShuffleReply(0, hashset!{pasv_peer.clone()}),
    );

    assert_eq!(hpv.passive_view.len(), 2);
    assert!(hpv.passive_view.contains(&pasv_peer));
    assert!(hpv.passive_view.contains(&shuffled_peer));
}

#[test]
fn handle_shuffle_when_activeviewsize_0or1() {
    let _ = System::new("test");

    let (_, active_probe) = mock_hpv_peer();
    let (reply_recv, shuffle_initiator) = mock_hpv_peer();
    let (_, pasv_peer) = mock_hpv_peer();
    let (_, shuffled_peer) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(active_probe.clone())
            .add_passive_node(pasv_peer.clone())
            .add_passive_node(shuffled_peer.clone());
    });

    hpv.handle_shuffle(
        0,
        shuffle_initiator.clone(),
        hashset!{shuffled_peer.clone()},
        1,
    );

    reply_recv.expect_msg(
        TIMEOUT,
        HpvMsg::ShuffleReply(0, hashset!{pasv_peer.clone()}),
    );

    assert_eq!(hpv.passive_view.len(), 3);
    assert!(hpv.passive_view.contains(&pasv_peer));
    assert!(hpv.passive_view.contains(&shuffled_peer));
    // The initiator can be included in the passive view, as it wasn't in either view
    assert!(hpv.passive_view.contains(&shuffle_initiator));
}

#[test]
fn merge_exchangeset_included_in_shuffle_reply() {
    let _ = System::new("test");

    let (_, actv_peer) = mock_hpv_peer();
    let (_, shuffled_peer) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_peer.clone())
            .set_shuffling(1337, HashSet::default());
    });

    hpv.handle_shuffle_reply(1337, hashset!{shuffled_peer.clone()});

    assert_eq!(hpv.active_view.len(), 1);
    assert!(hpv.active_view.contains(&actv_peer));

    assert_eq!(hpv.passive_view.len(), 1);
    assert!(hpv.passive_view.contains(&shuffled_peer));
}

#[test]
fn ignore_illegal_shuffle_replies() {
    let _ = System::new("test");
    let (_, shuffled_peer) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.set_shuffling(1337, HashSet::default());
    });

    hpv.handle_shuffle_reply(1336, hashset!{shuffled_peer.clone()});
    hpv.handle_shuffle_reply(1338, hashset!{shuffled_peer.clone()});

    assert_eq!(hpv.active_view.len(), 0);
    assert_eq!(hpv.passive_view.len(), 0);
}

#[test]
fn prioritize_removal_of_shuffled_peers() {
    let _ = System::new("test");

    let (_, actv_peer) = mock_hpv_peer();
    let (_, shuffled_peer) = mock_hpv_peer();
    let (_, received_peer) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_peer.clone())
            .add_passive_node(shuffled_peer.clone())
            .set_shuffling(1337, hashset!{shuffled_peer.clone()})
            .change_config(|c| {
                c.max_passive_view_size = 1;
            });
    });

    hpv.handle_shuffle_reply(1337, hashset!{received_peer.clone()});

    assert_eq!(hpv.active_view.len(), 1);
    assert!(hpv.active_view.contains(&actv_peer));

    assert_eq!(hpv.passive_view.len(), 1);
    assert!(hpv.passive_view.contains(&received_peer));
}
