mod common;

use std::error::Error;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use config::config;

use ocpp::{
    ChargePointState, OcppMeterValuesHook, OcppStatusNotificationHook, StatusNotificationRequest,
};

use serde::Deserialize;
use serde_json::json;
use tungstenite::{WebSocket, stream::MaybeTlsStream};

//-------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct ExpectedJSONFormat {
    message_id: u32,
    uuid: String,
    message_type: String,
    json: serde_json::Value,
}

pub struct Hook {
    pub called: bool,
}

impl Hook {
    pub fn default() -> Self {
        Self { called: false }
    }
}

impl OcppStatusNotificationHook for Hook {
    fn evaluate(
        &mut self,
        _status_notification: &StatusNotificationRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.called = true;
        Ok(())
    }
}

impl OcppMeterValuesHook for Hook {
    fn evaluate(
        &mut self,
        _charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unimplemented!("This hook needs to be implemented!");
    }
}

//-------------------------------------------------------------------------------------------------

fn validate_message(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    expected_message: &ExpectedJSONFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match serde_json::from_str::<ExpectedJSONFormat>(websocket.read()?.to_text()?) {
        Ok(request) => {
            assert_eq!(request.message_id, expected_message.message_id);
            assert_eq!(request.message_type, expected_message.message_type);
            assert_eq!(request.json, expected_message.json);

            return Ok(request.uuid);
        }
        _ => {
            assert!(false);
        }
    }

    Err("Message validation failed".into())
}

//-------------------------------------------------------------------------------------------------

fn validate_initial_messages(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for expected_message in vec![
        ExpectedJSONFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "TriggerMessage".to_owned(),
            json: json!({"requestedMessage": "BootNotification"}),
        },
        ExpectedJSONFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({}),
        },
        ExpectedJSONFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "TriggerMessage".to_owned(),
            json: json!({ "requestedMessage": "MeterValues" }),
        },
    ] {
        let uuid = validate_message(websocket, &expected_message)?;
        websocket.send(tungstenite::Message::text(format!(
            "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
            uuid
        )))?;
    }

    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn boot_notification() -> Result<(), Box<dyn Error>> {
    let log_directory = "/tmp/boot_notification";
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8080,
        },
        charging_point: config::ChargePoint {
            serial_number: "".to_owned(),
            heartbeat_interval: 60,
            max_charging_power: 11000.0,
            default_system_voltage: 696.0,
            default_current: 16.0,
            default_cos_phi: 0.86,
            minimum_charging_current: 6.0,
            config_parameters: vec![],
        },
        id_tags: vec![],
        log_directory: log_directory.to_owned(),
        fronius: config::Fronius {
            username: "TEST".into(),
            password: "TEST".into(),
            url: "127.0.0.1:8081".into(),
        },
    };

    let hook = Arc::new(Mutex::new(Hook::default()));
    let mut integration_test = common::IntegrationTest::new(config, Arc::clone(&hook));

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    integration_test.teardown(log_directory, &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn charging_status_notification() -> Result<(), Box<dyn Error>> {
    let log_directory = "/tmp/charging_status_notification";
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8081,
        },
        charging_point: config::ChargePoint {
            serial_number: "".to_owned(),
            heartbeat_interval: 60,
            max_charging_power: 11000.0,
            default_system_voltage: 696.0,
            default_current: 16.0,
            default_cos_phi: 0.86,
            minimum_charging_current: 6.0,
            config_parameters: vec![],
        },
        id_tags: vec![],
        log_directory: log_directory.to_owned(),
        fronius: config::Fronius {
            username: "TEST".into(),
            password: "TEST".into(),
            url: "127.0.0.1:8081".into(),
        },
    };

    let hook = Arc::new(Mutex::new(Hook::default()));
    let mut integration_test = common::IntegrationTest::new(config, Arc::clone(&hook));

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static CHARGING_STATUS_NOTIFCATION: &str = r#"{"connectorId": 1,"errorCode": "NoError","info": "","status": "Charging","timestamp": "2026-01-18T14:09:24Z","vendorId": "Schneider Electric","vendorErrorCode": "0.0"}"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StatusNotification\",{}]",
        CHARGING_STATUS_NOTIFCATION
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<(u32, String, serde_json::Value)>(message.to_text()?) {
        Ok((id, uuid, json)) => {
            assert_eq!(id, 3);
            assert_eq!(uuid, "12345");
            assert_eq!(json, json!({}));
        }
        _ => assert!(false),
    }

    integration_test.teardown(log_directory, &mut websocket);

    assert!(hook.lock().unwrap().called);

    Ok(())
}
