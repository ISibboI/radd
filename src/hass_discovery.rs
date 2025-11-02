use anyhow::anyhow;
use paho_mqtt::{Message, MessageBuilder, QoS};
use serde::{Deserialize, Serialize};

use crate::{config::Config, ruuvi::RuuviMessage};

pub struct HassDiscoveryMessages {
    device_id: String,
    topic_prefix: String,
    model_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HassDiscoveryPayload {
    stat_t: String,
    name: String,
    uniq_id: String,
    val_tpl: String,
    device: HassDiscoveryDevice,
}

#[derive(Debug, Serialize, Deserialize)]
struct HassDiscoveryDevice {
    ids: Vec<String>,
    cns: Vec<(String, String)>,
    mf: String,
    mdl: String,
    name: String,
    via_device: String,
}

/*
Example RuuviTag birth message

Topic: homeassistant/sensor/E08BAE6FD896-mov/config
QoS: 0

{"stat_t": "+/+/BTtoMQTT/E08BAE6FD896", "name": "RuuviTag_RAWv2-mov", "uniq_id": "E08BAE6FD896-mov", "val_tpl": "{{ value_json.mov | is_defined }}", "device": {"ids": ["E08BAE6FD896"], "cns": [["mac", "E08BAE6FD896"]], "mf": "Ruuvi", "mdl": "RuuviTag_RAWv2", "name": "RuuviTag-6FD896", "via_device": "TheengsGateway"}}
 */

/* Sample RuuviTag message
{"name": "Ruuvi 346C", "id": "D4:D8:D8:CB:34:6C", "rssi": -84, "brand": "Ruuvi", "model": "RuuviTag", "model_id": "RuuviTag_RAWv2", "type": "ACEL", "tempc": -19.575, "tempf": -3.235, "hum": 60.7725, "pres": 1010.48, "accx": 0.0196133, "accy": -0.0784532, "accz": -1.03558224, "volt": 2.595, "tx": 4, "mov": 33, "seq": 37604, "mac": "D4:D8:D8:CB:34:6C"}
*/

impl HassDiscoveryMessages {
    pub fn new(config: &Config, ruuvi_message: &RuuviMessage) -> Self {
        let device_id = ruuvi_message.id.replace(':', "");
        let topic_prefix = format!("{}/sensor/{device_id}-", config.hass_discovery_topic());

        Self {
            device_id,
            topic_prefix,
            model_id: ruuvi_message.model_id.clone(),
        }
    }

    pub fn iter_messages(&self) -> impl Iterator<Item = anyhow::Result<Message>> {
        ["dewpoint"].into_iter().map(|measurement| {
            let payload = HassDiscoveryPayload {
                stat_t: format!("home/Radd/RuuviTagAdditions/{}", self.device_id),
                name: format!("{}-{}", self.model_id, measurement),
                uniq_id: format!("{}-{}", self.device_id, measurement),
                val_tpl: format!("{{{{ value_json.{} | is_defined }}}}", measurement),
                device: HassDiscoveryDevice {
                    ids: vec![self.device_id.clone()],
                    cns: vec![("mac".to_string(), self.device_id.clone())],
                    mf: "Ruuvi".to_string(),
                    mdl: self.model_id.clone(),
                    name: format!("RuuviTag-{}", &self.device_id[6..]),
                    via_device: "RuuviTag Additions".to_string(),
                },
            };
            let payload = match serde_json::to_vec(&payload) {
                Ok(payload) => payload,
                Err(error) => {
                    return Err(anyhow!("Unable to format hass discovery message: {error}"));
                }
            };

            Ok(MessageBuilder::new()
                .topic(format!("{}-{}/config", self.topic_prefix, measurement))
                .qos(QoS::AtMostOnce)
                .payload(payload)
                .finalize())
        })
    }
}
