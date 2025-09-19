use crate::gpt::Job;
use crate::modeling::{Atomic, Component, InPort, OutPort};

pub struct Processor {
    component: Component,
    sigma: f64,
    time: f64,
    job: Option<usize>,
    input_req: InPort<usize>,
    output_res: OutPort<Job>,
}

impl Processor {
    pub fn new(name: &str, time: f64) -> Self {
        let mut component = Component::new(name);
        let input_req = component.add_in_port::<usize>("input_req");
        let output_res = component.add_out_port::<Job>("output_res");
        Self {
            sigma: f64::INFINITY,
            time,
            job: None,
            input_req,
            output_res,
            component,
        }
    }
}

impl Atomic for Processor {
    fn get_component(&self) -> &Component {
        &self.component
    }

    fn get_component_mut(&mut self) -> &mut Component {
        &mut self.component
    }

    fn lambda(&self) {
        if let Some(job) = self.job {
            // Safety: adding message on atomic model's output port at lambda
            unsafe { self.output_res.add_value(Job(job, self.time)) };
        }
    }

    fn delta_int(&mut self) {
        self.sigma = f64::INFINITY;
        self.job = None;
    }

    fn delta_ext(&mut self, e: f64) {
        self.sigma -= e;
        if self.job.is_none() {
            // Safety: reading messages on atomic model's input port at delta_ext
            self.job = Some(*unsafe { self.input_req.get_values() }.first().unwrap());
            self.sigma = self.time;
        }
    }

    fn ta(&self) -> f64 {
        self.sigma
    }
}
