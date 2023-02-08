//! This tasks sends a request through the handler and reports the response to a channel.

use super::*;

pub async fn request<const N: usize, D: Document<N>>(
    mut stream: KamOutStreamSink<NegotiatedSubstream>,
    request: RequestPacket,
    sender: OneshotSender<Option<ResponsePacket>>,
    our_peer_id: PeerId,
    remote_peer_id: PeerId
) -> HandlerTaskOutput {
    // Send request packet
    match stream.start_send_unpin(request) {
        Ok(()) => (),
        Err(e) => {
            sender.send(None).unwrap();
            return HandlerTaskOutput::None;
        }
    }
    if stream.flush().await.is_err() {
        sender.send(None).unwrap();
        return HandlerTaskOutput::None;
    }

    // Receive response packet
    let packet = match stream.next().await {
        Some(Ok(packet)) => packet,
        _ => {
            sender.send(None).unwrap();
            return HandlerTaskOutput::None;
        }
    };

    // Send results packet
    sender.send(Some(packet)).unwrap();
    
    HandlerTaskOutput::None
}

pub fn request_boxed<const N: usize, D: Document<N>>(
    stream: KamOutStreamSink<NegotiatedSubstream>,
    vals: Box<dyn std::any::Any + Send>
) -> Pin<Box<dyn Future<Output = HandlerTaskOutput> + Send>> {
    let vals: Box<(RequestPacket, OneshotSender<Option<ResponsePacket>>, PeerId, PeerId)> = vals.downcast().unwrap(); // TODO: downcast unchecked?
    request::<N, D>(stream, vals.0, vals.1, vals.2, vals.3).boxed()
}

pub fn pending_request<const N: usize, D: Document<N>>(
    request: RequestPacket,
    sender: OneshotSender<Option<ResponsePacket>>,
    our_peer_id: PeerId,
    remote_peer_id: PeerId,
) -> PendingHandlerTask<Box<dyn std::any::Any + Send>> {
    PendingHandlerTask {
        params: Box::new((request, sender, our_peer_id, remote_peer_id)),
        fut: request_boxed::<N, D>
    }
}

