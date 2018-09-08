extern crate actix;
extern crate futures;
extern crate futures_channel;

use self::actix::dev::MessageResponse;
use self::actix::prelude::*;
use self::channelactor::ChannelActor;
use self::channelactor::TrySendResult;
use bounded_set::BoundedSet;
use hpv::Config;
use hpv::HpvMsg;
use hpv::HyParViewActor;
use hpv::Peer;
use hpv::Views;
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::result::Result;
use std::sync::mpsc::Receiver;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_millis(10);

// === Utility functions ===

/// Creates a default HyParViewActor that allows its configuration to be overridden
fn new_hyparview<F>(setup: F) -> (Receiver<Peer>, HyParViewActor)
where
    F: FnOnce(&mut HyParViewActor) -> (),
{
    let (rx, mut hpv) = HyParViewActor::default();
    setup(&mut hpv);
    (rx, hpv)
}

fn start_hyparview<F>(setup: F) -> (Receiver<Peer>, Addr<HyParViewActor>)
where
    F: FnOnce(&mut HyParViewActor) -> (),
{
    let (rx, hpv) = new_hyparview(setup);
    (rx, Arbiter::start(|_| hpv))
}

impl<M> From<TrySendResult<M>> for Result<(), io::Error> {
    fn from(try: TrySendResult<M>) -> Result<(), io::Error> {
        try.0
            .map(|_| ())
            .map_err(|_| io::Error::from(UnexpectedEof))
    }
}

trait Expectation<T> {
    fn recv_msg(&self, timeout: Duration) -> T;
    fn expect_msg(&self, timeout: Duration, msg: T)
    where
        T: Eq + Debug;
    fn expect_no_msg(&self, timeout: Duration)
    where
        T: Debug;
}

impl<T> Expectation<T> for Receiver<T> {
    fn recv_msg(&self, timeout: Duration) -> T {
        self.recv_timeout(timeout)
            .expect("Timeout waiting for Message")
    }
    fn expect_msg(&self, timeout: Duration, msg: T)
    where
        T: Eq + Debug,
    {
        assert_eq!(self.recv_msg(timeout), msg);
    }
    fn expect_no_msg(&self, timeout: Duration)
    where
        T: Debug,
    {
        self.recv_timeout(timeout)
            .expect_err("Received message while not expecting one");
    }
}

fn mock_recipient<T>() -> (Receiver<T>, Recipient<T>)
where
    T: 'static + Send + Message,
    <T as Message>::Result: Send + MessageResponse<ChannelActor<T>, T> + From<TrySendResult<T>>,
{
    let (rx, ca): (Receiver<T>, ChannelActor<T>) = ChannelActor::new();
    let recipient = Arbiter::start(|_| ca).recipient::<T>();
    (rx, recipient)
}

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

pub mod channelactor;

#[cfg(test)]
mod join;

#[cfg(test)]
mod passive_view;

#[cfg(test)]
mod active_view;

#[cfg(test)]
mod loquat;
