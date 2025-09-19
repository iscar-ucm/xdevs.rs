use crate::modeling::*;

pub(super) struct DEVStoneSeeder {
    component: Component,
    sigma: f64,
    output: OutPort<usize>,
}

impl DEVStoneSeeder {
    pub(super) fn new(name: &str) -> Self {
        let mut component = Component::new(name);
        let output = component.add_out_port("output");
        Self {
            sigma: 0.,
            component,
            output,
        }
    }
}

impl Atomic for DEVStoneSeeder {
    #[inline]
    fn get_component(&self) -> &Component {
        &self.component
    }

    #[inline]
    fn get_component_mut(&mut self) -> &mut Component {
        &mut self.component
    }

    #[inline]
    fn lambda(&self) {
        // Safety: adding message on atomic model's output port at lambda
        unsafe { self.output.add_value(0) };
    }

    #[inline]
    fn delta_int(&mut self) {
        self.sigma = f64::INFINITY;
    }

    #[inline]
    fn delta_ext(&mut self, _e: f64) {
        self.sigma = f64::INFINITY;
    }

    #[inline]
    fn ta(&self) -> f64 {
        self.sigma
    }
}
