use super::super::proto;
use super::super::proto::hpv::*;
use super::super::protobuf;
use super::super::protobuf::Message;
use super::super::unsigned_varint::codec;
use super::bytes::Bytes;
use super::futures::future;
use super::futures::future::{Future, FutureResult};
use super::futures::prelude::*;
use super::futures::stream;
use super::futures::stream::*; //{Forward, FromErr, MapErr, Repeat, SplitSink, SplitStream, Zip};
use super::futures::sync::mpsc;
use super::libp2p_core::*;
use super::tokio_codec::Framed;
use super::tokio_io::{AsyncRead, AsyncWrite};
use super::{HpvMsg, Peer};
use std::fmt;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::iter;

pub struct HpvConfig {
    self_peer: mpsc::Sender<HpvMsg>,
    my_addr_proto: proto::hpv::Peer,
    recv_peer: mpsc::Receiver<Peer>,
    mock_temp_peer: Peer,
}

impl<C, Maf> ConnectionUpgrade<C, Maf> for HpvConfig
where
    C: AsyncRead + AsyncWrite + Send + 'static,
{
    type NamesIter = iter::Once<(Bytes, Self::UpgradeIdentifier)>;
    type UpgradeIdentifier = ();

    #[inline]
    fn protocol_names(&self) -> Self::NamesIter {
        iter::once(("/demograph/hyparview/1.0.0".into(), ()))
    }

    type Output = HpvFuture;
    type MultiaddrFuture = Maf;
    type Future = FutureResult<(Self::Output, Self::MultiaddrFuture), IoError>;

    #[inline]
    fn upgrade(
        self,
        incoming: C,
        _: Self::UpgradeIdentifier,
        _: Endpoint,
        remote_addr: Maf,
    ) -> Self::Future {
        future::ok((hyparview_protocol(incoming, self), remote_addr))
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct HpvFuture {
    inner: Box<Future<Item = (), Error = IoError> + Send>,
}

impl Future for HpvFuture {
    type Item = ();
    type Error = IoError;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

impl fmt::Debug for HpvFuture {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("HpvFuture").finish()
    }
}

fn hyparview_protocol<S>(socket: S, config: HpvConfig) -> HpvFuture
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    // Create a channel for this connection. The sender part should be propagated with each incoming
    // message, such that the message processor can communicate to the remote party
    let (send, recv): (mpsc::Sender<HpvMsg>, mpsc::Receiver<HpvMsg>) = mpsc::channel(10);

    let (proto_writer, proto_reader) = Framed::new(socket, codec::UviBytes::<Vec<u8>>::default())
        .from_err::<IoError>()
        .split();

    // Transform outgoing HpvMsg messages to protobuf HpvMessage
    let hpv_writer =
        proto_writer.with::<_, fn((_, _)) -> _, _>(|(request, self_addr)| -> Result<_, IoError> {
            let proto_struct = msg_to_proto(request, self_addr);
            Ok(proto_struct.write_to_bytes().unwrap()) // TODO: error?
        });

    // Get a copy of our local network address in protobuf
    // FIXME: derive Peer from mpsc::Sender
    let mock_peer = stream::repeat(config.mock_temp_peer);

    // Transform incoming HpvMessage protobuf requests to HpvMsg, including the sender part for
    // this connection, such that the handler can talk back
    let hpv_reader = proto_reader.zip(mock_peer).and_then(|(bytes, remote)| {
        let response = protobuf::parse_from_bytes(&bytes)?;
        proto_to_msg(response, remote)
    });

    // All messages received back from the HpvMsg processing actor using the channel of _this_
    // connection should be forwarded to the remote party
    let my_addr_stream = stream::repeat(config.my_addr_proto);
    let forward = recv
        .zip(my_addr_stream)
        .map_err(|e| IoError::new(IoErrorKind::UnexpectedEof, "Channel closed"))
        .forward(hpv_writer);

    // Get a reference to our singleton actor that should receive messages of the HyParView protocol
    let self_peer = config.self_peer.clone();
    // Send all messages received from the remote side to the message processor
    //    let source = self_peer.send_all(hpv_reader);
    let source = hpv_reader
        .forward(self_peer.sink_map_err(|e| IoError::new(IoErrorKind::ConnectionAborted, "Boom")));

    // Construct a Future by aligning sink and source and selecting either
    let final_sink = forward.map(|_| ());
    let final_source = source.map(|_| ());
    let final_future = to_impl(
        final_source
            .select(final_sink)
            .map(|_| ())
            // TODO: Try holding on to error cause
            .map_err(|e| IoError::new(IoErrorKind::UnexpectedEof, "Channel closed")),
    );

    HpvFuture {
        inner: Box::new(final_future) as Box<_>,
    }
}

fn to_impl<I, E, F>(f: F) -> impl Future<Item = I, Error = E>
where
    F: Future<Item = I, Error = E>,
{
    f
}

fn msg_to_proto(hpv_msg: HpvMsg, self_address: proto::hpv::Peer) -> proto::hpv::HpvMessage {
    let mut hpv_proto = proto::hpv::HpvMessage::new();
    hpv_proto.set_origin(self_address);

    match hpv_msg {
        HpvMsg::Inspect(_) => panic!("Tried to serialize `Inspect`"),
        HpvMsg::InitiateJoin(_) => panic!("Tried to serialize `InitiateJoin`"),
        HpvMsg::Disconnect(peer) => {
            hpv_proto.set_field_type(proto::hpv::MessageType::DISCONNECT);
        }
        HpvMsg::Join(peer) => {
            hpv_proto.set_field_type(proto::hpv::MessageType::JOIN);
        }
        HpvMsg::ForwardJoin {
            joining,
            forwarder,
            ttl,
        } => {
            hpv_proto.set_field_type(proto::hpv::MessageType::FORWARD_JOIN);
        }
        HpvMsg::Neighbour { peer, prio } => {
            hpv_proto.set_field_type(proto::hpv::MessageType::NEIGHBOUR);
        }
        HpvMsg::NeighbourReply { peer, accepted } => {
            hpv_proto.set_field_type(proto::hpv::MessageType::NEIGHBOUR_REPLY);
        }
        HpvMsg::Shuffle {
            id,
            origin,
            exchange,
            ttl,
        } => {
            hpv_proto.set_field_type(proto::hpv::MessageType::SHUFFLE);
        }
        HpvMsg::ShuffleReply(peer, exchange) => {
            hpv_proto.set_field_type(proto::hpv::MessageType::SHUFFLE_REPLY);
        }
    }

    hpv_proto
}

fn proto_to_msg(proto: proto::hpv::HpvMessage, hpv_recipient: Peer) -> Result<HpvMsg, IoError> {
    match proto.get_field_type() {
        proto::hpv::MessageType::DISCONNECT => Ok(HpvMsg::Disconnect(hpv_recipient)),
        proto::hpv::MessageType::JOIN => Ok(HpvMsg::Disconnect(hpv_recipient)),
        proto::hpv::MessageType::FORWARD_JOIN => Ok(HpvMsg::Disconnect(hpv_recipient)),
        proto::hpv::MessageType::NEIGHBOUR => Ok(HpvMsg::Disconnect(hpv_recipient)),
        proto::hpv::MessageType::NEIGHBOUR_REPLY => Ok(HpvMsg::Disconnect(hpv_recipient)),
        proto::hpv::MessageType::SHUFFLE => Ok(HpvMsg::Disconnect(hpv_recipient)),
        proto::hpv::MessageType::SHUFFLE_REPLY => Ok(HpvMsg::Disconnect(hpv_recipient)),
    }
}
