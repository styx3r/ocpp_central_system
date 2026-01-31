mod api_types;

pub(crate) use api_types::MarketData;
use chrono::Local;
use config::config;

use log::info;

//-------------------------------------------------------------------------------------------------

#[derive(PartialEq, Debug)]
pub struct Period {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub average_price: f64,
}

//-------------------------------------------------------------------------------------------------

pub fn update_price_chart(config: &config::Config) -> Result<Period, Box<dyn std::error::Error>> {
    let response = reqwest::blocking::Client::new()
        .get(format!(
            "{}?start={}",
            &config.awattar.base_url,
            Local::now().timestamp_millis()
        ))
        .send()?;

    let market_data = parse_api_response(response.text()?.as_str())?;

    match find_cheapest_period(&market_data, &config) {
        Some(cheapest_period) => Ok(cheapest_period),
        _ => Err("Could not calculate cheapest period".into()),
    }
}

//-------------------------------------------------------------------------------------------------

fn parse_api_response(response: &str) -> Result<MarketData, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str::<MarketData>(response)?)
}

//-------------------------------------------------------------------------------------------------

fn find_cheapest_period(market_data: &MarketData, config: &config::Config) -> Option<Period> {
    let window_size = (config.electric_vehicle.average_watt_hours_needed as f64
        / config.charging_point.max_charging_power.ceil()) as usize;
    let sliding_windows_average = market_data
        .data
        .windows(window_size)
        .map(|e| {
            e.iter()
                .map(|e| e.marketprice)
                .collect::<Vec<f64>>()
                .iter()
                .sum::<f64>()
                / e.len() as f64
        })
        .collect::<Vec<f64>>();

    let sliding_window_min = sliding_windows_average
        .clone()
        .into_iter()
        .reduce(f64::min)
        .unwrap_or(0.0);

    let sliding_window_index = sliding_windows_average
        .iter()
        .position(|&e| e == sliding_window_min)
        .unwrap();

    let market_data_window =
        &market_data.data[sliding_window_index..sliding_window_index + window_size];

    if let Some(period_start) = market_data_window.first()
        && let Some(period_end) = market_data_window.last()
    {
        info!(
            "Found cheapest period starting at {} and ending at {} with {:.2} c/kWh",
            period_start.start_timestamp, period_end.end_timestamp, sliding_window_min / 10.0
        );

        return Some(Period {
            start_timestamp: period_start.start_timestamp,
            end_timestamp: period_end.end_timestamp,
            average_price: sliding_window_min,
        });
    }

    None
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use api_types::Data;

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

    #[test]
    fn parse_valid_api_response() -> Result<(), Box<dyn std::error::Error>> {
        let parsed_response = parse_api_response(API_RESPONSE)?;

        assert_eq!(parsed_response.object, "list");
        assert_eq!(
            parsed_response.data.first().unwrap(),
            &Data {
                start_timestamp: 1769630400000,
                end_timestamp: 1769634000000,
                marketprice: 137.02,
                unit: "Eur/MWh".to_owned()
            }
        );
        assert_eq!(
            parsed_response.data.last().unwrap(),
            &Data {
                start_timestamp: 1769709600000,
                end_timestamp: 1769713200000,
                marketprice: 161.55,
                unit: "Eur/MWh".to_owned()
            }
        );
        assert_eq!(parsed_response.url, "/at/v1/marketdata");
        Ok(())
    }

    #[test]
    fn cheapest_period_for_six_kilowatt() -> Result<(), Box<dyn std::error::Error>> {
        let parsed_response = parse_api_response(API_RESPONSE)?;

        let config = config::Config {
            websocket: config::Websocket {
                ip: "127.0.0.1".to_owned(),
                port: 8080,
            },
            charging_point: config::ChargePoint {
                serial_number: "".to_owned(),
                heartbeat_interval: 60,
                max_charging_power: 6000.0,
                default_system_voltage: 696.0,
                default_current: 16.0,
                default_cos_phi: 0.86,
                minimum_charging_current: 6.0,
                config_parameters: vec![],
            },
            id_tags: vec![],
            log_directory: "".to_owned(),
            fronius: config::Fronius {
                username: "TEST".into(),
                password: "TEST".into(),
                url: "127.0.0.1:8081".into(),
            },
            awattar: config::Awattar {
                base_url: "".to_owned(),
            },
            electric_vehicle: config::Ev {
                average_watt_hours_needed: 30000,
            },
        };

        let cheapest_period = find_cheapest_period(&parsed_response, &config);

        assert_eq!(
            cheapest_period,
            Some(Period {
                start_timestamp: 1769641200000,
                end_timestamp: 1769659200000,
                average_price: 102.744
            })
        );
        Ok(())
    }
}
