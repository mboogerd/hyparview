extern crate actix;
extern crate futures;

use self::actix::prelude::*;
use super::*;
use hpv::{BoundedSet, HpvMsg, Views};

#[test]
fn allow_inspections() {
    let _ = System::new("test");
    let (_, addr) = start_hyparview(|_| {});
    let (rx, view_recipient): (Receiver<Views>, Recipient<Views>) = mock_recipient();
    let _req = addr.send(HpvMsg::Inspect(view_recipient));

    rx.expect_msg(
        TIMEOUT,
        Views {
            active_view: BoundedSet::new(Config::default().max_active_view_size),
            passive_view: BoundedSet::new(Config::default().max_passive_view_size),
        },
    );
}
