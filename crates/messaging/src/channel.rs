use localmessenger_transport::{TransportConnection, TransportError, TransportFrame};
use std::future::Future;
use std::pin::Pin;

pub trait FrameChannel: Send + Sync {
    fn send_frame<'a>(
        &'a self,
        frame: &'a TransportFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + 'a>>;
    fn receive_frame<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<TransportFrame, TransportError>> + Send + 'a>>;
    fn close(&self, reason: &'static str);
}

impl FrameChannel for TransportConnection {
    fn send_frame<'a>(
        &'a self,
        frame: &'a TransportFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + 'a>> {
        Box::pin(async move { TransportConnection::send_frame(self, frame).await })
    }

    fn receive_frame<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<TransportFrame, TransportError>> + Send + 'a>> {
        Box::pin(async move { TransportConnection::receive_frame(self).await })
    }

    fn close(&self, reason: &'static str) {
        TransportConnection::close(self, reason);
    }
}

use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

#[derive(Default)]
struct QueueState {
    frames: VecDeque<TransportFrame>,
    closed: bool,
}

#[derive(Default)]
struct SharedQueue {
    state: Mutex<QueueState>,
    notify: Notify,
}

#[derive(Clone)]
pub struct InMemoryFrameChannel {
    inbound: Arc<SharedQueue>,
    outbound: Arc<SharedQueue>,
}

impl InMemoryFrameChannel {
    pub fn pair() -> (Self, Self) {
        let first = Arc::new(SharedQueue::default());
        let second = Arc::new(SharedQueue::default());
        (
            Self {
                inbound: first.clone(),
                outbound: second.clone(),
            },
            Self {
                inbound: second,
                outbound: first,
            },
        )
    }
}

impl FrameChannel for InMemoryFrameChannel {
    fn send_frame<'a>(
        &'a self,
        frame: &'a TransportFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + 'a>> {
        Box::pin(async move {
            let mut state = self.outbound.state.lock().await;
            if state.closed {
                return Err(TransportError::ConnectionClosed);
            }
            state.frames.push_back(frame.clone());
            drop(state);
            self.outbound.notify.notify_one();
            Ok(())
        })
    }

    fn receive_frame<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<TransportFrame, TransportError>> + Send + 'a>> {
        Box::pin(async move {
            loop {
                let notified = {
                    let mut state = self.inbound.state.lock().await;
                    if let Some(frame) = state.frames.pop_front() {
                        return Ok(frame);
                    }
                    if state.closed {
                        return Err(TransportError::ConnectionClosed);
                    }
                    self.inbound.notify.notified()
                };
                notified.await;
            }
        })
    }

    fn close(&self, _reason: &'static str) {
        if let Ok(mut state) = self.inbound.state.try_lock() {
            state.closed = true;
        }
        self.inbound.notify.notify_waiters();
        if let Ok(mut state) = self.outbound.state.try_lock() {
            state.closed = true;
        }
        self.outbound.notify.notify_waiters();
    }
}
