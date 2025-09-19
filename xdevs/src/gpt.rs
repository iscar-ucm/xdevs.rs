use crate::{impl_coupled, modeling::Coupled};
use std::str::FromStr;

pub mod generator;
pub mod processor;
pub mod transducer;

pub use generator::Generator;
pub use processor::Processor;
pub use transducer::Transducer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Job(pub usize, pub f64);

impl std::fmt::Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

pub struct ParseError();

impl FromStr for Job {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // parse something like "(id, time)"
        let s = s.trim();
        if s.starts_with('(') && s.ends_with(')') {
            let s = &s[1..s.len() - 1];
            let mut parts = s.split(',');
            let id = parts
                .next()
                .ok_or(ParseError())?
                .trim()
                .parse()
                .map_err(|_| ParseError())?;
            let time = parts
                .next()
                .ok_or(ParseError())?
                .trim()
                .parse()
                .map_err(|_| ParseError())?;
            if parts.next().is_none() {
                Ok(Self(id, time))
            } else {
                Err(ParseError())
            }
        } else {
            Err(ParseError())
        }
    }
}

pub struct Gpt {
    coupled: Coupled,
}

impl Gpt {
    pub fn new(name: &str, req_period: f64, proc_time: f64, obs_time: f64) -> Self {
        let mut coupled = Coupled::new(name);

        let generator = Generator::new("generator", req_period);
        let processor = Processor::new("processor", proc_time);
        let transducer = Transducer::new("transducer", obs_time);

        coupled.add_component(Box::new(generator));
        coupled.add_component(Box::new(processor));
        coupled.add_component(Box::new(transducer));

        coupled.add_ic("generator", "output_req", "processor", "input_req");
        coupled.add_ic("generator", "output_req", "transducer", "input_req");
        coupled.add_ic("processor", "output_res", "transducer", "input_res");
        coupled.add_ic("transducer", "output_stop", "generator", "input_stop");

        Self { coupled }
    }
}

impl_coupled!(Gpt);

pub struct ExperimentalFrame {
    coupled: Coupled,
}

impl ExperimentalFrame {
    pub fn new(name: &str, req_period: f64, obs_time: f64) -> Self {
        let mut coupled = Coupled::new(name);

        coupled.add_in_port::<Job>("input_res");
        coupled.add_out_port::<usize>("output_req");

        let generator = Generator::new("generator", req_period);
        let transducer = Transducer::new("transducer", obs_time);

        coupled.add_component(Box::new(generator));
        coupled.add_component(Box::new(transducer));

        coupled.add_eic("input_res", "transducer", "input_res");
        coupled.add_ic("generator", "output_req", "transducer", "input_req");
        coupled.add_ic("transducer", "output_stop", "generator", "input_stop");
        coupled.add_eoc("generator", "output_req", "output_req");

        Self { coupled }
    }
}

impl_coupled!(ExperimentalFrame);

pub struct Efp {
    coupled: Coupled,
}

impl Efp {
    pub fn new(name: &str, req_period: f64, proc_time: f64, obs_time: f64) -> Self {
        let mut coupled = Coupled::new(name);

        let ef = ExperimentalFrame::new("ef", req_period, obs_time);
        let processor = Processor::new("processor", proc_time);

        coupled.add_component(Box::new(ef));
        coupled.add_component(Box::new(processor));

        coupled.add_ic("ef", "output_req", "processor", "input_req");
        coupled.add_ic("processor", "output_res", "ef", "input_res");

        Self { coupled }
    }
}

impl_coupled!(Efp);
