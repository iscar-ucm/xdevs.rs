use crate::simulation::Simulator;
use crate::Event;
use std::sync::Arc;
use tokio::sync::broadcast::{channel, Receiver, Sender};

/// Asynchronous receiver for output events.
pub type OutputReceiver = Receiver<Arc<Event>>;

/// Output event queue.
#[derive(Debug)]
pub struct OutputQueue(Sender<Arc<Event>>);

impl OutputQueue {
    /// Creates a new output queue with the specified capacity.
    #[inline]
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = channel(capacity);
        Self(tx)
    }

    /// Returns a clone of the output event receiver to allow receiving events from the queue.
    #[inline]
    pub fn subscribe(&self) -> OutputReceiver {
        self.0.subscribe()
    }

    /// Propagates output events from the provided component to the output queue.
    #[inline]
    pub fn propagate_output(&self, component: &impl Simulator) {
        // Safety: ejecting events from output handler
        for event in unsafe { component.get_component().eject() } {
            tracing::info!("propagating output event {event}");
            if self.0.send(Arc::new(event)).is_err() {
                tracing::warn!("all receivers have been dropped");
                break;
            }
        }
    }
}
