use crate::modeling::{Atomic, Component, Coupled};
use crate::DynRef;
#[cfg(feature = "par_any")]
use rayon::prelude::*;
use std::ops::{Deref, DerefMut};

/// Interface for simulating DEVS models. All DEVS models must implement this trait.
pub trait Simulator: DynRef {
    /// Returns reference to inner [`Component`].
    fn get_component(&self) -> &Component;

    /// Returns mutable reference to inner [`Component`].
    fn get_component_mut(&mut self) -> &mut Component;

    /// Returns the name of the inner DEVS [`Component`].
    #[inline]
    fn get_name(&self) -> &str {
        self.get_component().get_name()
    }

    /// Returns the time for the last state transition of the inner DEVS [`Component`].
    #[inline]
    fn get_t_last(&self) -> f64 {
        self.get_component().get_t_last()
    }

    /// Returns the time for the next state transition of the inner DEVS [`Component`].
    #[inline]
    fn get_t_next(&self) -> f64 {
        self.get_component().get_t_next()
    }

    /// Sets the tine for the last and next state transitions of the inner DEVS [`Component`].
    #[inline]
    fn set_sim_t(&mut self, t_last: f64, t_next: f64) {
        self.get_component_mut().set_sim_t(t_last, t_next);
    }

    /// Removes all the messages from all the ports.
    #[inline]
    fn clear_ports(&mut self) {
        let component = self.get_component_mut();
        component.clear_input();
        component.clear_output()
    }

    /// It starts the simulation, setting the initial time to t_start.
    fn start(&mut self, t_start: f64);

    /// It stops the simulation, setting the last time to t_stop.
    fn stop(&mut self, t_stop: f64);

    /// Executes output functions and propagates messages according to ICs and EOCs.
    fn collection(&mut self, t: f64);

    /// Propagates messages according to EICs and executes model transition functions.
    fn transition(&mut self, t: f64);
}

impl<T: Atomic + DynRef> Simulator for T {
    #[inline]
    fn get_component(&self) -> &Component {
        Atomic::get_component(self)
    }

    #[inline]
    fn get_component_mut(&mut self) -> &mut Component {
        Atomic::get_component_mut(self)
    }

    #[inline]
    fn start(&mut self, t_start: f64) {
        Atomic::start(self);
        let ta = self.ta();
        self.set_sim_t(t_start, t_start + ta);
    }

    #[inline]
    fn stop(&mut self, t_stop: f64) {
        self.set_sim_t(t_stop, f64::INFINITY);
        Atomic::stop(self);
    }

    #[inline]
    fn collection(&mut self, t: f64) {
        if t >= self.get_t_next() {
            Atomic::lambda(self)
        }
    }

    fn transition(&mut self, t: f64) {
        let t_next = self.get_t_next();
        if !self.get_component().is_input_empty() {
            if t == t_next {
                Atomic::delta_conf(self);
            } else {
                let e = t - self.get_t_last();
                Atomic::delta_ext(self, e);
            }
        } else if t == t_next {
            Atomic::delta_int(self);
        } else {
            return;
        }
        let ta = Atomic::ta(self);
        self.set_sim_t(t, t + ta);
    }
}

impl Simulator for Coupled {
    #[inline]
    fn get_component(&self) -> &Component {
        &self.component
    }

    #[inline]
    fn get_component_mut(&mut self) -> &mut Component {
        &mut self.component
    }

    /// Iterates over all the subcomponents to call their [`Simulator::start`] method and obtain the next simulation time.
    /// If the feature `par_start` is activated, the iteration is parallelized.
    fn start(&mut self, t_start: f64) {
        #[cfg(feature = "par_start")]
        let iter = self.components.par_iter_mut();
        #[cfg(not(feature = "par_start"))]
        let iter = self.components.iter_mut();
        // we obtain the minimum next time of all the subcomponents
        let t_next = iter
            .map(|c| {
                c.start(t_start);
                c.get_t_next()
            })
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(f64::INFINITY);
        // and set the inner component's last and next times
        self.set_sim_t(t_start, t_next);

        #[cfg(feature = "par_xic")]
        self.build_par_xics();
        #[cfg(feature = "par_eoc")]
        self.build_par_eocs();
    }

    /// Iterates over all the subcomponents to call their [`Simulator::stop`] method and obtain the next simulation time.
    /// If the feature `par_stop` is activated, the iteration is parallelized.
    #[inline]
    fn stop(&mut self, t_stop: f64) {
        #[cfg(feature = "par_stop")]
        let iter = self.components.par_iter_mut();
        #[cfg(not(feature = "par_stop"))]
        let iter = self.components.iter_mut();
        iter.for_each(|c| c.stop(t_stop));
        // we set the inner component's last and next times accordingly
        self.set_sim_t(t_stop, f64::INFINITY);
    }

    /// Iterates over all the subcomponents to call their [`Simulator::collection`] method.
    /// If the feature `par_collection` is activated, the iteration is parallelized.
    /// Then, it iterates over all the EOCs and propagates messages accordingly.
    /// If the feature `par_eoc` is activated, the iteration is parallelized.
    fn collection(&mut self, t: f64) {
        if t >= self.get_t_next() {
            #[cfg(feature = "par_collection")]
            let iter = self.components.par_iter_mut();
            #[cfg(not(feature = "par_collection"))]
            let iter = self.components.iter_mut();
            iter.for_each(|c| c.collection(t));

            #[cfg(feature = "par_eoc")]
            self.par_eocs.par_iter().for_each(|coups| {
                for &i in coups.iter() {
                    let (port_to, port_from) = &self.eocs[i];
                    unsafe { port_from.propagate(&**port_to) };
                }
            });
            #[cfg(not(feature = "par_eoc"))]
            self.eocs.iter().for_each(|(port_to, port_from)| {
                // Safety: coupled model propagating messages
                unsafe { port_from.propagate(&**port_to) };
            });
        }
    }

    /// Iterates over all the EICs and ICs and propagates messages accordingly.
    /// If the feature `par_xic` is activated, the iteration is parallelized.
    /// Then, itterates over all the subcomponents to:
    /// 1. Call their [`Simulator::transition`] method
    /// 2. Clear their ports
    /// 3. obtain their next simulation time.
    ///
    /// If the feature `par_transition` is activated, the iteration is parallelized.
    fn transition(&mut self, t: f64) {
        #[cfg(feature = "par_xic")]
        self.par_eics.par_iter().for_each(|coups| {
            for &i in coups.iter() {
                let (port_to, port_from) = &self.xics[i];
                unsafe { port_from.propagate(&**port_to) };
            }
        });
        #[cfg(feature = "par_xic")]
        self.par_ics.par_iter().for_each(|coups| {
            for &i in coups.iter() {
                let (port_to, port_from) = &self.xics[i];
                unsafe { port_from.propagate(&**port_to) };
            }
        });
        #[cfg(not(feature = "par_xic"))]
        self.xics.iter().for_each(|(port_to, port_from)| {
            // Safety: coupled model propagating messages
            unsafe { port_from.propagate(&**port_to) };
        });
        #[cfg(feature = "par_transition")]
        let iterator = self.components.par_iter_mut();
        #[cfg(not(feature = "par_transition"))]
        let iterator = self.components.iter_mut();
        let next_t = iterator
            .map(|c| {
                c.transition(t);
                c.clear_ports();
                c.get_t_next()
            })
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(f64::INFINITY);
        self.set_sim_t(t, next_t);
    }
}

/// Root coordinator for sequential simulations of DEVS models.
pub struct RootCoordinator<T>(T);

impl<T: Simulator> RootCoordinator<T> {
    /// Creates a new root coordinator from a DEVS-compliant model.
    pub fn new(model: T) -> Self {
        Self(model)
    }

    /// Runs a simulation for a given period of time.
    pub fn simulate_time(&mut self, t_end: f64) {
        self.start(0.);
        let mut t_next = self.get_t_next();
        while t_next < t_end {
            self.collection(t_next);
            self.transition(t_next);
            self.clear_ports();
            t_next = self.get_t_next();
        }
        self.stop(t_next);
    }

    /// Runs a simulation for a given number of simulation cycles.
    pub fn simulate_steps(&mut self, mut n_steps: usize) {
        self.start(0.);
        let mut t_next = self.get_t_next();
        while t_next < f64::INFINITY && n_steps > 0 {
            self.collection(t_next);
            self.transition(t_next);
            self.clear_ports();
            t_next = self.get_t_next();
            n_steps -= 1;
        }
        self.stop(t_next);
    }
}

impl<T> Deref for RootCoordinator<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RootCoordinator<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
