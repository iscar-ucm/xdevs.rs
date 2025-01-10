use xdevs::{
    gpt::Processor,
    simulation::rt::{RootCoordinator, RootCoordinatorConfig},
};
use xdevs_utils::mqtt::MqttHandler;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // First, let's create the DEVS model
    let model_name = "p";
    let proc_time = 1.;
    let obst_time = 1000.;
    let processor = Processor::new(model_name, proc_time);

    // Next, we will create the MQTT handler for the DEVS model
    let mqtt_root = "xdevs/efp";
    let mqtt_host = "0.0.0.0";
    let mqtt_port = 1883;
    let mqtt_handler = MqttHandler::new(
        format!("{mqtt_root}/components/{model_name}"),
        model_name,
        mqtt_host,
        mqtt_port,
    );

    // Now, we create the real-time simulation environment
    let time_scale = 1.;
    let max_jitter = None;
    let queue_size = 10;
    let window = None;
    let config = RootCoordinatorConfig::new(
        time_scale,
        max_jitter,
        Some(queue_size),
        Some(queue_size),
        window,
    );
    let mut simulator = RootCoordinator::new(processor, config);

    // We spawn the MQTT handler and the simulation
    let mut handles = simulator.spawn_handler(mqtt_handler);
    handles.push(tokio::task::spawn(simulator.simulate(obst_time + 10.)));

    // Finally, we wait for the simulation to finish
    for handle in handles {
        handle.await.unwrap();
    }
}
