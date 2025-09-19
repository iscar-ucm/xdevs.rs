use super::Component;

/// Interface for atomic DEVS models.
pub trait Atomic {
    /// Returns reference to inner component.
    fn get_component(&self) -> &Component;

    /// Returns mutable reference to inner component.
    fn get_component_mut(&mut self) -> &mut Component;

    /// Method for performing any operation before simulating. By default, it does nothing.
    #[inline]
    fn start(&mut self) {}

    /// Method for performing any operation after simulating. By default, it does nothing.
    #[inline]
    fn stop(&mut self) {}

    /// Output function of the atomic DEVS model.
    ///
    /// # Note
    ///
    /// This is the only method where implementers can safely manipulate their [`OutPort`](super::OutPort)s.
    fn lambda(&self);

    /// Internal transition function of the atomic DEVS model.
    fn delta_int(&mut self);

    /// External transition function of the atomic DEVS model.
    ///
    /// `e` corresponds to the elapsed time since the last state transition of the model.
    ///
    /// # Note
    ///
    /// This is the only method where implementers can safely manipulate their [`InPort`](super::InPort)s.
    fn delta_ext(&mut self, e: f64);

    /// Time advance function of the atomic DEVS model.
    fn ta(&self) -> f64;

    /// Confluent transition function of the atomic DEVS model.
    ///
    /// By default, it first triggers [`Atomic::delta_int`].
    /// Then, it triggers [`Atomic::delta_ext`] with the elapsed time set to `0.0`.
    #[inline]
    fn delta_conf(&mut self) {
        self.delta_int();
        self.delta_ext(0.);
    }
}
