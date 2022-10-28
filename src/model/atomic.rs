use std::fmt::Debug;
use crate::{AsModel, Model};

impl<T: AsAtomic> AsModel for T {
    fn as_model(&self) -> &Model {
        self.get_model()
    }

    fn as_model_mut(&mut self) -> &mut Model {
        self.get_model_mut()
    }

    fn start_simulation(&mut self, t_start: f64) {
        self.set_clock(t_start, t_start + self.ta());
    }

    fn stop_simulation(&mut self, t_stop: f64) {
        self.set_clock(t_stop, f64::INFINITY);
    }

    fn lambda(&mut self, t: f64) {
        if t >= self.get_t_next() {
            AsAtomic::lambda(self);
        }
    }

    fn delta(&mut self, t: f64) {
        if !self.is_input_empty() {
            if t == self.get_t_next() {
                self.delta_conf();
            } else {
                let e = t - self.get_t_last();
                self.delta_ext(e);
            }
        } else if t == self.get_t_next() {
            self.delta_int();
        } else {
            return;
        }
        self.set_clock(t, t + self.ta())
    }
}

/// Interface for atomic DEVS models.
pub trait AsAtomic: Debug {
    fn get_model(&self) -> &Model;

    fn get_model_mut(&mut self) -> &mut Model;

    /// Output function of the atomic DEVS model.
    fn lambda(&self);

    /// Internal transition function of the atomic DEVS model.
    fn delta_int(&mut self);

    /// External transition function of the atomic DEVS model.
    /// `e` corresponds to the elapsed time since the last state transition of the model.
    fn delta_ext(&mut self, e: f64);

    /// Time advance function of the atomic DEVS model.
    fn ta(&self) -> f64;

    /// Confluent transition function of the atomic DEVS model.
    /// By default, it first triggers [`AsAtomic::delta_int`].
    /// Then, it triggers [`AsAtomic::delta_ext`] with elapsed time 0.
    fn delta_conf(&mut self) {
        self.delta_int();
        self.delta_ext(0.);
    }
}

/// Helper macro to implement the AsModel trait.
/// You can use this macro with any struct containing a field `model` of type [`Model`].
/// TODO try to use the derive stuff (it will be more elegant).
#[macro_export]
macro_rules! impl_atomic {
    ($($ATOMIC:ident),+) => {
        // use $crate::{AbstractSimulator, AsAtomic, AsModel};
        $(
            impl AsAtomic for $ATOMIC {
                fn get_model(&self) -> &Model { &self.model }
                fn get_model_mut(&mut self) -> &mut Model { &mut self.model }
                fn lambda(&self) { self.lambda(); }
                fn delta_int(&mut self) { self.delta_int() }
                fn delta_ext(&mut self, e: f64) { self. delta_ext(e) }
                fn ta(&self) -> f64 { self.ta() }
            }
        )+
    }
}
