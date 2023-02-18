mod filter_broadcaster;
mod request_handler;
mod filter_receiver;
mod search;
mod request_maker;
mod routing_init;

pub(self) use crate::prelude::*;
pub(crate) use filter_broadcaster::*;
pub(crate) use request_handler::*;
pub(crate) use filter_receiver::*;
pub(crate) use search::*;
pub(crate) use request_maker::*;
pub(crate) use routing_init::*;

pub struct HandlerTask {
    pub fut: BoxFuture<'static, HandlerTaskOutput>,
    pub name: &'static str,
}

/// Task owned and ran by an [handler](ConnectionHandler)
pub struct PendingHandlerTask<T> {
    pub params: T,
    #[allow(clippy::type_complexity)]
    pub fut: fn(KamOutStreamSink<NegotiatedSubstream>, T) -> BoxFuture<'static, HandlerTaskOutput>,
    pub name: &'static str,
}

/// Output of a [HandlerTask]
pub enum HandlerTaskOutput {
    None,
    Disconnect(DisconnectPacket),
    SetTask {
        tid: u32,
        task: HandlerTask,
    },
    NewPendingTask {
        tid: Option<u32>,
        pending_task: PendingHandlerTask<Box<dyn std::any::Any + Send>>,
    },
    Many(Vec<HandlerTaskOutput>),
}

impl HandlerTaskOutput {
    pub fn into_vec(self) -> Vec<HandlerTaskOutput> {
        match self {
            HandlerTaskOutput::None => Vec::new(),
            HandlerTaskOutput::Disconnect(_) => vec![self],
            HandlerTaskOutput::SetTask {..} => vec![self],
            HandlerTaskOutput::NewPendingTask {..} => vec![self],
            HandlerTaskOutput::Many(outputs) => outputs,
        }
    }
}

/// Task owned and ran by the [behaviour](NetworkBehaviour)
pub type Task = Pin<Box<dyn Future<Output = TaskOutput> + Send + Sync + 'static>>;

/// Output of a [Task]
pub enum TaskOutput {
    None,
}
