extern crate actix;
extern crate futures;
extern crate futures_channel;

use self::actix::prelude::*;
use super::*;
use hpv::HpvMsg;
use std::sync::mpsc::Receiver;

#[test]
fn initiate_join() {
    let _ = System::new("test");
    let (_, mut hpv) = new_hyparview(|_| {});
    let (rx, bootstrap): (Receiver<HpvMsg>, Recipient<HpvMsg>) = mock_recipient();
    let (_, mock_self): (Receiver<HpvMsg>, Recipient<HpvMsg>) = mock_recipient();
    hpv.handle_init_join(mock_self.clone(), bootstrap);
    rx.expect_msg(TIMEOUT, HpvMsg::Join(mock_self.into()));
}

#[test]
fn broadcast_to_active_view() {
    let _ = System::new("test");
    let (p1, probe1): (Receiver<HpvMsg>, Recipient<HpvMsg>) = mock_recipient();
    let (p2, probe2): (Receiver<HpvMsg>, Recipient<HpvMsg>) = mock_recipient();
    let (p3, probe3): (Receiver<HpvMsg>, Recipient<HpvMsg>) = mock_recipient();

    let (_, mut hpv) = new_hyparview(|x| {
        x.add_active_node(probe1.clone().into())
            .add_active_node(probe2.clone().into())
            .add_passive_node(probe3.clone().into());
    });

    hpv.handle_join(probe3.clone());

    p1.expect_msg(TIMEOUT, HpvMsg::Join(probe3.clone().into()));
    p2.expect_msg(TIMEOUT, HpvMsg::Join(probe3.clone().into()));
    p3.expect_no_msg(TIMEOUT);
}
