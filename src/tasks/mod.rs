mod filter_broadcaster;
mod request_handler;
mod filter_receiver;
mod search;
mod request_maker;
mod filter_updater;

pub(self) use crate::prelude::*;
pub(crate) use filter_broadcaster::*;
pub(crate) use request_handler::*;
pub(crate) use filter_receiver::*;
pub(crate) use search::*;
pub(crate) use request_maker::*;
pub(crate) use filter_updater::*;

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

