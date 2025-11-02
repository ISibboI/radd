use std::time::Duration;

use log::{LevelFilter, info};
use paho_mqtt::{Client, ConnectOptionsBuilder, QoS};
use simplelog::{ColorChoice, TermLogger, TerminalMode};

fn init_logging() {
    TermLogger::init(
        LevelFilter::Debug,
        Default::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();
}

fn main() -> anyhow::Result<()> {
    init_logging();

    let broker_url = std::env::var("MQTT_BROKER_URL").map_err(|error| match error {
        std::env::VarError::NotPresent => {
            anyhow::format_err!("Environment variable MQTT_BROKER_URL not set")
        }
        std::env::VarError::NotUnicode(connection_url) => anyhow::format_err!(
            "Environment variable MQTT_BROKER_URL contains non-unicode characters: {connection_url:?}"
        ),
    })?;

    let username = std::env::var("MQTT_USERNAME").map_err(|error| match error {
        std::env::VarError::NotPresent => {
            anyhow::format_err!("Environment variable MQTT_USERNAME not set")
        }
        std::env::VarError::NotUnicode(username) => anyhow::format_err!(
            "Environment variable MQTT_USERNAME contains non-unicode characters: {username:?}"
        ),
    })?;

    let password = std::env::var("MQTT_PASSWORD").map_err(|error| match error {
        std::env::VarError::NotPresent => {
            anyhow::format_err!("Environment variable MQTT_PASSWORD not set")
        }
        std::env::VarError::NotUnicode(password) => anyhow::format_err!(
            "Environment variable MQTT_PASSWORD contains non-unicode characters: {password:?}"
        ),
    })?;

    let client = Client::new(broker_url.as_str())
        .map_err(|error| anyhow::format_err!("Error creating MQTT client: {error}"))?;
    let connection_options = ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .automatic_reconnect(Duration::from_secs(5), Duration::from_hours(1))
        .clean_session(true)
        .user_name(username.as_str())
        .password(password.as_str())
        .finalize();
    client
        .connect(connection_options)
        .map_err(|error| anyhow::format_err!("Unable to connect to broker: {error}"))?;

    // According to paho-mqtt docs, we first need to set up receiving and only afterwards subscribe to topics.
    let message_receiver = client.start_consuming();

    let topic = "home/TheengsGateway/BTtoMQTT/#";
    client
        .subscribe(topic, QoS::AtMostOnce)
        .map_err(|error| anyhow::format_err!("Unable to subscribe to topic {topic}: {error}"))?;

    let message = message_receiver
        .recv()
        .map_err(|error| anyhow::format_err!("Error receiving message: {error}"))?;

    if let Some(message) = message {
        info!("Received message: {message}");
    } else {
        info!("Received no message");
    }

    Ok(())
}
