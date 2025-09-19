use std::env;
use xdevs::{
    gpt::{Efp, Gpt},
    modeling::Coupled,
    simulation::rt::{RootCoordinator, RootCoordinatorConfig},
};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).unwrap();

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

    let time_scale = 1.;
    let max_jitter = None;
    let output_capacity = Some(10);
    let input_buffer = Some(10);
    let input_window = None;

    let config = RootCoordinatorConfig::new(
        time_scale,
        max_jitter,
        output_capacity,
        input_buffer,
        input_window,
    );

    let simulator = RootCoordinator::new(coupled, config);

    simulator.simulate(observation + 10.).await;
}
