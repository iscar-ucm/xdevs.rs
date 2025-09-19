use crate::modeling::{Atomic, Component, InPort, OutPort};

pub struct Generator {
    component: Component,
    sigma: f64,
    period: f64,
    count: usize,
    input_stop: InPort<bool>,
    output_req: OutPort<usize>,
}

impl Generator {
    pub fn new(name: &str, period: f64) -> Self {
        let mut component = Component::new(name);
        let input_stop = component.add_in_port::<bool>("input_stop");
        let output_req = component.add_out_port::<usize>("output_req");
        Self {
            sigma: 0.,
            period,
            count: 0,
            input_stop,
            output_req,
            component,
        }
    }
}

impl Atomic for Generator {
    fn get_component(&self) -> &Component {
        &self.component
    }

    fn get_component_mut(&mut self) -> &mut Component {
        &mut self.component
    }

    fn lambda(&self) {
        // Safety: adding message on atomic model's output port at lambda
        unsafe { self.output_req.add_value(self.count) };
    }

    fn delta_int(&mut self) {
        self.count += 1;
        self.sigma = self.period;
    }

    fn delta_ext(&mut self, e: f64) {
        self.sigma -= e;
        // Safety: reading messages on atomic model's input port at delta_ext
        if !unsafe { self.input_stop.is_empty() } {
            self.sigma = f64::INFINITY;
        }
    }

    fn ta(&self) -> f64 {
        self.sigma
    }
}
