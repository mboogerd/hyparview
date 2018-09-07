extern crate actix;
extern crate futures;
extern crate futures_channel;

use self::actix::prelude::*;
use super::{mock_recipient, start_hyparview};
use bounded_set::BoundedSet;
use hpv::Config;
use hpv::HpvMsg;
use hpv::Views;
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[test]
fn allow_inspections() {
    let _ = System::new("test");
    let (_, addr) = start_hyparview(|_| {});
    let (rx, view_recipient): (Receiver<Views>, Recipient<Views>) = mock_recipient();
    let _req = addr.send(HpvMsg::Inspect(view_recipient));

    assert!(
        rx.recv_timeout(Duration::from_millis(100))
            .expect("Timeout waiting for Views") == Views {
            active_view: BoundedSet::new(Config::default().max_active_view_size),
            passive_view: BoundedSet::new(Config::default().max_passive_view_size)
        }
    );
}
