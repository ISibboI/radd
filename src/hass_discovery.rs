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
    dev_cla: String,
    unit_of_meas: String,
    state_class: String,
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

{
    "stat_t": "+/+/BTtoMQTT/E69FBF983814",
    "dev_cla": "humidity",
    "unit_of_meas": "%",
    "state_class": "measurement",
    "name": "RuuviTag_RAWv2-hum",
    "uniq_id": "E69FBF983814-hum",
    "val_tpl": "{{ value_json.hum | is_defined }}",
    "device": {"ids": ["E69FBF983814"], "cns": [["mac", "E69FBF983814"]], "mf": "Ruuvi", "mdl": "RuuviTag_RAWv2", "name": "RuuviTag-983814", "via_device": "TheengsGateway"}
}
 */

/* Sample RuuviTag message
{"name": "Ruuvi 346C", "id": "D4:D8:D8:CB:34:6C", "rssi": -84, "brand": "Ruuvi", "model": "RuuviTag", "model_id": "RuuviTag_RAWv2", "type": "ACEL", "tempc": -19.575, "tempf": -3.235, "hum": 60.7725, "pres": 1010.48, "accx": 0.0196133, "accy": -0.0784532, "accz": -1.03558224, "volt": 2.595, "tx": 4, "mov": 33, "seq": 37604, "mac": "D4:D8:D8:CB:34:6C"}
*/

/*
Error 'expected SensorDeviceClass or one of 'date', 'enum', 'timestamp', 'absolute_humidity', 'apparent_power', 'aqi', 'area', 'atmospheric_pressure', 'battery', 'blood_glucose_concentration', 'carbon_monoxide', 'carbon_dioxide', 'conductivity', 'current', 'data_rate', 'data_size', 'distance', 'duration', 'energy', 'energy_distance', 'energy_storage', 'frequency', 'gas', 'humidity', 'illuminance', 'irradiance', 'moisture', 'monetary', 'nitrogen_dioxide', 'nitrogen_monoxide', 'nitrous_oxide', 'ozone', 'ph', 'pm1', 'pm10', 'pm25', 'pm4', 'power_factor', 'power', 'precipitation', 'precipitation_intensity', 'pressure', 'reactive_energy', 'reactive_power', 'signal_strength', 'sound_pressure', 'speed', 'sulphur_dioxide', 'temperature', 'volatile_organic_compounds', 'volatile_organic_compounds_parts', 'voltage', 'volume', 'volume_storage', 'volume_flow_rate', 'water', 'weight', 'wind_direction', 'wind_speed'
for dictionary value @ data['device_class']' when processing MQTT discovery message topic: 'homeassistant/sensor/C6CFC7267C2B-dewpoint/config',
message: '{'state_class': 'measurement', 'device': {'via_device': 'RuuviTag Additions', 'connections': [['mac', 'C6CFC7267C2B']], 'identifiers': ['C6CFC7267C2B'], 'model': 'RuuviTag_RAWv2', 'name': 'RuuviTag-267C2B', 'manufacturer': 'Ruuvi'}, 'unique_id': 'C6CFC7267C2B-dewpoint2', 'device_class': 'dewpoint', 'value_template': '{{ value_json.dewpoint | is_defined }}', 'name': 'RuuviTag_RAWv2-dewpoint', 'state_topic': 'home/Radd/RuuviTagAdditions/C6CFC7267C2B', 'unit_of_measurement': '°C'}'
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
        [("dewpoint", "absolute_humidity", "°C")].into_iter().map(
            |(measurement, device_class, unit_of_measurement)| {
                let payload = HassDiscoveryPayload {
                    stat_t: format!("home/Radd/RuuviTagAdditions/{}", self.device_id),
                    dev_cla: device_class.to_string(),
                    unit_of_meas: unit_of_measurement.to_string(),
                    state_class: "measurement".to_string(),
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
                    .topic(format!("{}{}/config", self.topic_prefix, measurement))
                    .qos(QoS::AtMostOnce)
                    .payload(payload)
                    .finalize())
            },
        )
    }
}
