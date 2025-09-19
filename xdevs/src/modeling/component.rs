use crate::modeling::port::{Bag, InPort, OutPort, Port, PortVal};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    UnknownPort,
    ValueParseError,
}

/// DEVS component. Models must comprise a component to fulfill the [`crate::simulation::Simulator`] trait.
pub struct Component {
    /// Name of the DEVS component.
    name: String,
    /// Time of the last component state transition.
    t_last: f64,
    /// Time for the next component state transition.
    t_next: f64,
    /// Input ports map. Keys are the port IDs, and values correspond to the index of the port in `in_ports`.
    in_map: HashMap<String, usize>,
    /// Output ports map. Keys are the port IDs, and values correspond to the index of the port in `out_ports`.
    out_map: HashMap<String, usize>,
    /// Input port set of the DEVS component (serialized for better performance).
    in_ports: Vec<Arc<dyn Port>>,
    /// Output port set of the DEVS component (serialized for better performance).
    out_ports: Vec<Arc<dyn Port>>,
}

impl Component {
    /// It creates a new component with the provided name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            t_last: 0.,
            t_next: f64::INFINITY,
            in_map: HashMap::new(),
            out_map: HashMap::new(),
            in_ports: Vec::new(),
            out_ports: Vec::new(),
        }
    }

    /// Returns name of the component.
    #[inline]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the time for the last component state transition.
    #[inline]
    pub fn get_t_last(&self) -> f64 {
        self.t_last
    }

    /// Returns the time for the next component state transition.
    #[inline]
    pub fn get_t_next(&self) -> f64 {
        self.t_next
    }

    /// Sets the time for the for the last and next component state transitions.
    #[inline]
    pub(crate) fn set_sim_t(&mut self, t_last: f64, t_next: f64) {
        self.t_last = t_last;
        self.t_next = t_next;
    }

    /// Adds a new input port of type `T` and returns a reference to it.
    ///
    /// It panics if there is already an input port with the same name.
    pub fn add_in_port<T: PortVal>(&mut self, name: &str) -> InPort<T> {
        if self.in_map.contains_key(name) {
            panic!("component already contains input port with the name provided");
        }
        self.in_map.insert(name.to_string(), self.in_ports.len());
        let bag = Bag::new();
        self.in_ports.push(bag.clone());
        InPort(bag)
    }

    /// Adds a new output port of type `T` and returns a reference to it.
    ///
    /// It panics if there is already an output port with the same name.
    pub fn add_out_port<T: PortVal>(&mut self, name: &str) -> OutPort<T> {
        if self.out_map.contains_key(name) {
            panic!("component already contains output port with the name provided");
        }
        self.out_map.insert(name.to_string(), self.out_ports.len());
        let bag = Bag::new();
        self.out_ports.push(bag.clone());
        OutPort(bag)
    }

    /// Returns `true` if all the input ports of the model are empty.
    ///
    /// # Safety
    ///
    /// This method can only be executed when implementing the [`crate::simulation::Simulator::transition`]
    /// method to determine whether to execute the internal, external, or confluent transition function.
    #[inline]
    pub(crate) unsafe fn is_input_empty(&self) -> bool {
        self.in_ports.iter().all(|p| p.is_empty())
    }

    /// Returns a reference to an input port with the given name.
    ///
    /// If the component does not have any input port with this name, it returns [`None`].
    #[inline]
    pub(crate) fn get_in_port(&self, port_name: &str) -> Option<Arc<dyn Port>> {
        let i = *self.in_map.get(port_name)?;
        Some(self.in_ports.get(i)?.clone())
    }

    /// Returns a reference to an output port with the given name.
    ///
    /// If the component does not have any output port with this name, it returns [`None`].
    #[inline]
    pub(crate) fn get_out_port(&self, port_name: &str) -> Option<Arc<dyn Port>> {
        let i = *self.out_map.get(port_name)?;
        Some(self.out_ports.get(i)?.clone())
    }

    /// Clears all the input ports of the model.
    ///
    /// # Safety
    ///
    /// This method can only be executed when implementing [`crate::simulation::Simulator::transition`] method.
    #[inline]
    pub(crate) unsafe fn clear_input(&mut self) {
        self.in_ports.iter_mut().for_each(|p| p.clear());
    }

    /// Clears all the output ports of the model.
    ///
    /// # Safety
    ///
    /// This method can only be executed when implementing [`crate::simulation::Simulator::transition`] method.
    #[inline]
    pub(crate) unsafe fn clear_output(&mut self) {
        self.out_ports.iter_mut().for_each(|p| p.clear());
    }

    /// Tries to inject a value into the input port with the given name.
    ///
    /// If the port does not exist or the value cannot be parsed, it returns an error.
    ///
    /// # Safety
    ///
    /// This method can only be executed in RT simulation by the input handler.
    #[cfg(feature = "rt")]
    pub(crate) unsafe fn inject(&self, event: crate::Event) -> Result<(), Error> {
        self.get_in_port(event.port())
            .ok_or(Error::UnknownPort)?
            .inject(event.value())
            .map_err(|_| Error::ValueParseError)
    }

    /// Ejects all the values from the output ports and returns them as an iterator of events.
    ///
    /// # Safety
    ///
    /// This method can only be executed in RT simulation by the output handler.
    #[cfg(feature = "rt")]
    pub(crate) unsafe fn eject(&self) -> impl Iterator<Item = crate::Event> + '_ {
        self.out_map.iter().flat_map(|(port_name, n)| {
            self.out_ports[*n]
                .eject()
                .into_iter()
                .map(move |value| crate::Event::new(port_name.to_string(), value))
        })
    }
}
