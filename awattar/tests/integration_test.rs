use ::config::f64::*;
use httpmock::prelude::*;
use std::error::Error;

use awattar::{AwattarApi, AwattarApiAdapter};
use config::config;

fn default_config() -> config::Config {
    config::Config {
        websocket: config::Websocket {
            ip: String::new(),
            port: 0,
        },
        charging_point: config::ChargePoint {
            serial_number: String::new(),
            heartbeat_interval: 0,
            max_charging_power: Power::new::<::config::power::watt>(0.0),
            default_system_voltage: ElectricPotential::new::<::config::electric_potential::volt>(
                0.0,
            ),
            default_current: ElectricCurrent::new::<::config::electric_current::ampere>(0.0),
            default_cos_phi: 0.0,
            minimum_charging_current: ElectricCurrent::new::<::config::electric_current::ampere>(
                0.0,
            ),
            config_parameters: vec![],
        },
        id_tags: vec![config::IdTag {
            id: String::new(),
            smart_charging_mode: config::SmartChargingMode::default(),
        }],
        log_directory: String::new(),
        fronius: config::Fronius {
            username: String::new(),
            password: String::new(),
            url: String::new(),
        },
        awattar: config::Awattar {
            base_url: String::new(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: Energy::new::<::config::energy::watt_hour>(0.0),
        },
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 0,
        },
    }
}

#[test]
fn update_price_chart() -> Result<(), Box<dyn Error>> {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/v1/marketdata")
            .query_param("start", "0")
            .query_param("end", "0");

        then.status(200)
            .header("content-type", "text/json")
            .body("{}");
    });

    let awattar_api = AwattarApiAdapter::default();

    let mut config = default_config();
    config.awattar.base_url = server.url("/v1/marketdata");

    awattar_api.update_price_chart(&config)?;

    Ok(())
}
