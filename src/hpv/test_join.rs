extern crate actix;
extern crate futures;

use self::actix::prelude::*;
use self::actix::*;
use self::futures::Future;
use bounded_set::BoundedSet;
use hpv::Config;
use hpv::HyParViewActor;
use hpv::Inspect;
use hpv::Peer;
use hpv::Views;
use std::io;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::thread::*;

fn start_hyparview<F>(setup: F) -> (Receiver<Peer>, Addr<HyParViewActor>)
where
    F: FnOnce(&mut HyParViewActor) -> (),
{
    let (rx, mut hpv) = HyParViewActor::default();
    setup(&mut hpv);
    let addr = Arbiter::start(|_| hpv);
    (rx, addr)
}

#[test]
fn allow_inspections() {
    let system = System::new("test");
    let (_, addr) = start_hyparview(|_| {});
    let req: Request<HyParViewActor, Inspect> = addr.send(Inspect);

    assert_eq!(
        req.wait().unwrap().unwrap(),
        Views {
            active_view: BoundedSet::new(Config::default().max_active_view_size),
            passive_view: BoundedSet::new(Config::default().max_passive_view_size)
        }
    );
}
