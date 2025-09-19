use crate::gpt::Job;
use crate::modeling::{Atomic, Component, InPort, OutPort};

pub struct Transducer {
    component: Component,
    sigma: f64,
    input_req: InPort<usize>,
    input_res: InPort<Job>,
    output_stop: OutPort<bool>,
}

impl Transducer {
    pub fn new(name: &str, time: f64) -> Self {
        let mut component = Component::new(name);
        let input_req = component.add_in_port::<usize>("input_req");
        let input_res = component.add_in_port::<Job>("input_res");
        let output_stop = component.add_out_port::<bool>("output_stop");
        Self {
            sigma: time,
            input_req,
            input_res,
            output_stop,
            component,
        }
    }
}

impl Atomic for Transducer {
    fn get_component(&self) -> &Component {
        &self.component
    }

    fn get_component_mut(&mut self) -> &mut Component {
        &mut self.component
    }

    fn lambda(&self) {
        // Safety: adding message on atomic model's output port at lambda
        unsafe { self.output_stop.add_value(true) };
    }

    fn delta_int(&mut self) {
        self.sigma = f64::INFINITY;
        println!("TRANSDUCER FINISHED");
    }

    fn delta_ext(&mut self, e: f64) {
        self.sigma -= e;
        let t = self.component.get_t_last() + e;
        // Safety: reading messages on atomic model's input port at delta_ext
        for job in unsafe { self.input_req.get_values() }.iter() {
            println!("generator sent job {job} at time {t}");
        }
        // Safety: reading messages on atomic model's input port at delta_ext
        for job in unsafe { self.input_res.get_values() }.iter() {
            println!(
                "processor processed job {} after {} seconds at time {t}",
                job.0, job.1
            );
        }
    }

    fn ta(&self) -> f64 {
        self.sigma
    }
}
