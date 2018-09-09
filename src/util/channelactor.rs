extern crate actix;
extern crate futures;
extern crate futures_channel;

use self::actix::dev::MessageResponse;
use self::actix::prelude::*;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};

///A ChannelActor wraps a `Sender` and relays all messages received to it
pub struct ChannelActor<M: 'static> {
    tx: Sender<M>,
}

impl<M> ChannelActor<M> {
    /// Wraps a `ChannelActor` around an existing `Sender`
    pub fn wrap(tx: Sender<M>) -> ChannelActor<M> {
        ChannelActor::<M> { tx: tx }
    }

    /// Creates a new `Receiver`/`Sender` pair and wraps a new ChannelActor around the `Sender`
    pub fn new() -> (Receiver<M>, ChannelActor<M>) {
        let (tx, rx) = channel();
        (rx, ChannelActor::wrap(tx))
    }
}

impl<M: 'static> Actor for ChannelActor<M> {
    type Context = Context<Self>;
}

pub struct TrySendResult<M>(pub Result<(), SendError<M>>);

impl<M: 'static, R: 'static> Handler<M> for ChannelActor<M>
where
    M: Message<Result = R>,
    <M as actix::Message>::Result: MessageResponse<ChannelActor<M>, M>,
    R: From<TrySendResult<M>>,
{
    type Result = M::Result;
    fn handle(&mut self, msg: M, _ctx: &mut Self::Context) -> M::Result {
        TrySendResult(self.tx.send(msg)).into()
    }
}

#[cfg(test)]
mod test {}
