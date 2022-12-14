use crate::modeling::port::AbstractPort;
use crate::modeling::{Component, Input, Output, Port};
use crate::simulation::Simulator;
use crate::RcHash;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter, Result};
use std::rc::Rc;

/// Helper type for keeping track of couplings and avoiding duplicates.
type CouplingsMap = HashMap<RcHash<dyn AbstractPort>, HashSet<RcHash<dyn AbstractPort>>>;

/// Helper type for storing couplings in a vector as tuples (port from, port to).
type CouplingsVec = Vec<(Rc<dyn AbstractPort>, Rc<dyn AbstractPort>)>;

/// Coupled DEVS model.
#[derive(Debug)]
pub struct Coupled {
    /// Component wrapped by the coupled model.
    pub(crate) component: Component,
    /// Keys are IDs of subcomponents, and values are indices of [`Coupled::comps_vec`].
    comps_map: HashMap<String, usize>,
    /// External input couplings.
    eic_map: CouplingsMap,
    /// Internal couplings.
    ic_map: CouplingsMap,
    /// External output couplings.
    eoc_map: CouplingsMap,
    /// Components of the DEVS coupled model (serialized for better performance).
    pub(crate) comps_vec: Vec<Box<dyn Simulator>>,
    /// External input couplings (serialized for better performance).
    pub(crate) eic_vec: CouplingsVec,
    /// Internal couplings (serialized for better performance).
    pub(crate) ic_vec: CouplingsVec,
    /// External output couplings (serialized for better performance).
    pub(crate) eoc_vec: CouplingsVec,
}

impl Coupled {
    /// Creates a new coupled DEVS model.
    pub fn new(name: &str) -> Self {
        Self {
            component: Component::new(name),
            comps_map: HashMap::new(),
            eic_map: HashMap::new(),
            ic_map: HashMap::new(),
            eoc_map: HashMap::new(),
            comps_vec: Vec::new(),
            eic_vec: Vec::new(),
            ic_vec: Vec::new(),
            eoc_vec: Vec::new(),
        }
    }

    /// Helper function to add a new coupling to a coupled model.
    fn add_coupling(
        coup_map: &mut CouplingsMap,
        coup_vec: &mut CouplingsVec,
        port_from: Rc<dyn AbstractPort>,
        port_to: Rc<dyn AbstractPort>,
    ) {
        if !port_from.is_compatible(&*port_to) {
            panic!("ports are incompatible");
        }
        let ports_from = coup_map
            .entry(RcHash(port_to.clone()))
            .or_insert_with(HashSet::new);
        if !ports_from.insert(RcHash(port_from.clone())) {
            panic!("duplicate coupling");
        }
        coup_vec.push((port_from.clone(), port_to.clone()));
    }

    /// Adds a new input port of type [`Port<Input, T>`] and returns a reference to it.
    /// It panics if there is already an input port with the same name.
    pub fn add_in_port<T: 'static + Clone + Debug>(&mut self, name: &str) -> Port<Input, T> {
        self.component.add_in_port::<T>(name)
    }

    /// Adds a new output port of type [`Port<Output, T>`] and returns a reference to it.
    /// It panics if there is already an output port with the same name.
    pub fn add_out_port<T: 'static + Clone + Debug>(&mut self, name: &str) -> Port<Output, T> {
        self.component.add_out_port::<T>(name)
    }

    /// Adds a new component to the coupled model.
    /// If there is already a component with the same name as the new component, it panics.
    pub fn add_component<T: 'static + Simulator>(&mut self, component: Box<T>) {
        let component_name = component.get_name();
        if self.comps_map.contains_key(component_name) {
            panic!("coupled model already contains component with the name provided")
        }
        self.comps_map
            .insert(component_name.to_string(), self.comps_vec.len());
        self.comps_vec.push(component);
    }

    /// Returns a reference to a component with the provided name.
    /// If the coupled model does not contain any model with that name, it panics.
    fn get_component(&self, name: &str) -> &dyn Simulator {
        self.comps_vec
            .get(
                *self
                    .comps_map
                    .get(name)
                    .expect("coupled model does not contain component with the name provided"),
            )
            .unwrap()
            .as_ref()
    }

    /// Adds a new EIC to the model.
    /// You must provide the input port name of the coupled model,
    /// the receiving component name, and its input port name.
    /// This method panics if:
    /// - the origin port does not exist.
    /// - the destination component does not exist.
    /// - the destination port does not exist.
    /// - ports are not compatible.
    /// - coupling already exists.
    pub fn add_eic(&mut self, port_from: &str, component_to: &str, port_to: &str) {
        let from = self.component.get_in_port(port_from);
        let component = self.get_component(component_to).get_component();
        let to = component.get_in_port(port_to);
        Self::add_coupling(&mut self.eic_map, &mut self.eic_vec, from, to);
    }

    /// Adds a new IC to the model.
    /// You must provide the sending component name, its output port name,
    /// the receiving component name, and its input port name.
    /// This method panics if:
    /// - the origin component does not exist.
    /// - the origin port does not exist.
    /// - the destination component does not exist.
    /// - the destination port does not exist.
    /// - ports are not compatible.
    /// - coupling already exists.
    pub fn add_ic(
        &mut self,
        component_from: &str,
        port_from: &str,
        component_to: &str,
        port_to: &str,
    ) {
        let from = self
            .get_component(component_from)
            .get_component()
            .get_out_port(port_from);
        let to = self
            .get_component(component_to)
            .get_component()
            .get_in_port(port_to);
        Self::add_coupling(&mut self.ic_map, &mut self.ic_vec, from, to);
    }

    /// Adds a new EOC to the model.
    /// You must provide the sending component name, its output port name,
    /// and the output port name of the coupled model.
    /// This method panics if:
    /// - the origin component does not exist.
    /// - the origin port does not exist.
    /// - the destination port does not exist.
    /// - ports are not compatible.
    /// - coupling already exists.
    pub fn add_eoc(&mut self, component_from: &str, port_from: &str, port_to: &str) {
        let from = self
            .get_component(component_from)
            .get_component()
            .get_out_port(port_from);
        let to = self.component.get_out_port(port_to);
        Self::add_coupling(&mut self.eoc_map, &mut self.eoc_vec, from, to);
    }
}

impl Display for Coupled {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.get_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "coupled model already contains component with the name provided")]
    fn test_duplicate_component() {
        let mut top_coupled = Coupled::new("top_coupled");
        top_coupled.add_component(Box::new(Coupled::new("component")));
        top_coupled.add_component(Box::new(Coupled::new("component")));
    }

    #[test]
    #[should_panic(expected = "coupled model does not contain component with the name provided")]
    fn test_get_component() {
        let mut top_coupled = Coupled::new("top_coupled");
        top_coupled.add_component(Box::new(Coupled::new("component_1")));
        assert_eq!(
            "component_1",
            top_coupled.get_component("component_1").get_name()
        );
        assert_eq!(
            top_coupled.get_component("component_1").get_name(),
            top_coupled.get_component("component_1").get_name()
        );
        top_coupled.get_component("component_2");
    }

    #[test]
    #[should_panic(expected = "component does not contain input port with the name provided")]
    fn test_eic_bad_port_from() {
        let mut top_coupled = Coupled::new("top_coupled");
        top_coupled.add_eic("bad_input", "bad_component", "bad_output");
    }

    #[test]
    #[should_panic(expected = "coupled model does not contain component with the name provided")]
    fn test_eic_bad_component_to() {
        let mut top_coupled = Coupled::new("top_coupled");
        top_coupled.add_in_port::<i32>("input");
        top_coupled.add_eic("input", "bad_component", "bad_output");
    }

    #[test]
    #[should_panic(expected = "component does not contain input port with the name provided")]
    fn test_eic_bad_port_to() {
        let mut top_coupled = Coupled::new("top_coupled");
        top_coupled.add_in_port::<i32>("input");
        top_coupled.add_component(Box::new(Coupled::new("component")));
        top_coupled.add_eic("input", "component", "bad_output");
    }

    #[test]
    #[should_panic(expected = "ports are incompatible")]
    fn test_eic_bad_types() {
        let mut top_coupled = Coupled::new("top_coupled");
        top_coupled.add_in_port::<i32>("input");
        let mut component = Coupled::new("component");
        component.add_in_port::<i64>("input");
        top_coupled.add_component(Box::new(component));
        top_coupled.add_eic("input", "component", "input");
    }

    #[test]
    #[should_panic(expected = "duplicate coupling")]
    fn test_eic() {
        let mut top_coupled = Coupled::new("top_coupled");
        top_coupled.add_in_port::<i32>("input");
        let mut component = Coupled::new("component");
        component.add_in_port::<i32>("input");
        top_coupled.add_component(Box::new(component));
        top_coupled.add_eic("input", "component", "input");
        top_coupled.add_eic("input", "component", "input");
    }
}
