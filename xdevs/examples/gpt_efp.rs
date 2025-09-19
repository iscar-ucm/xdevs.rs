use std::env;
use xdevs::{
    gpt::{Efp, Gpt},
    modeling::Coupled,
    simulation::RootCoordinator,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let model_type = match args.get(1) {
        Some(model) => model.clone(),
        None => String::from("gpt"),
    }
    .to_lowercase();
    let period = 3.;
    let time = 1.;
    let observation = 50.;

    let coupled: Coupled = match model_type.as_str() {
        "gpt" => Gpt::new("gpt", period, time, observation).into(),
        "efp" => Efp::new("efp", period, time, observation).into(),
        _ => panic!("unknown model type. It must be either \"gpt\" or \"efp\""),
    };
    let mut simulator = RootCoordinator::new(coupled);
    simulator.simulate(f64::INFINITY)
}
