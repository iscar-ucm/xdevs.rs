//! MQTT sniffer that listens to all messages on a given MQTT broker and prints them to the console.
//!
//! # Usage
//!
//! ```sh
//! cargo run --bin mqtt_sniffer <client_id> <host> <port> <root_topic>
//! ```
//!
//! - `client_id`: MQTT client ID. Default is `mqtt_sniffer`.
//! - `host`: MQTT broker host. Default is `localhost`.
//! - `port`: MQTT broker port. Default is `1883`.
//! - `root_topic`: MQTT root topic to listen to. Default is `xdevs`.

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::time::Duration;

#[tokio::main]
async fn main() {
    // parse client id, jost, port and root topic from command line arguments
    let args: Vec<String> = std::env::args().collect();
    let id = args.get(1).unwrap_or(&String::from("mqtt_sniffer")).clone();
    let host = args.get(2).unwrap_or(&String::from("localhost")).clone();
    let port = args
        .get(3)
        .unwrap_or(&String::from("1883"))
        .parse()
        .unwrap();
    let root_topic = args.get(4).unwrap_or(&String::from("xdevs")).clone();

    let mut mqttoptions = MqttOptions::new(id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    tokio::task::spawn(client_thread(client, root_topic));

    loop {
        match eventloop.poll().await {
            Ok(notif) => {
                if let Event::Incoming(Packet::Publish(packet)) = notif {
                    println!(
                        "Publication at topic {}: {:?}",
                        packet.topic, packet.payload
                    );
                }
            }
            Err(e) => {
                panic!("MQTT eventloop error: {:?}", e);
            }
        }
    }
}

async fn client_thread(client: AsyncClient, root_topic: String) {
    let topic = format!("{root_topic}/#");
    println!("subscribing to MQTT topic {topic}");
    if let Err(e) = client.subscribe(topic.clone(), QoS::AtMostOnce).await {
        panic!("failed to subscribe to MQTT topic {topic}: {e:?}");
    }
}
