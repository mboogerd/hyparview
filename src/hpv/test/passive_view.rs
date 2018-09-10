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

    let shuffle = HpvMsg::Shuffle {
        origin: hpv.recipient().into(),
        exchange: HashSet::new(),
        ttl: Config::default().shuffle_rwl,
    };

    ap.expect_msg(SHUFFLE_INTERVAL + TIMEOUT, shuffle.clone());
    ap.expect_no_msg(SHUFFLE_INTERVAL / 2);
    ap.expect_msg(SHUFFLE_INTERVAL, shuffle.clone());
}
