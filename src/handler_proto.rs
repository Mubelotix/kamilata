use crate::prelude::*;

/// Builder for a [KamilataHandler]
pub struct KamilataHandlerBuilder<const N: usize, D: Document<N>> {
    our_peer_id: PeerId,
    db: Arc<Db<N, D>>,
}

impl<const N: usize, D: Document<N>> IntoConnectionHandler for KamilataHandlerBuilder<N, D> {
    type Handler = KamilataHandler<N, D>;

    fn into_handler(self, remote_peer_id: &PeerId, _endpoint: &ConnectedPoint) -> Self::Handler {
        KamilataHandler::new(self.our_peer_id, remote_peer_id.to_owned(), self.db)
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        upgrade::EitherUpgrade::A(KamilataProtocolConfig::new()) // Should be KamilataHandlerConfig
    }
}

impl<const N: usize, D: Document<N>> KamilataHandlerBuilder<N, D> {
    pub(crate) fn new(our_peer_id: PeerId, db: Arc<Db<N, D>>) -> KamilataHandlerBuilder<N, D> {
        KamilataHandlerBuilder {
            our_peer_id,
            db
        }
    }
}

use asynchronous_codec::{Framed, BytesMut};
use libp2p::kad::protocol::KadStreamSink;
use unsigned_varint::codec::UviBytes;

#[derive(Debug, Clone, Default)]
pub struct KamilataProtocolConfig {}

impl KamilataProtocolConfig {
    // TODO: remove because used too often
    pub fn new() -> KamilataProtocolConfig {
        KamilataProtocolConfig {}
    }
}

impl UpgradeInfo for KamilataProtocolConfig {
    type Info = &'static [u8];
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/kamilata/0.0.1")
    }
}

pub(crate) type KamInStreamSink<S> = KadStreamSink<S, ResponsePacket, RequestPacket>;
pub(crate) type KamOutStreamSink<S> = KadStreamSink<S, RequestPacket, ResponsePacket>;

impl<S> InboundUpgrade<S> for KamilataProtocolConfig
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Output = KamInStreamSink<S>;
    type Error = ioError;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: S, _: Self::Info) -> Self::Future {
        use protocol::{Parcel, Settings as ProtocolSettings};

        let mut codec = UviBytes::default();
        codec.set_max_len(5_000_000); // TODO: Change this value

        future::ok(
            Framed::new(socket, codec)
                .err_into()
                .with::<_, _, fn(_) -> _, _>(|response: ResponsePacket| {
                    let stream = response.into_stream(&ProtocolSettings::default()).map_err(|e| {
                        ioError::new(std::io::ErrorKind::Other, e.to_string()) // TODO: error handling
                    });
                    future::ready(stream)
                })
                .and_then::<_, fn(_) -> _>(|bytes: BytesMut| {
                    let request = RequestPacket::from_raw_bytes(&bytes, &ProtocolSettings::default()).map_err(|e| {
                        ioError::new(std::io::ErrorKind::Other, e.to_string()) // TODO: error handling
                    });
                    future::ready(request)
                }),
        )
    }
}

impl<S> OutboundUpgrade<S> for KamilataProtocolConfig
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Output = KamOutStreamSink<S>;
    type Error = ioError;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: S, _: Self::Info) -> Self::Future {
        use protocol::{Parcel, Settings as ProtocolSettings};

        let mut codec = UviBytes::default();
        codec.set_max_len(5_000_000); // TODO: Change this value

        future::ok(
            Framed::new(socket, codec)
                .err_into()
                .with::<_, _, fn(_) -> _, _>(|request: RequestPacket| {
                    let stream = request.into_stream(&ProtocolSettings::default()).map_err(|e| {
                        ioError::new(std::io::ErrorKind::Other, e.to_string()) // TODO error handling
                    });
                    future::ready(stream)
                })
                .and_then::<_, fn(_) -> _>(|bytes: BytesMut| {
                    let response = ResponsePacket::from_raw_bytes(&bytes, &ProtocolSettings::default()).map_err(|e| {
                        ioError::new(std::io::ErrorKind::Other, e.to_string()) // TODO error handling
                    });
                    future::ready(response)
                }),
        )
    }
}
