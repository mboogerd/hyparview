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
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::result::Result;
use std::sync::mpsc::Receiver;
use std::time::Duration;

// Test utilities

fn start_hyparview<F>(setup: F) -> (Receiver<Peer>, Addr<HyParViewActor>)
where
    F: FnOnce(&mut HyParViewActor) -> (),
{
    let (rx, mut hpv) = HyParViewActor::default();
    setup(&mut hpv);
    let addr = Arbiter::start(|_| hpv);
    (rx, addr)
}

impl<M> From<TrySendResult<M>> for Result<(), io::Error> {
    fn from(try: TrySendResult<M>) -> Result<(), io::Error> {
        try.0
            .map(|_| ())
            .map_err(|_| io::Error::from(UnexpectedEof))
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

    assert!(
        rx.recv_timeout(Duration::from_millis(100))
            .expect("Timeout waiting for Views") == Views {
            active_view: BoundedSet::new(Config::default().max_active_view_size),
            passive_view: BoundedSet::new(Config::default().max_passive_view_size)
        }
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
