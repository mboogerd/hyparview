extern crate actix;
extern crate futures;

use self::actix::prelude::*;
use super::*;
use hpv::HpvMsg;

#[test]
fn promote_prioritized_when_final_active_disconnects() {
    let _ = System::new("test");
    let (_, actv_probe) = mock_hpv_peer();
    let (pp, pasv_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe.clone())
            .add_passive_node(pasv_probe.clone());
    });

    hpv.handle_disconnect(mock_self.clone(), &actv_probe);
    pp.expect_msg(
        TIMEOUT,
        HpvMsg::Neighbour {
            peer: mock_self.clone(),
            prio: true,
        },
    );

    assert_eq!(hpv.passive_view.len(), 0);
    assert!(hpv.active_view.contains(&pasv_probe));
}

#[test]
fn promote_unprioritized_when_nonfinal_active_disconnects() {
    let _ = System::new("test");
    let (_, actv_probe1) = mock_hpv_peer();
    let (_, actv_probe2) = mock_hpv_peer();
    let (pp, pasv_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe1.clone())
            .add_active_node(actv_probe2.clone())
            .add_passive_node(pasv_probe.clone());
    });

    hpv.handle_disconnect(mock_self.clone(), &actv_probe1);
    pp.expect_msg(
        TIMEOUT,
        HpvMsg::Neighbour {
            peer: mock_self.clone(),
            prio: false,
        },
    );

    assert_eq!(hpv.passive_view.len(), 0);
    assert!(hpv.active_view.contains(&pasv_probe));
    assert!(hpv.active_view.contains(&actv_probe2));
}

#[test]
fn always_accept_prio_neighbour_requests() {
    let _ = System::new("test");
    let (ap, actv_probe) = mock_hpv_peer();
    let (np, neighbour_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe.clone()).change_config(|c| {
            c.max_active_view_size = 1;
        });
    });

    hpv.handle_neighbour(mock_self.clone(), neighbour_probe.clone(), true);
    np.expect_msg(
        TIMEOUT,
        HpvMsg::NeighbourReply {
            peer: mock_self.clone(),
            accepted: true,
        },
    );
    ap.expect_msg(TIMEOUT, HpvMsg::Disconnect(mock_self.clone()));
    assert!(hpv.active_view.contains(&neighbour_probe));
}

#[test]
fn accept_nonprio_when_activeview_nonfull() {
    let _ = System::new("test");
    let (ap, actv_probe) = mock_hpv_peer();
    let (np, neighbour_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe.clone());
    });

    hpv.handle_neighbour(mock_self.clone(), neighbour_probe.clone(), false);
    np.expect_msg(
        TIMEOUT,
        HpvMsg::NeighbourReply {
            peer: mock_self.clone(),
            accepted: true,
        },
    );
    ap.expect_no_msg(TIMEOUT);
    assert!(hpv.active_view.contains(&actv_probe));
    assert!(hpv.active_view.contains(&neighbour_probe));
}

#[test]
fn reject_nonprio_when_activeview_full() {
    let _ = System::new("test");
    let (ap, actv_probe) = mock_hpv_peer();
    let (np, neighbour_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe.clone()).change_config(|c| {
            c.max_active_view_size = 1;
        });
    });

    hpv.handle_neighbour(mock_self.clone(), neighbour_probe.clone(), false);
    np.expect_msg(
        TIMEOUT,
        HpvMsg::NeighbourReply {
            peer: mock_self.clone(),
            accepted: false,
        },
    );
    ap.expect_no_msg(TIMEOUT);
    assert!(hpv.active_view.contains(&actv_probe));
}

#[test]
fn rejection_should_demote_requested_and_promote_another() {
    let _ = System::new("test");
    let (_, actv_probe) = mock_hpv_peer();
    let (_, rjct_probe) = mock_hpv_peer();
    let (cp, cand_probe) = mock_hpv_peer();
    let (_, mock_self) = mock_hpv_peer();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(actv_probe.clone())
            .add_active_node(rjct_probe.clone())
            .add_passive_node(cand_probe.clone());
    });

    hpv.handle_neighbour_reply(mock_self.clone(), rjct_probe.clone(), false);
    cp.expect_msg(
        TIMEOUT,
        HpvMsg::Neighbour {
            peer: mock_self.clone(),
            prio: false,
        },
    );

    assert!(hpv.active_view.contains(&actv_probe));
    assert!(hpv.active_view.contains(&cand_probe));
    assert!(hpv.passive_view.contains(&rjct_probe));
}
