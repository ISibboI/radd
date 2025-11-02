use std::fmt::Display;

use anyhow::anyhow;
use paho_mqtt::{Message, MessageBuilder, QoS};
use serde::{Deserialize, Serialize};

use crate::dewpoint::compute_dewpoint;

/* Sample RuuviTag message
{"name": "Ruuvi 346C", "id": "D4:D8:D8:CB:34:6C", "rssi": -84, "brand": "Ruuvi", "model": "RuuviTag", "model_id": "RuuviTag_RAWv2", "type": "ACEL", "tempc": -19.575, "tempf": -3.235, "hum": 60.7725, "pres": 1010.48, "accx": 0.0196133, "accy": -0.0784532, "accz": -1.03558224, "volt": 2.595, "tx": 4, "mov": 33, "seq": 37604, "mac": "D4:D8:D8:CB:34:6C"}
*/

#[derive(Debug, Serialize, Deserialize)]
pub struct RuuviMessage {
    #[serde(default)]
    pub topic: String,
    pub name: String,
    pub id: String,
    pub brand: String,
    pub model: String,
    pub model_id: String,
    pub mac: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "tempc")]
    pub temperature_celsius: f32,
    #[serde(rename = "hum")]
    pub relative_humidity_percent: f32,
    #[serde(rename = "pres")]
    pub pressure_millibar: f32,

    #[serde(flatten)]
    pub other_data: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuuviAdditionalMessage {
    #[serde(skip)]
    topic: String,
    name: String,
    id: String,
    brand: String,
    model: String,
    model_id: String,
    pub mac: String,
    #[serde(rename = "type")]
    r#type: String,

    #[serde(rename = "dewpoint")]
    dewpoint_celsius: f32,
}

impl TryFrom<Message> for RuuviMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let topic = message.topic();
        let mut ruuvi_message: RuuviMessage =
            serde_json::from_slice(message.payload()).map_err(|error| {
                anyhow!(
                    "Cannot parse message as RuuviMessage: {error}\n{}",
                    message.payload_str()
                )
            })?;
        ruuvi_message.topic = topic.to_owned();
        Ok(ruuvi_message)
    }
}

impl RuuviMessage {
    pub fn create_additional_message(&self) -> RuuviAdditionalMessage {
        RuuviAdditionalMessage {
            topic: format!("home/Radd/RuuviTagAdditions/{}", self.id.replace(':', "")),
            name: self.name.clone(),
            id: self.id.clone(),
            brand: self.brand.clone(),
            model: self.model.clone(),
            model_id: self.model_id.clone(),
            mac: self.mac.clone(),
            r#type: self.r#type.clone(),

            dewpoint_celsius: compute_dewpoint(
                self.temperature_celsius,
                self.relative_humidity_percent,
                self.pressure_millibar,
            ),
        }
    }
}

impl RuuviAdditionalMessage {
    pub fn to_message(&self) -> anyhow::Result<Message> {
        let payload = match serde_json::to_vec(self) {
            Ok(payload) => payload,
            Err(error) => {
                return Err(anyhow!(
                    "Unable to format Ruuvi additional message: {error}"
                ));
            }
        };
        Ok(MessageBuilder::new()
            .topic(&self.topic)
            .qos(QoS::AtMostOnce)
            .payload(payload)
            .finalize())
    }
}

impl Display for RuuviMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({}; {}) T={}°C Rh={}% p={}mbar",
            self.topic,
            self.name,
            self.model_id,
            self.r#type,
            self.temperature_celsius,
            self.relative_humidity_percent,
            self.pressure_millibar
        )
    }
}

impl Display for RuuviAdditionalMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({}; {}) Dewpoint={}°C",
            self.topic, self.name, self.model_id, self.r#type, self.dewpoint_celsius
        )
    }
}
