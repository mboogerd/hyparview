extern crate actix;
extern crate futures;
extern crate futures_channel;

use self::actix::prelude::*;
use super::*;
use hpv::HpvMsg;

#[test]
fn initiate_join() {
    let _ = System::new("test");
    let (_, mut hpv) = new_hyparview(|_| {});
    let (rx, bootstrap) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();
    hpv.handle_init_join(mock_self.clone(), bootstrap);
    rx.expect_msg(TIMEOUT, HpvMsg::Join(mock_self));
}

#[test]
fn broadcast_to_active_view() {
    let _ = System::new("test");
    let (ap1, actv_probe1) = mock_hpv_peer();
    let (ap2, actv_probe2) = mock_hpv_peer();
    let (pp1, pasv_probe1) = mock_hpv_peer();
    let (_, join_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe1.clone())
            .add_active_node(actv_probe2.clone())
            .add_passive_node(pasv_probe1.clone());
    });

    hpv.handle_join(mock_self.clone(), join_probe.clone());

    let forward_join = HpvMsg::ForwardJoin {
        joining: join_probe.clone(),
        forwarder: mock_self.clone(),
        ttl: Config::default().active_rwl,
    };
    ap1.expect_msg(TIMEOUT, forward_join.clone());
    ap2.expect_msg(TIMEOUT, forward_join.clone());
    pp1.expect_no_msg(TIMEOUT);
}

#[test]
fn make_room_for_joins() {
    let _ = System::new("test");
    let (ap1, actv_probe1) = mock_hpv_peer();
    let (ap2, actv_probe2) = mock_hpv_peer();
    let (pp1, pasv_probe) = mock_hpv_peer();
    let (_, join_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe1.clone())
            .add_active_node(actv_probe2.clone())
            .add_passive_node(pasv_probe.clone())
            .change_config(|c| {
                c.max_active_view_size = 2;
            });
    });

    hpv.handle_join(mock_self.clone(), join_probe.clone());

    pp1.expect_no_msg(TIMEOUT);

    assert_eq!(hpv.active_view.len(), 2);
    assert!(hpv.active_view.contains(&join_probe.clone()));
    // Either of the previously active peers should still be there
    let ((live_rcv, live_probe), (dead_rcv, dead_probe)) = {
        if hpv.active_view.contains(&actv_probe1.clone()) {
            ((ap1, actv_probe1), (ap2, actv_probe2))
        } else {
            ((ap2, actv_probe2), (ap1, actv_probe1))
        }
    };

    assert!(hpv.active_view.contains(&live_probe));
    assert!(hpv.passive_view.contains(&dead_probe));
    assert!(hpv.passive_view.contains(&pasv_probe));

    dead_rcv.expect_msg(TIMEOUT, HpvMsg::Disconnect(mock_self.clone()));
    live_rcv.expect_msg(
        TIMEOUT,
        HpvMsg::ForwardJoin {
            joining: join_probe.clone(),
            forwarder: mock_self.clone(),
            ttl: Config::default().active_rwl,
        },
    );
}

#[test]
fn include_joiner_at_ttl0() {
    let _ = System::new("test");
    let (_, actv_probe) = mock_hpv_peer();
    let (_, join_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();
    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe.clone()).change_config(|c| {
            c.max_active_view_size = 1;
        });
    });

    hpv.handle_forward_join(mock_self.clone(), join_probe.clone(), join_probe.clone(), 0);

    assert!(hpv.active_view.contains(&join_probe));
}

#[test]
fn include_joiner_when_active_view_empty() {
    let _ = System::new("test");
    let (_, join_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();
    let (_, mut hpv) = new_hyparview(|_| {});

    hpv.handle_forward_join(mock_self.clone(), join_probe.clone(), join_probe.clone(), 1);

    assert!(hpv.active_view.contains(&join_probe));
}

#[test]
fn continue_forwarding_until_ttl0() {
    let _ = System::new("test");
    let (fp, forw_probe) = mock_hpv_peer();
    let (_, actv_probe) = mock_hpv_peer();
    let (_, join_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();
    let (_, mut hpv) = new_hyparview(|x| {
        // non-empty active view
        x.add_active_node(forw_probe.clone())
            .add_active_node(actv_probe.clone());
    });

    // Don't include the node of a ForwardJoin when TTL != passive_rwl
    hpv.handle_forward_join(
        mock_self.clone(),
        join_probe.clone(),
        actv_probe.clone(),
        Config::default().passive_rwl + 1,
    );
    fp.expect_msg(
        TIMEOUT,
        HpvMsg::ForwardJoin {
            joining: join_probe.clone(),
            forwarder: mock_self.clone(),
            ttl: Config::default().passive_rwl,
        },
    );
    assert_eq!(hpv.passive_view.len(), 0);

    // Include the node of a ForwardJoin when TTL == passive_rwl
    hpv.handle_forward_join(
        mock_self.clone(),
        join_probe.clone(),
        actv_probe.clone(),
        Config::default().passive_rwl,
    );
    fp.expect_msg(
        TIMEOUT,
        HpvMsg::ForwardJoin {
            joining: join_probe.clone(),
            forwarder: mock_self.clone(),
            ttl: Config::default().passive_rwl - 1,
        },
    );
    assert!(hpv.passive_view.contains(&join_probe));
}
