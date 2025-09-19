use crate::simulation::Simulator;
use std::time::{Duration, SystemTime};

pub mod input;
pub mod output;

/// Configuration for the Real-Time Root Coordinator.
#[derive(Debug, Clone, Copy)]
pub struct RootCoordinatorConfig {
    /// Time scale factor for the simulation.
    ///
    /// A value of 1.0 means real-time, values greater than 1.0 speed up
    /// the simulation, and values less than 1.0 slow it down.
    pub time_scale: f64,

    /// Maximum allowable jitter for the simulation.
    ///
    /// If the jitter exceeds this value, the simulation will panic.
    /// A value of `None` means no jitter check.
    pub max_jitter: Option<Duration>,

    /// Capacity of the output event queue.
    ///
    /// A value of `None` means no output queue.
    pub output_capacity: Option<usize>,

    /// Buffer size of the input event queue.
    ///
    /// A value of `None` means no input queue.
    pub input_buffer: Option<usize>,

    /// Optional time window for batching input events.
    ///
    /// A value of `None` means no batching.
    pub input_window: Option<Duration>,
}

impl RootCoordinatorConfig {
    /// Creates a new `RootCoordinatorConfig` with the specified parameters.
    #[inline]
    pub fn new(
        time_scale: f64,
        max_jitter: Option<Duration>,
        output_capacity: Option<usize>,
        input_buffer: Option<usize>,
        input_window: Option<Duration>,
    ) -> Self {
        Self {
            time_scale,
            max_jitter,
            output_capacity,
            input_buffer,
            input_window,
        }
    }
}

impl Default for RootCoordinatorConfig {
    fn default() -> Self {
        Self {
            time_scale: 1.,
            max_jitter: None,
            output_capacity: None,
            input_buffer: None,
            input_window: None,
        }
    }
}

/// Real-Time Root Coordinator for managing the simulation of a DEVS model in real-time.
#[derive(Debug)]
pub struct RootCoordinator<T> {
    /// The DEVS model being simulated.
    model: T,
    /// Time scale factor for the simulation.
    time_scale: f64,
    /// Maximum allowable jitter for the simulation.
    max_jitter: Option<Duration>,
    /// Output event queue. `None` if no output queue is configured.
    output_queue: Option<output::OutputQueue>,
    /// Input event queue. `None` if no input queue is configured.
    input_queue: Option<input::InputQueue>,
}

impl<T: Simulator> RootCoordinator<T> {
    /// Creates a new `RootCoordinator` with the provided DEVS model and configuration.
    #[inline]
    pub fn new(model: T, config: RootCoordinatorConfig) -> Self {
        let output_queue = config.output_capacity.map(output::OutputQueue::new);
        let input_queue = config
            .input_buffer
            .map(|buffer| input::InputQueue::new(buffer, config.input_window));
        Self {
            model,
            time_scale: config.time_scale,
            max_jitter: config.max_jitter,
            output_queue,
            input_queue,
        }
    }

    /// Spawns a handler for managing input and output events.
    ///
    /// It returns a vector of `JoinHandle`s for the spawned tasks. It is the caller's
    /// responsibility to manage the lifecycle of these tasks.
    #[inline]
    pub fn spawn_handler<H: Handler>(&mut self, handler: H) -> Vec<tokio::task::JoinHandle<()>> {
        let input_tx = self
            .input_queue
            .as_ref()
            .map(|input_handler| input_handler.subscribe());
        let output_rx = self
            .output_queue
            .as_ref()
            .map(|output_handler| output_handler.subscribe());
        // Safety: spawning from the spawn_handler method
        unsafe { Handler::spawn(handler, input_tx, output_rx) }
    }

    /// Runs the Real-Time simulation until the specified stop time.
    #[inline]
    pub async fn simulate(mut self, t_stop: f64) {
        tracing::info!("starting simulation");

        let mut last_vt = 0.;
        let mut next_vt = f64::min(self.model.start(last_vt), t_stop);

        let start_rt = SystemTime::now();
        let mut last_rt = start_rt;

        while last_vt < t_stop {
            tracing::debug!("simulation step from vt={last_vt} to {next_vt}");
            // Compute corresponding next_rt (None means infinity)
            let duration = match next_vt {
                f64::INFINITY => Duration::MAX,
                _ => Duration::from_secs_f64((next_vt - last_vt) * self.time_scale),
            };
            let next_rt = last_rt.checked_add(duration);

            // Use input handler if available, otherwise sleep
            match &mut self.input_queue {
                Some(input_handler) => input_handler.wait_event(next_rt, &self.model).await,
                None => {
                    let duration = match next_rt {
                        Some(t_next) => {
                            t_next.duration_since(SystemTime::now()).unwrap_or_default()
                        }
                        None => Duration::MAX,
                    };
                    tracing::debug!("sleeping for {duration:?}");
                    tokio::time::sleep(duration).await
                }
            };

            // Check the jitter and update last_rt and last_vt
            let t = SystemTime::now();
            let jitter = match next_rt {
                Some(next_rt) => t.duration_since(next_rt).ok(),
                None => None,
            };
            match jitter {
                Some(jitter) => {
                    tracing::debug!("jitter of {jitter:?}");
                    // t >= next_rt, check for the jitter
                    if let Some(max_jitter) = self.max_jitter {
                        if jitter > max_jitter {
                            tracing::error!("jitter too high: {jitter:?}");
                            panic!("jitter too high: {jitter:?}");
                        }
                    }
                    last_rt = next_rt.unwrap();
                    last_vt = next_vt;
                }
                None => {
                    // t < next_rt
                    last_rt = t;
                    let duration = last_rt.duration_since(start_rt).unwrap();
                    last_vt = duration.as_secs_f64() / self.time_scale;
                }
            };
            tracing::debug!("simulation step reached vt={last_vt}");

            if last_vt >= next_vt {
                self.model.collection(last_vt);
                if let Some(output_handler) = &self.output_queue {
                    output_handler.propagate_output(&self.model);
                }
            } else if unsafe { self.model.get_component().is_input_empty() } {
                tracing::warn!("spurious external transition. Ignoring.");
                continue;
            }
            next_vt = f64::min(self.model.transition(last_vt), t_stop);
            tracing::debug!("next simulation vt = {next_vt}");
        }
        self.model.stop(t_stop);

        tracing::info!("simulation completed");
    }
}

pub trait Handler {
    /// Spawns a handler for managing input and output events.
    ///
    /// # Safety
    ///
    /// Do not call this method directly. Use [`RootCoordinator::spawn_handler`] instead.
    unsafe fn spawn(
        self,
        input_tx: Option<input::InputSender>,
        output_rx: Option<output::OutputReceiver>,
    ) -> Vec<tokio::task::JoinHandle<()>>;
}
