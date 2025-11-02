use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::anyhow;
use crossbeam_channel::RecvTimeoutError;
use log::{LevelFilter, debug, info};
use paho_mqtt::{Client, ConnectOptionsBuilder, Message, QoS, Receiver};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

use crate::{config::Config, ruuvi::RuuviMessage};

mod config;
mod ruuvi;

fn init_signals() -> anyhow::Result<Arc<AtomicBool>> {
    let stop_requested = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop_requested))
        .map_err(|error| anyhow!("Unable to register custom handler for SIGTERM: {error}"))?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&stop_requested))
        .map_err(|error| anyhow!("Unable to register custom handler for SIGINT: {error}"))?;
    Ok(stop_requested)
}

fn init_logging(config: &Config) -> anyhow::Result<()> {
    TermLogger::init(
        LevelFilter::from_str(config.log_level())
            .map_err(|error| anyhow!("Cannot parse log level {:?}: {error}", config.log_level()))?,
        ConfigBuilder::new()
            .add_filter_ignore_str("paho_mqtt_c")
            .add_filter_ignore_str("paho_mqtt")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .map_err(|error| anyhow!("Error initialising logger: {error}"))
}

/*
Example RuuviTag birth message

Topic: homeassistant/sensor/E08BAE6FD896-mov/config
QoS: 0

{"stat_t": "+/+/BTtoMQTT/E08BAE6FD896", "name": "RuuviTag_RAWv2-mov", "uniq_id": "E08BAE6FD896-mov", "val_tpl": "{{ value_json.mov | is_defined }}", "device": {"ids": ["E08BAE6FD896"], "cns": [["mac", "E08BAE6FD896"]], "mf": "Ruuvi", "mdl": "RuuviTag_RAWv2", "name": "RuuviTag-6FD896", "via_device": "TheengsGateway"}}
 */

/// Connect to the MQTT broker specified by the environment variables.
fn connect(config: &Config, stop_requested: Arc<AtomicBool>) -> anyhow::Result<Client> {
    if stop_requested.load(Ordering::Relaxed) {
        return Err(anyhow!("Received termination signal"));
    }

    let client = Client::new(config.broker_url())
        .map_err(|error| anyhow!("Error creating MQTT client: {error}"))?;
    let connection_options = ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .automatic_reconnect(Duration::from_secs(5), Duration::from_hours(1))
        .clean_session(true)
        .user_name(config.username())
        .password(config.password())
        .finalize();
    client
        .connect(connection_options)
        .map_err(|error| anyhow!("Unable to connect to broker: {error}"))?;
    Ok(client)
}

/// Subscribe to the topics containing RuuviTag data.
fn subscribe(
    client: &Client,
    config: &Config,
    stop_requested: Arc<AtomicBool>,
) -> anyhow::Result<Receiver<Option<Message>>> {
    if stop_requested.load(Ordering::Relaxed) {
        return Err(anyhow!("Received termination signal"));
    }

    // According to paho-mqtt docs, we first need to set up receiving and only afterwards subscribe to topics.
    let message_receiver = client.start_consuming();

    if stop_requested.load(Ordering::Relaxed) {
        return Err(anyhow!("Received termination signal"));
    }

    client
        .subscribe(config.listen_topic(), QoS::AtMostOnce)
        .map_err(|error| {
            anyhow!(
                "Unable to subscribe to topic {}: {error}",
                config.listen_topic()
            )
        })?;
    Ok(message_receiver)
}

fn consume_messages(
    client: &Client,
    message_receiver: &Receiver<Option<Message>>,
    config: &Config,
    stop_requested: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut known_devices = HashSet::new();

    while !stop_requested.load(Ordering::Relaxed) {
        let message = match message_receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(Some(message)) => message,
            Ok(None) | Err(RecvTimeoutError::Disconnected) => {
                return Err(anyhow!("MQTT client unexpectedly disconnected"));
            }
            Err(RecvTimeoutError::Timeout) => continue,
        };

        let ruuvi_message = match RuuviMessage::try_from(message) {
            Ok(message) => message,
            Err(error) => {
                debug!("{error}"); // Error is a decorated message already.
                continue;
            }
        };

        debug!("Received message: {ruuvi_message}");

        if !known_devices.contains(&ruuvi_message.id) {
            info!("Discovered new RuuviTag {ruuvi_message}");
            known_devices.insert(ruuvi_message.id.clone());
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let stop_requested = init_signals()?;
    let config = Config::parse_env()?;
    init_logging(&config)?;

    let client = connect(&config, Arc::clone(&stop_requested))?;
    let message_receiver = subscribe(&client, &config, Arc::clone(&stop_requested))?;
    consume_messages(
        &client,
        &message_receiver,
        &config,
        Arc::clone(&stop_requested),
    )?;

    Ok(())
}
