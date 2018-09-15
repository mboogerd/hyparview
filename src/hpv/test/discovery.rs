extern crate actix;
extern crate futures;

use self::actix::prelude::*;
use super::*;

#[test]
fn publish_excludes_views() {
    let _ = System::new("test");

    let (_, ap) = mock_hpv_peer();
    let (_, pp) = mock_hpv_peer();
    let (_, pubp1) = mock_hpv_peer();
    let (_, pubp2) = mock_hpv_peer();

    let (out, hpv) = new_hyparview(|x| {
        x.add_active_node(ap.clone()).add_passive_node(pp.clone());
    });

    hpv.publish_peers(hashset!{ap.clone(), pp.clone(), pubp1.clone(), pubp2.clone()});

    let r1 = out.recv_msg(TIMEOUT);
    let r2 = out.recv_msg(TIMEOUT);

    assert_eq!(hashset!{pubp1, pubp2}, hashset!{r1, r2});
}

#[test]
fn publish_on_discovery() {
    let _ = System::new("test");

    let (_, ap1) = mock_hpv_peer();
    let (_, ap2) = mock_hpv_peer();
    let (_, pp) = mock_hpv_peer();
    let (_, discovered) = mock_hpv_peer();
    let (_, self_peer) = mock_hpv_peer();

    let (out, mut hpv) = new_hyparview(|x| {
        x.add_active_node(ap1.clone())
            .add_active_node(ap2.clone())
            .add_passive_node(pp.clone())
            .set_shuffling(0, hashset!{discovered.clone()})
            .change_config(|c| {
                c.max_active_view_size = 2;
                c.max_passive_view_size = 1;
            });
    });

    let exchange = hashset!{discovered.clone(), ap1.clone(), pp.clone()};

    hpv.handle_shuffle(0, self_peer.clone(), exchange.clone(), 10);
    out.expect_msg(TIMEOUT, discovered.clone());

    hpv.handle_shuffle_reply(0, exchange.clone());
    out.expect_msg(TIMEOUT, discovered.clone());

    hpv.handle_neighbour(self_peer.clone(), discovered.clone(), false);
    out.expect_msg(TIMEOUT, discovered.clone());

    hpv.handle_forward_join(
        self_peer.clone(),
        discovered.clone(),
        discovered.clone(),
        10,
    );
    out.expect_msg(TIMEOUT, discovered.clone());

    hpv.handle_join(self_peer.clone(), discovered.clone());
    out.expect_msg(TIMEOUT, discovered.clone());
}
