use anyhow::anyhow;

pub struct Config {
    log_level: String,

    broker_url: String,
    username: String,
    password: String,

    listen_topic: String,
    hass_discovery_topic: String,
}

impl Config {
    fn read_env_var(var: &str) -> anyhow::Result<String> {
        std::env::var(var).map_err(|error| match error {
            std::env::VarError::NotPresent => {
                anyhow!("Environment variable {var} not set")
            }
            std::env::VarError::NotUnicode(value) => {
                anyhow!("Environment variable {var} contains non-unicode characters: {value:?}")
            }
        })
    }

    pub fn parse_env() -> anyhow::Result<Self> {
        let log_level = Self::read_env_var("LOG_LEVEL")?;
        let broker_url = Self::read_env_var("MQTT_BROKER_URL")?;
        let username = Self::read_env_var("MQTT_USERNAME")?;
        let password = Self::read_env_var("MQTT_PASSWORD")?;
        let listen_topic = Self::read_env_var("MQTT_LISTEN_TOPIC")?;
        let hass_discovery_topic = Self::read_env_var("MQTT_HASS_DISCOVERY_TOPIC")?;

        let hass_discovery_topic = hass_discovery_topic
            .trim_end_matches(|c| "+#/".contains(c))
            .to_owned();

        Ok(Self {
            log_level,

            broker_url,
            username,
            password,

            listen_topic,
            hass_discovery_topic,
        })
    }

    pub fn log_level(&self) -> &str {
        &self.log_level
    }

    pub fn broker_url(&self) -> &str {
        &self.broker_url
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn listen_topic(&self) -> &str {
        &self.listen_topic
    }

    /// Topic for Home Assistant discovery without trailing /, + or #.
    pub fn hass_discovery_topic(&self) -> &str {
        &self.hass_discovery_topic
    }
}
