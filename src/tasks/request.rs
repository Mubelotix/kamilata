//! This tasks sends a request through the handler and reports the response to a channel.

use super::*;

pub async fn request<const N: usize, D: Document<N>>(
    mut stream: KamOutStreamSink<NegotiatedSubstream>,
    request: RequestPacket,
    sender: OneshotSender<ResponsePacket>,
    our_peer_id: PeerId,
    remote_peer_id: PeerId
) -> HandlerTaskOutput {
    // Send request packet
    stream.start_send_unpin(request).unwrap();
    stream.flush().await.unwrap();

    // Receive response packet
    let packet = stream.next().await.unwrap().unwrap();

    // Send results packet
    sender.send(packet).unwrap();
    
    HandlerTaskOutput::None
}

pub fn request_boxed<const N: usize, D: Document<N>>(
    stream: KamOutStreamSink<NegotiatedSubstream>,
    vals: Box<dyn std::any::Any + Send>
) -> Pin<Box<dyn Future<Output = HandlerTaskOutput> + Send>> {
    let vals: Box<(RequestPacket, OneshotSender<ResponsePacket>, PeerId, PeerId)> = vals.downcast().unwrap(); // TODO: downcast unchecked?
    request::<N, D>(stream, vals.0, vals.1, vals.2, vals.3).boxed()
}

pub fn pending_request<const N: usize, D: Document<N>>(
    request: RequestPacket,
    sender: OneshotSender<ResponsePacket>,
    our_peer_id: PeerId,
    remote_peer_id: PeerId,
) -> PendingHandlerTask<Box<dyn std::any::Any + Send>> {
    PendingHandlerTask {
        params: Box::new((request, sender, our_peer_id, remote_peer_id)),
        fut: request_boxed::<N, D>
    }
}

