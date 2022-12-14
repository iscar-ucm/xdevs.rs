use super::port::{AbstractPort, Input, Output, Port};
use crate::modeling::port::RawPort;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result};
use std::rc::Rc;

/// Generic DEVS component. Models must comprise a ['Component'] to fulfill the [`crate::simulation::Simulator`] trait.
#[derive(Debug)]
pub struct Component {
    /// name of the DEVS component.
    name: String,
    /// Time of the last component state transition.
    t_last: f64,
    /// Time for the next component state transition.
    t_next: f64,
    /// Keys are port IDs, and values are indices of input ports in [Component::input_ports].
    input_map: HashMap<String, usize>,
    /// Keys are port IDs, and values are indices of output ports in [Component::output_ports].
    output_map: HashMap<String, usize>,
    /// Input port set of the DEVS component.
    input_ports: Vec<Rc<dyn AbstractPort>>,
    /// Output port set of the DEVS component.
    output_ports: Vec<Rc<dyn AbstractPort>>,
}

impl Component {
    /// It creates a new component with the provided name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            t_last: 0.,
            t_next: f64::INFINITY,
            input_map: HashMap::new(),
            output_map: HashMap::new(),
            input_ports: Vec::new(),
            output_ports: Vec::new(),
        }
    }

    /// Returns name of the component.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the time for the last component state transition.
    pub fn get_t_last(&self) -> f64 {
        self.t_last
    }

    /// Returns the time for the next component state transition.
    pub fn get_t_next(&self) -> f64 {
        self.t_next
    }

    /// Sets the time for the for the last and next component state transitions.
    pub(crate) fn set_sim_t(&mut self, t_last: f64, t_next: f64) {
        self.t_last = t_last;
        self.t_next = t_next;
    }

    /// Adds a new input port of type [`Port<Input, T>`] and returns a reference to it.
    /// It panics if there is already an input port with the same name.
    pub fn add_in_port<T: 'static + Clone + Debug>(&mut self, name: &str) -> Port<Input, T> {
        if self.input_map.contains_key(name) {
            panic!("component already contains input port with the name provided");
        }
        let port = Rc::new(RawPort::<T>::new(name));
        self.input_map
            .insert(name.to_string(), self.input_ports.len());
        self.input_ports.push(port.clone());
        Port::<Input, T>::new(port)
    }

    /// Adds a new output port of type [`Port<Output, T>`] and returns a reference to it.
    /// It panics if there is already an output port with the same name.
    pub fn add_out_port<T: 'static + Clone + Debug>(&mut self, name: &str) -> Port<Output, T> {
        if self.output_map.contains_key(name) {
            panic!("component already contains output port with the name provided");
        }
        let port = Rc::new(RawPort::<T>::new(name));
        self.output_map
            .insert(name.to_string(), self.output_ports.len());
        self.output_ports.push(port.clone());
        Port::<Output, T>::new(port)
    }

    /// Returns true if all the input ports of the model are empty.
    pub fn is_input_empty(&self) -> bool {
        self.input_ports.iter().all(|p| p.is_empty())
    }

    /// Returns true if all the output ports of the model are empty.
    pub fn is_output_empty(&self) -> bool {
        self.output_ports.iter().all(|p| p.is_empty())
    }

    /// Returns a pointer to an input port with the given name.
    /// If the component does not have any input port with this name, it panics.
    pub(crate) fn get_in_port(&self, port_name: &str) -> Rc<dyn AbstractPort> {
        let i = *self
            .input_map
            .get(port_name)
            .expect("component does not contain input port with the name provided");
        self.input_ports
            .get(i)
            .expect("this code should never happen")
            .clone()
    }

    /// Returns a pointer to an output port with the given name.
    /// If the component does not have any output port with this name, it panics.
    pub(crate) fn get_out_port(&self, port_name: &str) -> Rc<dyn AbstractPort> {
        let i = *self
            .output_map
            .get(port_name)
            .expect("component does not contain output port with the name provided");
        self.output_ports
            .get(i)
            .expect("this code should never happen")
            .clone()
    }

    /// Clears all the input ports of the model.
    pub(crate) fn clear_input(&mut self) {
        self.input_ports.iter().for_each(|p| p.clear());
    }

    /// Clears all the output ports of the model.
    pub(crate) fn clear_output(&mut self) {
        self.output_ports.iter().for_each(|p| p.clear());
    }
}

impl Display for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "component does not contain input port with the name provided")]
    fn test_wrong_in_port() {
        Component::new("component_a").get_in_port("i32");
    }

    #[test]
    #[should_panic(expected = "component does not contain output port with the name provided")]
    fn test_wrong_out_port() {
        Component::new("component_a").get_out_port("i32");
    }

    #[test]
    #[should_panic(expected = "component already contains input port with the name provided")]
    fn test_duplicate_in_port() {
        let mut a = Component::new("component_a");
        a.add_in_port::<i32>("i32");
        assert_eq!(1, a.input_map.len());
        assert_eq!(0, a.output_map.len());
        a.add_in_port::<i32>("i32");
    }

    #[test]
    #[should_panic(expected = "component already contains output port with the name provided")]
    fn test_duplicate_out_port() {
        let mut a = Component::new("component_a");
        a.add_out_port::<i32>("i32");
        assert_eq!(0, a.input_map.len());
        assert_eq!(1, a.output_map.len());
        a.add_out_port::<i32>("i32");
    }

    #[test]
    fn test_component() {
        let mut a = Component::new("component_a");
        let in_i32 = a.add_in_port::<i32>("i32");
        let out_i32 = a.add_out_port::<i32>("i32");
        let out_f64 = a.add_out_port::<f64>("f64");

        assert_eq!("component_a", a.name);
        assert_eq!(1, a.input_map.len());
        assert_eq!(2, a.output_map.len());
        assert!(a.is_input_empty());
        assert!(a.is_output_empty());

        out_i32.add_value(1);
        out_f64.add_values(&vec![1.0, 2.0]);
        assert!(a.is_input_empty());
        assert!(!a.is_output_empty());
        {
            let port_dyn_ref = a.get_out_port("f64");
            assert!(!port_dyn_ref.is_empty());
        }

        a.clear_output();
        a.clear_input();
        assert!(a.is_input_empty());
        assert!(a.is_output_empty());

        in_i32.0.add_value(1);
        assert!(!a.is_input_empty());
        assert!(a.is_output_empty());
        {
            let port_dyn_ref = a.get_in_port("i32");
            assert!(!port_dyn_ref.is_empty());
        }

        a.clear_output();
        a.clear_input();
        assert!(a.is_input_empty());
        assert!(a.is_output_empty());
    }
}
