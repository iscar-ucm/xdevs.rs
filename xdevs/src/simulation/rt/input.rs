use crate::{simulation::Simulator, Event};
use std::time::{Duration, SystemTime};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    time::timeout,
};

/// Asynchronous sender for input events.
pub type InputSender = Sender<Event>;

/// Input event queue with optional time window for batching events.
#[derive(Debug)]
pub struct InputQueue {
    /// Channel sender for input events.
    ///
    /// The sender can be cloned to allow multiple producers of input events.
    sender: Sender<Event>,
    /// Channel receiver for input events.
    receiver: Receiver<Event>,
    /// Optional time window for batching events.
    window: Option<Duration>,
}

impl InputQueue {
    /// Creates a new input queue with the specified buffer size and optional time window.
    #[inline]
    pub fn new(buffer: usize, window: Option<Duration>) -> Self {
        let (sender, receiver) = channel(buffer);
        Self {
            sender,
            receiver,
            window,
        }
    }

    /// Returns a clone of the input event sender to allow sending events to the queue.
    #[inline]
    pub fn subscribe(&self) -> InputSender {
        self.sender.clone()
    }

    /// Awaits asynchronously for an external event or until the next scheduled internal event time.
    ///
    /// A `t_next` of `None` indicates that there are no scheduled internal events.
    #[inline]
    pub async fn wait_event(&mut self, t_next: Option<SystemTime>, component: &impl Simulator) {
        let duration = match t_next {
            Some(t_next) => t_next.duration_since(SystemTime::now()).unwrap_or_default(),
            None => Duration::MAX,
        };
        tracing::debug!("waiting for external events or timeout of {duration:?}");

        self.inject_timeout(duration, component).await;
        // If there is a window, keep receiving events until the window expires
        if let Some(window) = self.window {
            let t_max = match t_next {
                Some(t_next) => std::cmp::min(t_next, SystemTime::now() + window),
                None => SystemTime::now() + window,
            };
            while let Ok(duration) = t_max.duration_since(SystemTime::now()) {
                tracing::debug!("waiting for external events within the window of {duration:?}");
                self.inject_timeout(duration, component).await;
            }
        }
    }

    /// Awaits asynchronously for an external event or until the specified duration elapses.
    #[inline]
    async fn inject_timeout(&mut self, duration: Duration, component: &impl Simulator) {
        match timeout(duration, self.receiver.recv()).await {
            Err(_) => {
                tracing::debug!("timeout expired without any external events");
            }
            Ok(None) => {
                tracing::error!("all senders have been dropped");
                unreachable!();
            }
            Ok(Some(event)) => {
                tracing::info!("injecting input event {event}");
                // Safety: injecting event from input handler
                match unsafe { component.get_component().inject(event) } {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("failed to inject event: {e:?}. Skipping.");
                    }
                }
            }
        }
    }
}
