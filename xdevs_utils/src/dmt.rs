use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node(pub Option<String>, pub String);

impl Node {
    pub fn component(&self) -> Option<&String> {
        self.0.as_ref()
    }

    pub fn port(&self) -> &String {
        &self.1
    }
}

#[derive(Debug, Default)]
pub struct CouplingMap(HashMap<Node, HashSet<Node>>);

impl CouplingMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, from: Node, to: Node) {
        self.0.entry(from).or_default().insert(to);
    }

    pub fn get(&self, from: &Node) -> Option<&HashSet<Node>> {
        self.0.get(from)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Node, &HashSet<Node>)> {
        self.0.iter()
    }

    pub fn reverse(&self) -> CouplingMap {
        let mut map = CouplingMap::new();
        for (from, tos) in &self.0 {
            for to in tos {
                map.insert(to.clone(), from.clone());
            }
        }
        map
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CouplingType {
    EIC,
    IC,
    EOC,
    Invalid,
}

#[derive(Debug)]
pub enum CouplingError {
    NoComponents,
    InvalidPort,
    InvalidComponent,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Coupling {
    component_from: Option<String>,
    port_from: String,
    component_to: Option<String>,
    port_to: String,
}

impl Coupling {
    fn get_type(&self) -> CouplingType {
        if self.component_from.is_none() && self.component_to.is_some() {
            CouplingType::EIC
        } else if self.component_from.is_some() && self.component_to.is_some() {
            CouplingType::IC
        } else if self.component_from.is_some() && self.component_to.is_none() {
            CouplingType::EOC
        } else {
            CouplingType::Invalid
        }
    }

    fn nodes(&self) -> (Node, Node) {
        (
            Node(self.component_from.clone(), self.port_from.clone()),
            Node(self.component_to.clone(), self.port_to.clone()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct Component {
    #[serde(default)]
    input: HashSet<String>,
    #[serde(default)]
    output: HashSet<String>,
    #[serde(default)]
    components: HashMap<String, Component>,
    #[serde(default)]
    couplings: HashSet<Coupling>,
}

impl Component {
    pub fn new() -> Self {
        Self {
            input: HashSet::new(),
            output: HashSet::new(),
            components: HashMap::new(),
            couplings: HashSet::new(),
        }
    }

    pub fn add_input<S: Into<String>>(&mut self, input: S) -> bool {
        self.input.insert(input.into())
    }

    pub fn has_input(&self, input: &str) -> bool {
        self.input.contains(input)
    }

    pub fn add_output<S: Into<String>>(&mut self, output: S) -> bool {
        self.output.insert(output.into())
    }

    pub fn has_output(&self, output: &str) -> bool {
        self.output.contains(output)
    }

    pub fn add_component<S: Into<String>>(
        &mut self,
        name: S,
        component: Component,
    ) -> Option<Component> {
        self.components.insert(name.into(), component)
    }

    pub fn components(&self) -> &HashMap<String, Component> {
        &self.components
    }

    pub fn get_component(&self, name: &str) -> Option<&Component> {
        self.components.get(name)
    }

    pub fn add_coupling(&mut self, coupling: Coupling) -> Result<bool, CouplingError> {
        if !self.is_coupled() {
            return Err(CouplingError::NoComponents);
        }
        match coupling.get_type() {
            CouplingType::EIC => {
                if !self.has_input(&coupling.port_from) {
                    return Err(CouplingError::InvalidPort);
                }
                match self.get_component(coupling.component_to.as_ref().unwrap()) {
                    Some(component) => {
                        if component.has_input(&coupling.port_to) {
                            Ok(self.couplings.insert(coupling))
                        } else {
                            Err(CouplingError::InvalidPort)
                        }
                    }
                    None => Err(CouplingError::InvalidComponent),
                }
            }
            CouplingType::IC => {
                match (
                    self.get_component(coupling.component_from.as_ref().unwrap()),
                    self.get_component(coupling.component_to.as_ref().unwrap()),
                ) {
                    (Some(src), Some(dst)) => {
                        if src.has_output(&coupling.port_from) && dst.has_input(&coupling.port_to) {
                            Ok(self.couplings.insert(coupling))
                        } else {
                            Err(CouplingError::InvalidPort)
                        }
                    }
                    _ => Err(CouplingError::InvalidComponent),
                }
            }
            CouplingType::EOC => {
                match self.get_component(coupling.component_from.as_ref().unwrap()) {
                    Some(component) => {
                        if component.has_output(&coupling.port_from) {
                            Ok(self.couplings.insert(coupling))
                        } else {
                            Err(CouplingError::InvalidPort)
                        }
                    }
                    None => Err(CouplingError::InvalidComponent),
                }
            }
            CouplingType::Invalid => Err(CouplingError::InvalidComponent),
        }
    }

    pub fn get_couplings(&self) -> &HashSet<Coupling> {
        &self.couplings
    }

    pub fn coupling_map(&self) -> CouplingMap {
        let mut map = CouplingMap::new();
        for coupling in &self.couplings {
            let (from, to) = coupling.nodes();
            map.insert(from, to);
        }
        map
    }

    pub fn is_coupled(&self) -> bool {
        !self.components.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DevsModelTree {
    name: String,
    model: Component,
}

impl Deref for DevsModelTree {
    type Target = Component;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}

impl DerefMut for DevsModelTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.model
    }
}

impl DevsModelTree {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            model: Component::new(),
        }
    }
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs, path::Path};

    #[test]
    fn efp() {
        let mut p = Component::new();
        p.add_input("input_req");
        p.add_output("output_res");

        let mut ef = Component::new();
        ef.add_input("input_res");
        ef.add_output("output_req");

        let mut efp = DevsModelTree::new("efp");
        assert_eq!(efp.add_component("p", p), None);
        assert_eq!(efp.add_component("ef", ef), None);
    }

    #[test]
    fn json() {
        let path = Path::new("examples/efp_deep.json");
        let json_str = fs::read_to_string(path).expect("Failed to read JSON file");

        match DevsModelTree::from_json(&json_str) {
            Ok(devs_model_tree) => {
                println!("{:#?}", devs_model_tree);
            }
            Err(e) => {
                eprintln!("Failed to parse JSON: {:?}", e);
            }
        }
    }
}
