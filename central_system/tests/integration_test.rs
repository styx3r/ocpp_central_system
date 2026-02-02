mod common;

use std::error::Error;
use std::net::TcpStream;

use config::config;

use serde::Deserialize;
use serde_json::json;
use tungstenite::{WebSocket, stream::MaybeTlsStream};

use uuid::Uuid;

//-------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct ExpectedJSONRequestFormat {
    message_id: u32,
    uuid: String,
    message_type: String,
    json: serde_json::Value,
}

#[derive(Deserialize)]
struct ExpectedJSONResponseFormat {
    message_id: u32,
    uuid: String,
    json: serde_json::Value,
}

//-------------------------------------------------------------------------------------------------

fn validate_message(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    expected_message: &ExpectedJSONRequestFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match serde_json::from_str::<ExpectedJSONRequestFormat>(websocket.read()?.to_text()?) {
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
        ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "TriggerMessage".to_owned(),
            json: json!({"requestedMessage": "BootNotification"}),
        },
        ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({}),
        },
        ExpectedJSONRequestFormat {
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
fn authorize_request() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);
    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static AUTHORIZE_REQUEST: &str = r#"{"idTag": "1"}"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"Authorize\",{}]",
        AUTHORIZE_REQUEST
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(response.json, json!({"idTagInfo": { "status": "Blocked" }}));
        }
        _ => assert!(false),
    }

    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn boot_notification() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn meter_values_request() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8082,
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static METER_VALUES_REQUEST: &str = r#"{
        "connectorId": 1,
        "transactionId": 1,
        "meterValue": [
        {
            "timestamp": "2026-01-26T05:06:21Z",
            "sampledValue": [
            {
                "value": "50",
                "context": "Sample.Periodic",
                "format": "Raw",
                "measurand": "Voltage",
                "phase": "L1",
                "location": "Outlet",
                "unit": "V"
            } ]
        } ]
    }"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"MeterValues\",{}]",
        METER_VALUES_REQUEST
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(response.json, json!({}));
        }
        _ => assert!(false),
    }

    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn start_transaction_blocked() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8083,
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static START_TRANSACTION_REQUEST: &str = r#"{
        "connectorId": 1,
        "idTag": "INVALID_ID_TAG",
        "meterStart": 0,
        "timestamp": "2026-01-18T14:09:24Z"
    }"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StartTransaction\",{}]",
        START_TRANSACTION_REQUEST
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(
                response.json,
                json!({ "idTagInfo": { "status": "Invalid"}, "transactionId": 1 })
            );
        }
        _ => assert!(false),
    }

    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn start_transaction_accepted() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8084,
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
        id_tags: vec![config::IdTag {
            id: "VALID_ID_TAG".to_string(),
            smart_charging: false
        }],
        log_directory: log_directory.to_owned(),
        fronius: config::Fronius {
            username: "TEST".into(),
            password: "TEST".into(),
            url: "127.0.0.1:8081".into(),
        },
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static START_TRANSACTION_REQUEST: &str = r#"{
        "connectorId": 1,
        "idTag": "VALID_ID_TAG",
        "meterStart": 0,
        "timestamp": "2026-01-18T14:09:24Z"
    }"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StartTransaction\",{}]",
        START_TRANSACTION_REQUEST
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(
                response.json,
                json!({ "idTagInfo": { "status": "Accepted"}, "transactionId": 1 })
            );
        }
        _ => assert!(false),
    }

    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn charging_status_notification() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8085,
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static CHARGING_STATUS_NOTIFCATION: &str = r#"{
        "connectorId": 1,
        "errorCode": "NoError",
        "info": "",
        "status": "Charging",
        "timestamp": "2026-01-18T14:09:24Z",
        "vendorId": "Schneider Electric",
        "vendorErrorCode": "0.0"
    }"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StatusNotification\",{}]",
        CHARGING_STATUS_NOTIFCATION
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(response.json, json!({}));
        }
        _ => assert!(false),
    }

    assert!(
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    );
    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn stop_transaction_blocked() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8086,
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static STOP_TRANSACTION_REQUEST: &str = r#"{
        "idTag": "INVALID_ID_TAG",
        "meterStop": 20,
        "timestamp": "2026-01-18T14:09:24Z",
        "transactionId": 0,
        "reason": "Local"
    }"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StopTransaction\",{}]",
        STOP_TRANSACTION_REQUEST
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(
                response.json,
                json!({ "idTagInfo": { "status": "Invalid" } })
            );
        }
        _ => assert!(false),
    }

    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn heartbeat() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8087,
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"Heartbeat\",{{}}]",
    )))?;

    // Heartbeat response contains the system time which can't be checked deterministically.
    // Only way would be to work with deltas but this could be instable as well.
    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
        }
        _ => assert!(false),
    }

    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn available_status_notification() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8088,
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static CHARGING_STATUS_NOTIFCATION: &str = r#"{
        "connectorId": 1,
        "errorCode": "NoError",
        "info": "",
        "status": "Available",
        "timestamp": "2026-01-18T14:09:24Z",
        "vendorId": "Schneider Electric",
        "vendorErrorCode": "0.0"
    }"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StatusNotification\",{}]",
        CHARGING_STATUS_NOTIFCATION
    )))?;

    let message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(response.json, json!({}));
        }
        _ => assert!(false),
    }

    assert!(
        !integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    );
    assert!(
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .unblock_battery_called
    );
    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn suspendedev_status_notification() -> Result<(), Box<dyn Error>> {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8089,
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
        awattar: config::Awattar {
            base_url: "".to_owned(),
        },
        electric_vehicle: config::Ev {
            average_watt_hours_needed: 0,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static CHARGING_STATUS_NOTIFCATION: &str = r#"{
        "connectorId": 1,
        "errorCode": "NoError",
        "info": "",
        "status": "SuspendedEV",
        "timestamp": "2026-01-18T14:09:24Z",
        "vendorId": "Schneider Electric",
        "vendorErrorCode": "0.0"
    }"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StatusNotification\",{}]",
        CHARGING_STATUS_NOTIFCATION
    )))?;

    let response_message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(response_message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(response.json, json!({}));
        }
        _ => assert!(false),
    }

    assert!(
        !integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    );
    assert!(
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .unblock_battery_called
    );
    integration_test.teardown(log_directory.as_str(), &mut websocket);
    Ok(())
}
