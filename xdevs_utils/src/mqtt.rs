use rumqttc::{AsyncClient, EventLoop, Packet};
pub use rumqttc::{LastWill, MqttOptions, QoS};
use xdevs::{
    simulation::rt::{input::InputSender, output::OutputReceiver, Handler},
    Event,
};

#[derive(Clone)]
pub struct MqttHandler {
    root_topic: String,
    pub mqtt_options: MqttOptions,
    pub client_cap: usize,
    pub input_qos: QoS,
    pub output_qos: QoS,
    pub output_retain: bool,
}

impl MqttHandler {
    pub fn new<R: Into<String>, S: Into<String>, T: Into<String>>(
        root_topic: R,
        id: S,
        host: T,
        port: u16,
    ) -> Self {
        Self {
            root_topic: root_topic.into(),
            mqtt_options: MqttOptions::new(id, host, port),
            client_cap: 10,
            input_qos: QoS::AtMostOnce,
            output_qos: QoS::AtLeastOnce,
            output_retain: true,
        }
    }
}

impl Handler for MqttHandler {
    fn spawn(
        self,
        input_tx: Option<InputSender>,
        output_rx: Option<OutputReceiver>,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();
        if input_tx.is_none() && output_rx.is_none() {
            tracing::warn!("no input or output handler provided. Exiting.");
        } else {
            let root_topic = self.root_topic;
            let (client, eventloop) = AsyncClient::new(self.mqtt_options, self.client_cap);

            let client_config = ClientConfig {
                input: if input_tx.is_some() {
                    Some(self.input_qos)
                } else {
                    None
                },
                output: output_rx.map(|rx| (self.output_qos, self.output_retain, rx)),
            };
            handles.push(tokio::task::spawn(client_thread(
                client,
                root_topic.clone(),
                client_config,
            )));
            handles.push(tokio::task::spawn(eventloop_thread(eventloop, input_tx)));
        };

        handles
    }
}

struct ClientConfig {
    input: Option<QoS>,
    output: Option<(QoS, bool, OutputReceiver)>,
}

async fn client_thread(client: AsyncClient, root_topic: String, config: ClientConfig) {
    let input_topic = format!("{root_topic}/input");
    let output_topic = format!("{root_topic}/output");

    if let Some(input_qos) = config.input {
        tracing::info!("subscribing to MQTT topic {input_topic}/+");
        if let Err(e) = client
            .subscribe(format!("{input_topic}/+"), input_qos)
            .await
        {
            tracing::error!("failed to subscribe to MQTT topic {input_topic}/+: {e:?}");
            return;
        }
    }

    if let Some((output_qos, output_retain, mut output_rx)) = config.output {
        loop {
            match output_rx.recv().await {
                Ok(event) => {
                    let port = event.port();
                    let value = event.value();
                    tracing::info!("publishing value {value} to MQTT topic {output_topic}/{port}");

                    if let Err(e) = client
                        .publish(
                            format!("{output_topic}/{port}"),
                            output_qos,
                            output_retain,
                            value,
                        )
                        .await
                    {
                        tracing::warn!("failed to publish to MQTT topic: {e:?}.");
                    }
                }
                Err(e) => {
                    tracing::error!("output handler dropped: {e:?}. Disconnecting MQTT client.");
                    client.disconnect().await.ok();
                    break;
                }
            }
        }
    }
}

async fn eventloop_thread(mut eventloop: EventLoop, input_tx: Option<InputSender>) {
    loop {
        match eventloop.poll().await {
            Ok(notif) => {
                tracing::debug!("MQTT event notification: {notif:?}");
                if let Some(input_tx) = &input_tx {
                    if let rumqttc::Event::Incoming(Packet::Publish(packet)) = notif {
                        let port = packet.topic.split('/').last().unwrap().to_string();

                        let value = match String::from_utf8(packet.payload.to_vec()) {
                            Ok(string) => string,
                            Err(e) => {
                                tracing::warn!("Failed to convert payload to UTF8 String: {e}.");
                                continue;
                            }
                        };
                        match input_tx.send(Event::new(port, value)).await {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    "input handler dropped: {e:?}. Disconnecting MQTT client."
                                );
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("MQTT eventloop error: {:?}", e);
                break;
            }
        }
    }
}

#[cfg(feature = "dmt")]
pub use dmt::DmtPropagator;

#[cfg(feature = "dmt")]
mod dmt {
    use super::MqttHandler;
    use crate::dmt::{Component, DevsModelTree};
    use rumqttc::{AsyncClient, EventLoop, Packet, Publish, QoS, SubscribeFilter};
    use std::{
        collections::{HashMap, HashSet},
        ops::{Deref, DerefMut},
    };
    use tokio::sync::mpsc::{channel, Receiver, Sender};

    fn topic_map<S: Into<String>>(root: S, component: &Component) -> MqttTopicMap {
        let mut map = MqttTopicMap::new();

        let root: String = root.into();

        for (component_id, component) in component.components().iter() {
            let submap = topic_map(format!("{root}/components/{component_id}"), component);
            map.extend(submap);
        }

        for (node_from, nodes_to) in component.coupling_map().iter() {
            let port_from = node_from.port();
            let topic_from = match node_from.component() {
                Some(component) => format!("{root}/components/{component}/output/{port_from}"),
                None => format!("{root}/input/{port_from}"),
            };

            for node_to in nodes_to.iter() {
                let port_to = node_to.port();
                let topic_to = match node_to.component() {
                    Some(component) => format!("{root}/components/{component}/input/{port_to}"),
                    None => format!("{root}/output/{port_to}"),
                };

                map.insert(topic_from.clone(), topic_to);
            }
        }

        map
    }

    fn avoid_multihop(map: &mut MqttTopicMap) {
        let reverse = map.reverse();

        for (port_to, ports_from) in reverse.iter() {
            if let Some(ports_to) = map.0.remove(port_to) {
                // [ports_from] -> port_to -> [ports_to]
                // We removed port_to from map, so we will not subscribe to its topic

                for port_from in ports_from.iter() {
                    map.0.get_mut(port_from).unwrap().extend(ports_to.clone());
                }
                // We now propagate messages from [ports_from] to [ports_to] AND port_to
                // By keeping port_to in the map, we can still monitor MQTT messages on that topic
            }
        }
    }

    pub struct DmtPropagator {
        config: MqttHandler,
        group_id: Option<String>,
        model: DevsModelTree,
    }

    impl DmtPropagator {
        pub fn new<R: Into<String>, S: Into<String>, T: Into<String>>(
            root_topic: R,
            client_id: S,
            host: T,
            port: u16,
            model: DevsModelTree,
        ) -> Self {
            let config = MqttHandler::new(root_topic, client_id, host, port);
            Self {
                config,
                group_id: None,
                model,
            }
        }

        pub fn set_group_id<S: Into<String>>(&mut self, group_id: S) {
            let group_id: String = group_id.into();
            assert!(
                !group_id.contains('/'),
                "group_id cannot contain '/' character."
            );
            self.group_id = Some(group_id);
        }

        pub fn spawn(self) -> Vec<tokio::task::JoinHandle<()>> {
            let mut handles = Vec::new();
            let mut topic_map = topic_map(&self.root_topic, &self.model);
            avoid_multihop(&mut topic_map);

            let (config, _) = (self.config, self.model);

            let (sender, receiver) = channel(10);
            let (client, eventloop) = AsyncClient::new(config.mqtt_options, config.client_cap);

            handles.push(tokio::task::spawn(client_thread(
                client,
                config.input_qos,
                config.output_qos,
                config.output_retain,
                receiver,
                topic_map,
                self.group_id,
            )));
            handles.push(tokio::task::spawn(eventloop_thread(eventloop, sender)));
            handles
        }
    }

    impl Deref for DmtPropagator {
        type Target = MqttHandler;

        fn deref(&self) -> &Self::Target {
            &self.config
        }
    }

    impl DerefMut for DmtPropagator {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.config
        }
    }

    async fn client_thread(
        client: AsyncClient,
        input_qos: QoS,
        output_qos: QoS,
        output_retain: bool,
        mut input_rx: Receiver<Publish>,
        topic_map: MqttTopicMap,
        group_id: Option<String>,
    ) {
        let mut topics = topic_map
            .iter()
            .map(|(topic, _)| topic.clone())
            .collect::<Vec<_>>();
        topics.sort_unstable();
        for (n, topic) in topics.iter().enumerate() {
            let mut topic = topic.clone();
            if let Some(group_id) = &group_id {
                topic = format!("$share/{group_id}{n}/{topic}");
            }
            tracing::info!("subscribing to MQTT topic {topic}.");
            if let Err(e) = client.subscribe(topic, input_qos).await {
                tracing::error!("failed to subscribe to MQTT topic: {e:?}.");
                return;
            }
        }

        loop {
            match input_rx.recv().await {
                Some(publish) => {
                    let topic = publish.topic;
                    let payload = publish.payload;
                    tracing::info!("received value from MQTT topic {topic}.");
                    match topic_map.get(&topic) {
                        Some(topics) => {
                            for topic in topics.iter() {
                                tracing::info!("publishing value to MQTT topic {topic}");
                                if let Err(e) = client
                                    .publish(topic, output_qos, output_retain, payload.clone())
                                    .await
                                {
                                    tracing::warn!("failed to publish to MQTT topic: {e:?}.");
                                }
                            }
                        }
                        None => {
                            tracing::error!("no mapping found for MQTT topic {topic}.");
                        }
                    }
                }
                None => {
                    tracing::error!("input handler dropped. Disconnecting MQTT client.");
                    client.disconnect().await.ok();
                    break;
                }
            }
        }
    }

    async fn eventloop_thread(mut eventloop: EventLoop, sender: Sender<Publish>) {
        loop {
            match eventloop.poll().await {
                Ok(notif) => {
                    tracing::debug!("MQTT event notification: {notif:?}");
                    if let rumqttc::Event::Incoming(Packet::Publish(packet)) = notif {
                        match sender.send(packet).await {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    "output handler dropped: {e:?}. Disconnecting MQTT client."
                                );
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("MQTT eventloop error: {:?}", e);
                    break;
                }
            }
        }
    }

    struct MqttTopicMap(HashMap<String, HashSet<String>>);

    impl MqttTopicMap {
        fn new() -> Self {
            Self(HashMap::new())
        }

        fn get(&self, topic: &str) -> Option<&HashSet<String>> {
            self.0.get(topic)
        }

        fn insert(&mut self, topic: String, port: String) {
            self.0.entry(topic).or_default().insert(port);
        }

        fn extend(&mut self, other: MqttTopicMap) {
            self.0.extend(other.0);
        }

        fn iter(&self) -> impl Iterator<Item = (&String, &HashSet<String>)> {
            self.0.iter()
        }

        fn reverse(&self) -> MqttTopicMap {
            let mut map = MqttTopicMap::new();
            for (topic, ports) in self.0.iter() {
                for port in ports.iter() {
                    map.insert(port.clone(), topic.clone());
                }
            }
            map
        }
    }
}
