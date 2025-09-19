#[cfg(test)]
use super::SharedProbe;
use crate::modeling::*;
#[cfg(feature = "devstone_busy")]
use cpu_time::ThreadTime;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
struct State {
    n_internals: usize,
    n_externals: usize,
    n_events: usize,
    #[cfg(test)]
    probe: SharedProbe,
}

#[cfg(test)]
impl State {
    fn new(probe: SharedProbe) -> Self {
        Self {
            n_internals: 0,
            n_externals: 0,
            n_events: 0,
            probe,
        }
    }

    fn bulk_data(&self) {
        let mut x = self.probe.lock().unwrap();
        x.n_atomics += 1;
        x.n_internals += self.n_internals;
        x.n_externals += self.n_externals;
        x.n_events += self.n_events;
    }
}

pub(super) struct DEVStoneAtomic {
    component: Component,
    input: InPort<usize>,
    output: OutPort<usize>,
    int_delay: Option<Duration>,
    ext_delay: Option<Duration>,
    state: State,
    sigma: f64,
}

impl DEVStoneAtomic {
    pub(super) fn new(
        name: &str,
        int_delay: u64,
        ext_delay: u64,
        #[cfg(test)] probe: SharedProbe,
    ) -> Self {
        let mut component = Component::new(name);
        let input = component.add_in_port("input");
        let output = component.add_out_port("output");

        let int_delay = match int_delay > 0 {
            true => Some(Duration::from_millis(int_delay)),
            false => None,
        };
        let ext_delay = match ext_delay > 0 {
            true => Some(Duration::from_millis(ext_delay)),
            false => None,
        };

        #[cfg(not(test))]
        let state = State::default();
        #[cfg(test)]
        let state = State::new(probe);
        Self {
            component,
            input,
            output,
            int_delay,
            ext_delay,
            state,
            sigma: f64::INFINITY,
        }
    }

    #[inline]
    fn sleep(duration: &Option<Duration>) {
        if let Some(duration) = duration {
            #[cfg(feature = "devstone_busy")]
            {
                let now = ThreadTime::now();
                let mut x: u32 = 0;
                while now.elapsed() < *duration {
                    std::hint::black_box(&mut x);
                    x = x.wrapping_add(1);
                }
            }
            #[cfg(not(feature = "devstone_busy"))]
            std::thread::sleep(*duration);
        }
    }
}

impl Atomic for DEVStoneAtomic {
    #[inline]
    fn get_component(&self) -> &Component {
        &self.component
    }

    #[inline]
    fn get_component_mut(&mut self) -> &mut Component {
        &mut self.component
    }

    #[cfg(test)]
    #[inline]
    fn stop(&mut self) {
        self.state.bulk_data();
    }

    #[inline]
    fn lambda(&self) {
        // Safety: adding message on atomic model's output port at lambda
        unsafe { self.output.add_value(self.state.n_events) };
    }

    #[inline]
    fn delta_int(&mut self) {
        self.state.n_internals += 1;
        self.sigma = f64::INFINITY;
        Self::sleep(&self.int_delay);
    }

    fn delta_ext(&mut self, _e: f64) {
        self.state.n_externals += 1;
        // Safety: reading messages on atomic model's input port at delta_ext
        self.state.n_events += unsafe { self.input.get_values() }.len();
        self.sigma = 0.;
        Self::sleep(&self.ext_delay);
    }

    #[inline]
    fn ta(&self) -> f64 {
        self.sigma
    }
}
