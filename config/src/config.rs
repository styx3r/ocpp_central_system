use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub websocket: Websocket,
    pub charging_point: ChargePoint,
    pub id_tags: Vec<IdTag>,
    pub log_directory: String,
    pub fronius: Fronius,
    pub awattar: Awattar,
    pub electric_vehicle: Ev,
    pub photo_voltaic: PhotoVoltaic
}

#[derive(Deserialize, Clone)]
pub struct Websocket {
    pub ip: String,
    pub port: u32,
}

#[derive(Deserialize, Clone)]
pub struct ChargePoint {
    pub serial_number: String,
    pub heartbeat_interval: u32,

    pub max_charging_power: f64,
    pub default_system_voltage: f64,
    pub default_current: f64,
    pub default_cos_phi: f64,

    pub minimum_charging_current: f64,

    pub config_parameters: Vec<ConfigSetting>,
}

#[derive(Deserialize, Clone)]
pub struct ConfigSetting {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IdTag {
    pub id: String,
    pub smart_charging: bool
}

#[derive(Deserialize, Debug, Clone)]
pub struct Fronius {
    pub username: String,
    pub password: String,
    pub url: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Awattar {
    pub base_url: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Ev {
    pub average_watt_hours_needed: u64
}

#[derive(Deserialize, Debug, Clone)]
pub struct PhotoVoltaic {
    pub moving_window_size_in_minutes: i64
}
