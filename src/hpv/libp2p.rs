use super::super::proto;
use super::super::proto::hpv::*;
use super::super::protobuf;
use super::super::protobuf::Message;
use super::super::unsigned_varint::codec;
use super::actix::dev;
use super::bytes::{Bytes, BytesMut};
use super::futures::future;
use super::futures::future::{Future, FutureResult};
use super::futures::prelude::*;
use super::futures::sink::{SinkMapErr, With};
use super::futures::stream;
use super::futures::stream::*; //{Forward, FromErr, MapErr, Repeat, SplitSink, SplitStream, Zip};
use super::futures::sync::mpsc;
use super::futures::*; //{Forward, FromErr, MapErr, Repeat, SplitSink, SplitStream, Zip};
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

    type Output = HpvFuture<C>;
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
pub struct HpvFuture<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    inner: NormalizedBridge<S>,
}

impl<S> Future for HpvFuture<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    type Item = ();
    type Error = IoError;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

//use super::futures::{Map, Select};
//use super::futures::{Forward, AndThen, Zip, SplitStream, FromErr};
//type HpvFutureInternal = Map<Select<Map<Forward<AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>>, fn((AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>)) {hpv::libp2p::forget::<(AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>)>}>, Map<Forward<MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>>, fn((MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>)) {hpv::libp2p::forget::<(MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>)>}>>, fn(((), SelectNext<Map<Forward<AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>>, fn((AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>)) {hpv::libp2p::forget::<(AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>)>}>, Map<Forward<MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>>, fn((MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>)) {hpv::libp2p::forget::<(MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>)>}>>)) {hpv::libp2p::forget::<((), SelectNext<Map<Forward<AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>>, fn((AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>)) {hpv::libp2p::forget::<(AndThen<Zip<SplitStream<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, Repeat<Peer, IoError>>, fn((BytesMut, Peer)) -> Result<hpv::message::HpvMsg, IoError>, Result<hpv::message::HpvMsg, IoError>>, sink::SinkMapErr<hpv::libp2p::RecipientWrapper, fn(hpv::actix::prelude::SendError<hpv::message::HpvMsg>) -> IoError>)>}>, Map<Forward<MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>>, fn((MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>)) {hpv::libp2p::forget::<(MapErr<Zip<sync::mpsc::Receiver<hpv::message::HpvMsg>, Repeat<proto::hpv::Peer, ()>>, fn(()) -> IoError>, sink::With<SplitSink<FromErr<Framed<S, codec::UviBytes<std::vec::Vec<u8>>>, IoError>>, (hpv::message::HpvMsg, proto::hpv::Peer), fn((hpv::message::HpvMsg, proto::hpv::Peer)) -> Result<std::vec::Vec<u8>, IoError>, Result<std::vec::Vec<u8>, IoError>>)>}>>)>}>

impl<S> fmt::Debug for HpvFuture<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("HpvFuture").finish()
    }
}

type ProtoSink<S> = SplitSink<FromErr<Framed<S, codec::UviBytes<Vec<u8>>>, IoError>>;
type ProtoSource<S> = SplitStream<FromErr<Framed<S, codec::UviBytes<Vec<u8>>>, IoError>>;
type ProtoToHpv<S> = With<
    ProtoSink<S>,
    (HpvMsg, proto::hpv::Peer),
    fn((HpvMsg, proto::hpv::Peer)) -> Result<Vec<u8>, IoError>,
    Result<Vec<u8>, IoError>,
>;
type HpvToProto<S> = stream::AndThen<
    Zip<ProtoSource<S>, stream::Repeat<Peer, IoError>>,
    fn((BytesMut, Peer)) -> Result<HpvMsg, IoError>,
    Result<HpvMsg, IoError>,
>;
type ZipReceiverPeer = stream::MapErr<
    Zip<mpsc::Receiver<HpvMsg>, stream::Repeat<proto::hpv::Peer, ()>>,
    fn(()) -> IoError,
>;
type ActorToProto<S> = future::Map<
    Forward<ZipReceiverPeer, ProtoToHpv<S>>,
    fn((ZipReceiverPeer, ProtoToHpv<S>)) -> (),
>;
type ActorSink = SinkMapErr<mpsc::Sender<HpvMsg>, fn(mpsc::SendError<HpvMsg>) -> IoError>;
type AndThenSomething<S> = stream::AndThen<
    Zip<ProtoSource<S>, stream::Repeat<Peer, IoError>>,
    fn((BytesMut, Peer)) -> Result<HpvMsg, IoError>,
    Result<HpvMsg, IoError>,
>;
type ProtoToActor<S> = future::Map<
    Forward<AndThenSomething<S>, ActorSink>,
    fn((AndThenSomething<S>, ActorSink)) -> (),
>;
type SelectActorProtoBridge<S> = future::Select<ProtoToActor<S>, ActorToProto<S>>;
type ForgetfulBridge<S> = future::Map<
    SelectActorProtoBridge<S>,
    fn(((), future::SelectNext<ProtoToActor<S>, ActorToProto<S>>)) -> (),
>;
type NormalizedBridge<S> = future::MapErr<
    ForgetfulBridge<S>,
    fn(
        (
            IoError,
            future::SelectNext<ProtoToActor<S>, ActorToProto<S>>,
        ),
    ) -> IoError,
>;

fn hyparview_protocol<S>(socket: S, config: HpvConfig) -> HpvFuture<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    // Create a channel for this connection. The sender part should be propagated with each incoming
    // message, such that the message processor can communicate to the remote party
    let (send, recv): (mpsc::Sender<HpvMsg>, mpsc::Receiver<HpvMsg>) = mpsc::channel(10);

    let (proto_writer, proto_reader): (ProtoSink<S>, ProtoSource<S>) =
        Framed::new(socket, codec::UviBytes::<Vec<u8>>::default())
            .from_err::<IoError>()
            .split();

    // Transform outgoing HpvMsg messages to protobuf HpvMessage
    let hpv_writer: ProtoToHpv<S> =
        proto_writer.with::<_, fn((_, _)) -> _, _>(|(request, self_addr)| -> Result<_, IoError> {
            let proto_struct = msg_to_proto(request, self_addr);
            Ok(proto_struct.write_to_bytes().unwrap()) // TODO: error?
        });

    // Get a copy of our local network address in protobuf
    // FIXME: derive Peer from mpsc::Sender
    let mock_peer: stream::Repeat<Peer, IoError> = repeat(config.mock_temp_peer);

    // Transform incoming HpvMessage protobuf requests to HpvMsg, including the sender part for
    // this connection, such that the handler can talk back
    let hpv_reader: HpvToProto<S> = proto_reader.zip(mock_peer).and_then::<fn((_, _)) -> _, _>(
        |(bytes, remote)| {
            let response = protobuf::parse_from_bytes(&bytes)?;
            proto_to_msg(response, remote)
        },
    );

    // All messages received back from the HpvMsg processing actor using the channel of _this_
    // connection should be forwarded to the remote party
    let my_addr_stream: stream::Repeat<proto::hpv::Peer, ()> = repeat(config.my_addr_proto);
    let actor_to_proto: ActorToProto<S> = recv
        .zip(my_addr_stream)
        .map_err::<_, fn(_) -> _>(|e| IoError::new(IoErrorKind::UnexpectedEof, "Channel closed"))
        .forward(hpv_writer)
        .map(forget);

    // Get a reference to our singleton actor that should receive messages of the HyParView protocol
    let self_peer: ActorSink = config
        .self_peer
        .clone()
        .sink_map_err::<fn(_) -> _, _>(|e| IoError::new(IoErrorKind::ConnectionAborted, "Boom"));
    // Send all messages received from the remote side to the message processor
    //    let source = hpv_reader.forward(
    //        RecipientWrapper(self_peer).sink_map_err::<fn(_) -> _, _>(|e| {
    //            IoError::new(IoErrorKind::ConnectionAborted, "Boom")
    //        }),
    //    );
    let proto_to_actor: ProtoToActor<S> = hpv_reader.forward(self_peer).map(forget);

    // Construct a Future by aligning sink and source and selecting either
    let bridge: SelectActorProtoBridge<S> = proto_to_actor.select(actor_to_proto);
    let forgetful_bridge: ForgetfulBridge<S> = bridge.map(forget);
    let normalized: NormalizedBridge<S> = forgetful_bridge.map_err(unexpected_eof);

    HpvFuture { inner: normalized }
}

fn forget<T>(_: T) {}

fn unexpected_eof<T>(_: T) -> IoError {
    IoError::new(IoErrorKind::UnexpectedEof, "Channel closed")
}

struct RecipientWrapper(dev::Recipient<HpvMsg>);

impl Sink for RecipientWrapper {
    type SinkItem = HpvMsg;
    type SinkError = dev::SendError<HpvMsg>;

    fn start_send(&mut self, item: HpvMsg) -> Result<AsyncSink<HpvMsg>, dev::SendError<HpvMsg>> {
        unimplemented!()
        //        match self.0.do_send(item) {
        //            Ok(()) => Ok(AsyncSink::Ready),
        //            Err(dev::SendError::Full(item)) => Ok(AsyncSink::NotReady(item)),
        //            Err(e @ dev::SendError::Closed(_)) => Result::from_error(e),
        //        }
    }

    fn poll_complete(&mut self) -> Result<Async<()>, dev::SendError<HpvMsg>> {
        unimplemented!()
    }

    fn close(&mut self) -> Result<Async<()>, dev::SendError<HpvMsg>> {
        unimplemented!()
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
