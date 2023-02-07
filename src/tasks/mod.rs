mod broadcast_local_filters;
mod handle_request;
mod receive_remote_filters;
mod search;
mod request;

pub(self) use crate::prelude::*;
pub use broadcast_local_filters::*;
pub use handle_request::*;
pub use receive_remote_filters::*;
pub use search::*;
pub use request::*;

pub type HandlerTask = BoxFuture<'static, HandlerTaskOutput>;

/// Task owned and ran by an [handler](ConnectionHandler)
pub struct PendingHandlerTask<T> {
    pub params: T,
    #[allow(clippy::type_complexity)]
    pub fut: fn(KamOutStreamSink<NegotiatedSubstream>, T) -> HandlerTask
}

/// Output of a [HandlerTask]
pub enum HandlerTaskOutput {
    None,
    Disconnect(DisconnectPacket),
    SetOutboundRefreshTask(HandlerTask)
}

/// Task owned and ran by the [behaviour](NetworkBehaviour)
pub type Task = Pin<Box<dyn Future<Output = TaskOutput> + Send + Sync + 'static>>;

/// Output of a [Task]
pub enum TaskOutput {
    None,
}

