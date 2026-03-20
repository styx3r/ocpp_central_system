use ::config::f64::*;
use chrono::{Duration, Local, TimeZone, Timelike};
use httpmock::prelude::*;
use std::error::Error;

use awattar::{AwattarApi, AwattarApiAdapter, Period};
use config::config;

//-------------------------------------------------------------------------------------------------

fn default_config() -> config::Config {
    config::Config {
        websocket: config::Websocket {
            ip: String::new(),
            port: 0,
        },
        charging_point: config::ChargePoint {
            serial_number: String::new(),
            heartbeat_interval: 0,
            max_charging_power: Power::new::<::config::power::watt>(6000.0),
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
            average_watt_hours_needed: Energy::new::<::config::energy::watt_hour>(30000.0),
        },
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 0,
        },
    }
}

//-------------------------------------------------------------------------------------------------

static API_RESPONSE: &str = r#"{
  "object": "list",
  "data": [
    {
      "start_timestamp": 1769630400000,
      "end_timestamp": 1769634000000,
      "marketprice": 137.02,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769634000000,
      "end_timestamp": 1769637600000,
      "marketprice": 130.06,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769637600000,
      "end_timestamp": 1769641200000,
      "marketprice": 114.87,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769641200000,
      "end_timestamp": 1769644800000,
      "marketprice": 105.12,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769644800000,
      "end_timestamp": 1769648400000,
      "marketprice": 104.06,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769648400000,
      "end_timestamp": 1769652000000,
      "marketprice": 102.03,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769652000000,
      "end_timestamp": 1769655600000,
      "marketprice": 100.63,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769655600000,
      "end_timestamp": 1769659200000,
      "marketprice": 101.88,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769659200000,
      "end_timestamp": 1769662800000,
      "marketprice": 120.54,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769662800000,
      "end_timestamp": 1769666400000,
      "marketprice": 140.01,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769666400000,
      "end_timestamp": 1769670000000,
      "marketprice": 163.24,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769670000000,
      "end_timestamp": 1769673600000,
      "marketprice": 174.81,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769673600000,
      "end_timestamp": 1769677200000,
      "marketprice": 175.89,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769677200000,
      "end_timestamp": 1769680800000,
      "marketprice": 172.45,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769680800000,
      "end_timestamp": 1769684400000,
      "marketprice": 173.63,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769684400000,
      "end_timestamp": 1769688000000,
      "marketprice": 164.98,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769688000000,
      "end_timestamp": 1769691600000,
      "marketprice": 165.35,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769691600000,
      "end_timestamp": 1769695200000,
      "marketprice": 159.98,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769695200000,
      "end_timestamp": 1769698800000,
      "marketprice": 158.88,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769698800000,
      "end_timestamp": 1769702400000,
      "marketprice": 162.66,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769702400000,
      "end_timestamp": 1769706000000,
      "marketprice": 165.48,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769706000000,
      "end_timestamp": 1769709600000,
      "marketprice": 163.07,
      "unit": "Eur/MWh"
    },
    {
      "start_timestamp": 1769709600000,
      "end_timestamp": 1769713200000,
      "marketprice": 161.55,
      "unit": "Eur/MWh"
    }
  ],
  "url": "/at/v1/marketdata"
}"#;

//-------------------------------------------------------------------------------------------------

#[test]
fn update_price_chart() -> Result<(), Box<dyn Error>> {
    let server = MockServer::start();
    let marketdata = server.mock(|when, then| {
        when.method(GET).path("/v1/marketdata").is_true(|req| {
            let query_params_map = req.query_params_map();
            let start_param = query_params_map
                .get("start")
                .expect("Missing 'start' param")
                .parse::<i64>()
                .expect("Expected i64!");
            let end_param = query_params_map
                .get("end")
                .expect("Missing 'end' param")
                .parse::<i64>()
                .expect("Expected u64!");

            let end_hour = Local.timestamp_millis_opt(end_param).unwrap().hour();

            return Duration::milliseconds(Local::now().timestamp_millis() - start_param)
                <= Duration::milliseconds(20)
                && Duration::milliseconds(end_param - start_param) <= Duration::hours(24)
                && end_hour == 6;
        });

        then.status(200)
            .header("content-type", "application/json")
            .body(API_RESPONSE);
    });

    let mut config = default_config();
    config.awattar.base_url = server.url("/v1/marketdata");

    let awattar_api = AwattarApiAdapter::default();

    assert_eq!(
        awattar_api.update_price_chart(&config)?,
        Period {
            start_timestamp: 1769641200000,
            end_timestamp: 1769659200000,
            average_price: 102.744
        }
    );
    marketdata.assert();

    Ok(())
}
