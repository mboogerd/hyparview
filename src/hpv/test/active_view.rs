extern crate actix;
extern crate futures;
extern crate futures_channel;

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

    hpv.handle_disconnect(mock_self.clone(), actv_probe.clone());
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

    hpv.handle_disconnect(mock_self.clone(), actv_probe1.clone());
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
