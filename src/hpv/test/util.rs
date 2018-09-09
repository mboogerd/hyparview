extern crate actix;
extern crate futures;
extern crate futures_channel;

use self::actix::dev::MessageResponse;
use self::actix::prelude::*;
use hpv::HpvMsg;
use hpv::HyParViewActor;
use hpv::Peer;
use std::sync::mpsc::Receiver;
use util::channelactor::ChannelActor;
use util::channelactor::TrySendResult;

/// Creates a default HyParViewActor that allows its configuration to be overridden
pub fn new_hyparview<F>(setup: F) -> (Receiver<Peer>, HyParViewActor)
where
    F: FnOnce(&mut HyParViewActor) -> (),
{
    let (rx, mut hpv) = HyParViewActor::default();
    setup(&mut hpv);
    (rx, hpv)
}

pub fn start_hyparview<F>(setup: F) -> (Receiver<Peer>, Addr<HyParViewActor>)
where
    F: FnOnce(&mut HyParViewActor) -> (),
{
    let (rx, hpv) = new_hyparview(setup);
    (rx, Arbiter::start(|_| hpv))
}

pub fn mock_hpv_peer() -> (Receiver<HpvMsg>, Peer) {
    let (recv, recp): (Receiver<HpvMsg>, Recipient<HpvMsg>) = mock_recipient();
    (recv, recp.into())
}

pub fn mock_recipient<T>() -> (Receiver<T>, Recipient<T>)
where
    T: 'static + Send + Message,
    <T as Message>::Result: Send + MessageResponse<ChannelActor<T>, T> + From<TrySendResult<T>>,
{
    let (rx, ca): (Receiver<T>, ChannelActor<T>) = ChannelActor::new();
    let recipient = Arbiter::start(|_| ca).recipient::<T>();
    (rx, recipient)
}
