extern crate actix;
extern crate futures;
extern crate futures_channel;

use hpv::Config;
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::result::Result;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use util::channelactor::TrySendResult;

mod util;
pub use self::util::*;

const TIMEOUT: Duration = Duration::from_millis(10);

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

#[cfg(test)]
mod inspect;

#[cfg(test)]
mod join;

#[cfg(test)]
mod passive_view;

#[cfg(test)]
mod active_view;

#[cfg(test)]
mod loquat;
