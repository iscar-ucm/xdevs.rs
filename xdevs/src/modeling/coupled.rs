use crate::{
    modeling::{
        port::{Port, PortVal},
        Component, InPort, OutPort,
    },
    simulation::Simulator,
};
use std::{collections::HashMap, sync::Arc};

pub(crate) type Coupling = (Arc<dyn Port>, Arc<dyn Port>);

/// Coupled DEVS model.
pub struct Coupled {
    /// Component wrapped by the coupled model.
    pub(crate) component: Component,
    /// Components map. Keys are components' IDs.
    comps_map: HashMap<String, usize>,
    /// External input couplings map.
    eic_map: HashMap<String, HashMap<String, usize>>,
    /// Internal couplings map.
    ic_map: HashMap<String, HashMap<String, usize>>,
    /// External output couplings map.
    eoc_map: HashMap<String, HashMap<String, usize>>,
    /// Components of the DEVS coupled model (serialized for better performance).
    pub(crate) components: Vec<Box<dyn Simulator>>,
    /// External input couplings (serialized for better performance).
    pub(crate) eics: Vec<Coupling>,
    /// Internal couplings (serialized for better performance).
    pub(crate) ics: Vec<Coupling>,
    /// External output couplings (serialized for better performance).
    pub(crate) eocs: Vec<Coupling>,
    #[cfg(feature = "par_couplings")]
    pub(crate) par_eics: Vec<Vec<Coupling>>,
    #[cfg(feature = "par_couplings")]
    pub(crate) par_xxcs: Vec<Vec<Coupling>>,
}

impl Coupled {
    /// Creates a new coupled DEVS model with the provided name.
    pub fn new(name: &str) -> Self {
        Self {
            component: Component::new(name),
            comps_map: HashMap::new(),
            eic_map: HashMap::new(),
            ic_map: HashMap::new(),
            eoc_map: HashMap::new(),
            components: Vec::new(),
            eics: Vec::new(),
            ics: Vec::new(),
            eocs: Vec::new(),
            #[cfg(feature = "par_couplings")]
            par_eics: Vec::new(),
            #[cfg(feature = "par_couplings")]
            par_xxcs: Vec::new(),
        }
    }

    /// Returns the number of components in the coupled model.
    #[inline]
    pub fn n_components(&self) -> usize {
        self.components.len()
    }

    /// Returns the number of external input couplings in the coupled model.
    #[inline]
    pub fn n_eics(&self) -> usize {
        self.eic_map.values().map(|eics| eics.len()).sum()
    }

    /// Returns the number of internal couplings in the coupled model.
    #[inline]
    pub fn n_ics(&self) -> usize {
        self.ic_map.values().map(|ics| ics.len()).sum()
    }

    /// Returns the number of external output couplings in the coupled model.
    #[inline]
    pub fn n_eocs(&self) -> usize {
        self.eoc_map.values().map(|eocs| eocs.len()).sum()
    }

    /// Adds a new input port of type `T` and returns a reference to it.
    ///
    /// It panics if there is already an input port with the same name.
    #[inline]
    pub fn add_in_port<T: PortVal>(&mut self, name: &str) -> InPort<T> {
        self.component.add_in_port::<T>(name)
    }

    /// Adds a new output port of type `T` and returns a reference to it.
    ///
    /// It panics if there is already an output port with the same name.
    #[inline]
    pub fn add_out_port<T: PortVal>(&mut self, name: &str) -> OutPort<T> {
        self.component.add_out_port::<T>(name)
    }

    /// Adds a new component to the coupled model.
    ///
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
    ///
    /// If the coupled model does not contain any model with that name, it returns [`None`].
    #[inline]
    fn get_component(&self, name: &str) -> Option<&Component> {
        let index = *self.comps_map.get(name)?;
        Some(self.components.get(index)?.get_component())
    }

    /// Adds a new EIC to the model.
    ///
    /// You must provide the input port name of the coupled model,
    /// the receiving component name, and its input port name.
    ///
    /// # Panics
    ///
    /// This method panics if:
    ///
    /// - The origin port does not exist.
    /// - The destination component does not exist.
    /// - The destination port does not exist.
    /// - Ports are not compatible.
    /// - Coupling already exists.
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
        coups.insert(source_key, self.eics.len());
        self.eics.push((p_to, p_from));
    }

    /// Adds a new IC to the model.
    ///
    /// You must provide the sending component name, its output port name,
    /// the receiving component name, and its input port name.
    ///
    /// # Panics
    ///
    /// This method panics if:
    ///
    /// - The origin component does not exist.
    /// - The origin port does not exist.
    /// - The destination component does not exist.
    /// - The destination port does not exist.
    /// - Ports are not compatible.
    /// - Coupling already exists.
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
        coups.insert(source_key, self.ics.len());
        self.ics.push((p_to, p_from));
    }

    /// Adds a new EOC to the model.
    ///
    /// You must provide the sending component name, its output port name,
    /// and the output port name of the coupled model.
    ///
    /// # Panics
    ///
    /// This method panics if:
    ///
    /// - The origin component does not exist.
    /// - The origin port does not exist.
    /// - The destination port does not exist.
    /// - Ports are not compatible.
    /// - Coupling already exists.
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
        coups.insert(source_key, self.eocs.len());
        self.eocs.push((p_to, p_from));
    }

    #[cfg(feature = "par_couplings")]
    #[inline]
    pub(crate) fn build_par_eics(&mut self) {
        for coups in self.eic_map.values() {
            let mut x = Vec::new();
            for &source in coups.values() {
                x.push(self.eics[source].clone());
            }
            self.par_eics.push(x);
        }
    }

    #[cfg(feature = "par_couplings")]
    #[inline]
    pub(crate) fn build_par_ics(&mut self) {
        for coups in self.ic_map.values() {
            let mut x = Vec::new();
            for &source in coups.values() {
                x.push(self.ics[source].clone());
            }
            self.par_xxcs.push(x);
        }
    }

    #[cfg(feature = "par_couplings")]
    #[inline]
    pub(crate) fn build_par_eocs(&mut self) {
        for coups in self.eoc_map.values() {
            let mut x = Vec::new();
            for &source in coups.values() {
                x.push(self.eocs[source].clone());
            }
            self.par_xxcs.push(x);
        }
    }
}
