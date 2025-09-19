use std::{fs, path::Path};
use xdevs_utils::{dmt::DevsModelTree, mqtt::DmtPropagator};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // First, let's parse the DEVS Model Tree (DMT) that describe the structure of the EFP model
    let path = Path::new("xdevs_utils/examples/efp_deep.json");
    let json_str = fs::read_to_string(path).expect("Failed to read JSON file");
    let dmt = DevsModelTree::from_json(&json_str).expect("Failed to parse JSON file");

    // Next, we will create the MQTT client that propagates events through the DEVS model
    let mqtt_root = "xdevs/efp";
    let mqtt_host = "0.0.0.0";
    let mqtt_port = 1883;
    let coupled = DmtPropagator::new(mqtt_root, "efp", mqtt_host, mqtt_port, dmt);

    for handle in coupled.spawn() {
        handle.await.unwrap();
    }
}
