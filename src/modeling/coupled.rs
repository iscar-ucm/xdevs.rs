use super::port::Port;
use super::{Component, InPort, OutPort};
use crate::simulation::Simulator;
use crate::{DynRef, Shared};
use std::collections::HashMap;

#[cfg(feature = "par_xic")]
type XICLocation = (usize, usize);
#[cfg(not(feature = "par_xic"))]
type XICLocation = usize;
#[cfg(feature = "par_eoc")]
type EOCLocation = (usize, usize);
#[cfg(not(feature = "par_eoc"))]
type EOCLocation = usize;

/// Coupled DEVS model.
pub struct Coupled {
    /// Component wrapped by the coupled model.
    pub(crate) component: Component,
    /// Keys are IDs of subcomponents, and values are indices of [`Coupled::comps_vec`].
    comps_map: HashMap<String, usize>,
    /// External input couplings.
    eic_map: HashMap<String, HashMap<String, XICLocation>>,
    /// Internal couplings.
    ic_map: HashMap<String, HashMap<String, XICLocation>>,
    /// External output couplings.
    eoc_map: HashMap<String, HashMap<String, EOCLocation>>,
    /// Components of the DEVS coupled model (serialized for better performance).
    pub(crate) components: Vec<Box<dyn Simulator>>,
    /// External input and internal couplings (serialized for better performance).
    #[cfg(feature = "par_xic")]
    pub(crate) xics: Vec<(Shared<dyn Port>, Vec<Shared<dyn Port>>)>,
    #[cfg(not(feature = "par_xic"))]
    pub(crate) xics: Vec<(Shared<dyn Port>, Shared<dyn Port>)>,
    /// External output couplings (serialized for better performance).
    #[cfg(feature = "par_eoc")]
    pub(crate) eocs: Vec<(Shared<dyn Port>, Vec<Shared<dyn Port>>)>,
    #[cfg(not(feature = "par_eoc"))]
    pub(crate) eocs: Vec<(Shared<dyn Port>, Shared<dyn Port>)>,
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
            components: Vec::new(),
            xics: Vec::new(),
            eocs: Vec::new(),
        }
    }

    /// Returns the number of components in the coupled model.
    pub fn n_components(&self) -> usize {
        self.components.len()
    }

    /// Returns the number of external input couplings in the coupled model.
    pub fn n_eics(&self) -> usize {
        self.eic_map.values().map(|eics| eics.len()).sum()
    }

    /// Returns the number of internal couplings in the coupled model.
    pub fn n_ics(&self) -> usize {
        self.ic_map.values().map(|ics| ics.len()).sum()
    }

    /// Returns the number of external output couplings in the coupled model.
    pub fn n_eocs(&self) -> usize {
        self.eocs.len()
    }

    /// Adds a new input port of type [`Port<Input, T>`] and returns a reference to it.
    /// It panics if there is already an input port with the same name.
    #[inline]
    pub fn add_in_port<T: DynRef + Clone>(&mut self, name: &str) -> InPort<T> {
        self.component.add_in_port::<T>(name)
    }

    /// Adds a new output port of type [`Port<Output, T>`] and returns a reference to it.
    /// It panics if there is already an output port with the same name.
    #[inline]
    pub fn add_out_port<T: DynRef + Clone>(&mut self, name: &str) -> OutPort<T> {
        self.component.add_out_port::<T>(name)
    }

    /// Adds a new component to the coupled model.
    /// If there is already a component with the same name as the new component, it panics.
    pub fn add_component<T: Simulator>(&mut self, component: Box<T>) {
        let component_name = component.get_name();
        if self.comps_map.contains_key(component_name) {
            panic!("coupled model already contains component with the name provided")
        }
        self.comps_map
            .insert(component_name.to_string(), self.components.len());
        self.components.push(component);
    }

    /// Returns a reference to a component with the provided name.
    /// If the coupled model does not contain any model with that name, it return [`None`].
    fn get_component(&self, name: &str) -> Option<&Component> {
        let index = *self.comps_map.get(name)?;
        Some(self.components.get(index)?.get_component())
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
        let p_from = self
            .component
            .get_in_port(port_from)
            .expect("port_from does not exist");
        let comp_to = self
            .get_component(component_to)
            .expect("component_to does not exist");
        let p_to = comp_to
            .get_in_port(port_to)
            .expect("port_to does not exist");
        if !p_from.is_compatible(&*p_to) {
            panic!("ports are not compatible")
        }
        let source_key = port_from.to_string();
        let destination_key = component_to.to_string() + "-" + port_to;
        let coups = self.eic_map.entry(destination_key).or_default();
        if coups.contains_key(&source_key) {
            panic!("coupling already exists");
        }

        #[cfg(feature = "par_xic")]
        {
            let i = match coups.values().next() {
                Some((i, _)) => *i,
                None => {
                    self.xics.push((p_to, Vec::new()));
                    self.xics.len() - 1
                }
            };
            let eics = &mut self.xics[i].1;
            coups.insert(source_key, (i, eics.len()));
            eics.push(p_from);
        }
        #[cfg(not(feature = "par_xic"))]
        {
            coups.insert(source_key, self.xics.len());
            self.xics.push((p_to, p_from));
        }
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
        let comp_from = self
            .get_component(component_from)
            .expect("component_from does not exist");
        let p_from = comp_from
            .get_out_port(port_from)
            .expect("port_from does not exist");
        let comp_to = self
            .get_component(component_to)
            .expect("component_to does not exist");
        let p_to = comp_to
            .get_in_port(port_to)
            .expect("port_to does not exist");
        if !p_from.is_compatible(&*p_to) {
            panic!("ports are not compatible")
        }
        let source_key = component_from.to_string() + "-" + port_from;
        let destination_key = component_to.to_string() + "-" + port_to;
        let coups = self.ic_map.entry(destination_key).or_default();
        if coups.contains_key(&source_key) {
            panic!("coupling already exists");
        }

        #[cfg(feature = "par_xic")]
        {
            let i = match coups.values().next() {
                Some((i, _)) => *i,
                None => {
                    self.xics.push((p_to, Vec::new()));
                    self.xics.len() - 1
                }
            };
            let ics = &mut self.xics[i].1;
            coups.insert(source_key, (i, ics.len()));
            ics.push(p_from);
        }
        #[cfg(not(feature = "par_xic"))]
        {
            coups.insert(source_key, self.xics.len());
            self.xics.push((p_to, p_from));
        }
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
        let comp_from = self
            .get_component(component_from)
            .expect("component_from does not exist");
        let p_from = comp_from
            .get_out_port(port_from)
            .expect("port_from does not exist");
        let p_to = self
            .component
            .get_out_port(port_to)
            .expect("port_to does not exist");
        if !p_from.is_compatible(&*p_to) {
            panic!("ports are not compatible")
        }
        let source_key = component_from.to_string() + "-" + port_from;
        let destination_key = port_to.to_string();
        let coups = self.eoc_map.entry(destination_key).or_default();
        if coups.contains_key(&source_key) {
            panic!("coupling already exists");
        }

        #[cfg(feature = "par_eoc")]
        {
            let i = match coups.values().next() {
                Some((i, _)) => *i,
                None => {
                    self.eocs.push((p_to, Vec::new()));
                    self.eocs.len() - 1
                }
            };
            let eocs = &mut self.eocs[i].1;
            coups.insert(source_key, (i, eocs.len()));
            eocs.push(p_from);
        }
        #[cfg(not(feature = "par_eoc"))]
        {
            coups.insert(source_key, self.eocs.len());
            self.eocs.push((p_to, p_from));
        }
    }
}
