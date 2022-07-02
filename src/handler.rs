use libp2p::{swarm::{handler::{InboundUpgradeSend, OutboundUpgradeSend}, NegotiatedSubstream}, core::either::EitherOutput};

use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataHandlerIn {

}

#[derive(Debug)]
pub enum KamilataHandlerEvent {

}

enum InSubstreamState {
    WaitingForRequest,
    PendingSend(RequestPacket),
    PendingFlush,
}

enum OutSubstreamState {
    PendingSend(RequestPacket),
    PendingFlush,
    WaitingForResponse,
}

struct InSubstreamWithState {
    substream: KamInStreamSink<NegotiatedSubstream>,
    state: InSubstreamState,
}

struct OutSubstreamWithState {
    substream: KamOutStreamSink<NegotiatedSubstream>,
    state: OutSubstreamState,
}

impl OutSubstreamWithState {
    fn advance(self, cx: &mut Context<'_>) -> (
        Option<Self>,
        Option<ConnectionHandlerEvent<
            KamilataProtocolConfig,
            RequestPacket,
            KamilataHandlerEvent,
            ioError,
        >>,
        bool,
    ) {
        use OutSubstreamState::*;
        let OutSubstreamWithState { mut substream, state } = self;
        
        match state {
            PendingSend(request) => match Sink::poll_ready(Pin::new(&mut substream), cx) {
                Poll::Ready(Ok(())) => match Sink::start_send(Pin::new(&mut substream), request) {
                    Ok(()) => (Some(OutSubstreamWithState { substream, state: PendingFlush }), None, true),
                    Err(e) => todo!(),
                },
                Poll::Pending => (Some(OutSubstreamWithState { substream, state: PendingSend(request) }), None, false),
                Poll::Ready(Err(e)) => todo!(),
            }
            PendingFlush => match Sink::poll_flush(Pin::new(&mut substream), cx) {
                Poll::Ready(Ok(())) => (Some(OutSubstreamWithState { substream, state: WaitingForResponse }), None, true),
                Poll::Pending => todo!(),
                Poll::Ready(Err(e)) => todo!(),
            }
            WaitingForResponse => match Stream::poll_next(Pin::new(&mut substream), cx) {
                Poll::Ready(Some(Ok(response))) => todo!(),
                Poll::Pending => todo!(),
                Poll::Ready(None) => todo!(),
                Poll::Ready(Some(Err(e))) => todo!(),
            }
        }
    }
}

pub struct KamilataHandler {
    inbound_substreams: Vec<InSubstreamWithState>,
    outbound_substreams: Vec<OutSubstreamWithState>,
}

impl KamilataHandler {
    pub fn new() -> Self {
        KamilataHandler {
            inbound_substreams: Vec::new(),
            outbound_substreams: Vec::new(),
        }
    }
}

impl ConnectionHandler for KamilataHandler {
    type InEvent = KamilataHandlerIn;
    type OutEvent = KamilataHandlerEvent;
    type Error = ioError;
    type InboundProtocol = EitherUpgrade<KamilataProtocolConfig, DeniedUpgrade>;
    type OutboundProtocol = KamilataProtocolConfig;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = RequestPacket;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(KamilataProtocolConfig::new(), ()).map_upgrade(upgrade::EitherUpgrade::A)
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        _: Self::InboundOpenInfo,
    ) {
        let substream = match protocol {
            EitherOutput::First(s) => s,
            EitherOutput::Second(_void) => return,
        };

        // TODO: prevent DoS

        self.inbound_substreams.push(InSubstreamWithState {
            substream,
            state: InSubstreamState::WaitingForRequest,
        });
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        substream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        request: Self::OutboundOpenInfo,
    ) {
        self.outbound_substreams.push(OutSubstreamWithState {
            substream,
            state: OutSubstreamState::PendingSend(request),
        });
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        todo!()
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        error: libp2p::swarm::ConnectionHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgradeSend>::Error>,
    ) {
        todo!()
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        for n in (0..self.outbound_substreams.len()).rev() {
            let mut substream = self.outbound_substreams.swap_remove(n);

            loop {
                match substream.advance(cx)
                {
                    (Some(new_state), Some(event), _) => {
                        self.outbound_substreams.push(new_state);
                        return Poll::Ready(event);
                    }
                    (None, Some(event), _) => {
                        // if self.outbound_substreams.is_empty() {
                        //     self.keep_alive = KeepAlive::Until(Instant::now() + self.config.idle_timeout);
                        // }
                        return Poll::Ready(event);
                    }
                    (Some(new_state), None, false) => {
                        self.outbound_substreams.push(new_state);
                        break;
                    }
                    (Some(new_state), None, true) => {
                        substream = new_state;
                        continue;
                    }
                    (None, None, _) => {
                        break;
                    }
                }
            }
        }

        Poll::Pending
    }
}

pub struct KamilataHandlerProto {

}

impl IntoConnectionHandler for KamilataHandlerProto {
    type Handler = KamilataHandler;

    fn into_handler(self, remote_peer_id: &PeerId, endpoint: &ConnectedPoint) -> Self::Handler {
        KamilataHandler::new()
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        upgrade::EitherUpgrade::A(KamilataProtocolConfig::new()) // Should be KamilataHandlerConfig
    }
}

impl KamilataHandlerProto {
    pub fn new() -> KamilataHandlerProto {
        KamilataHandlerProto {}
    }
}