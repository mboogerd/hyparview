use super::super::proto;
use super::super::protobuf;
use super::super::protobuf::Message;
use super::super::unsigned_varint::codec;
use super::bytes::{BufMut, Bytes, BytesMut};
use super::futures::future;
use super::futures::future::{Future, FutureResult};
use super::futures::prelude::*;
use super::futures::stream::*;
use super::futures::sync::{mpsc, oneshot};
use super::libp2p_core::*;
use super::tokio_codec::{Decoder, Encoder, Framed};
use super::tokio_io::{AsyncRead, AsyncWrite};
use super::{HpvMsg, Peer};
use std::collections::HashMap;
use std::error::Error;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::iter;
use std::sync;
use std::sync::Arc;

pub struct HpvConfig {
    self_peer: Peer,
    my_addr_proto: proto::hpv::Peer,
    recv_peer: mpsc::Receiver<Peer>,
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

    type Output = HpvStreamSink;
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

type HpvStreamSink = Box<(
    Stream<Item = (), Error = IoError> + Send,
    Sink<SinkItem = (), SinkError = IoError> + Send,
)>;

fn hyparview_protocol<S>(socket: S, config: HpvConfig) -> HpvStreamSink
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    // Create a channel for this connection. The sender part should be propagated with each incoming
    // message, such that the message processor can communicate to the remote party
    let (send, recv): (mpsc::Sender<HpvMsg>, mpsc::Receiver<HpvMsg>) = channel(10);

    // Get a copy of our local network address in protobuf
    let my_addr_proto = config.my_addr_proto.clone();

    let framed = Framed::new(socket, codec::UviBytes::default())
        // Supposedly necessary?
        .from_err::<IoError>()
        // Transform outgoing HpvMsg messages to protobuf HpvMessage
        .with(move |request| -> Result<_, IoError> {
            let proto_struct = msg_to_proto(request, my_addr_proto);
            Ok(proto_struct.write_to_bytes().unwrap()) // TODO: error?
        })
        // Transform incoming HpvMessage protobuf requests to HpvMsg, including the sender part for
        // this connection, such that the handler can talk back
        .and_then(move |bytes| {
            let response = protobuf::parse_from_bytes(&bytes)?;
            proto_to_msg(response, hpv_recipient(send))
        });

    // Splitting seems necessary in order to be able to uniquely address the sink fragment of Framed?
    let (writer, reader) = framed.split();

    // All messages received back from the HpvMsg processing actor using the channel of _this_
    // connection should be forwarded to the remote party
    let sink = recv.map_err(|e| e.into()).forward(framed);

    // Get a reference to our singleton actor that should receive messages of the HyParView protocol
    let self_peer = config.self_peer.clone();
    // Send all messages received from the remote side to the message processor
    use super::actix;
    let source = reader.for_each(move |msg| {
        self_peer.recipient.do_send(msg).map_err(|e| match e {
            actix::prelude::SendError::Full(_) => IoError::new(
                IoErrorKind::WouldBlock,
                "Message can not be propagated because actor mailbox is full",
            ),
            actix::prelude::SendError::Closed(_) => IoError::new(
                IoErrorKind::ConnectionAborted,
                "Message could not be sent as the actor mailbox is closed",
            ),
        })
    });

    // What to return such that:
    // 1. We satisfy `Sized` constraint?
    // 2. The stream will actually start, supposedly a problem?
    Box::new((source, sink)) as HpvStreamSink
}

fn hpv_recipient(sender: mpsc::Sender<HpvMsg>) -> Peer {
    // TODO (Merlijn): Rewrite Peer to wrap new Tokio/future channels instead. (Messages on this channel should be relayed to the actor)
    // Ignore for now
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
