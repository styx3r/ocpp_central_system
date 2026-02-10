mod common;

use std::error::Error;
use std::net::TcpStream;

use awattar::Period;
use chrono::{Duration, TimeDelta, Utc};
use config::config;

use ::config::config::IdTag;
use ocpp::Decimal;
use serde::Deserialize;
use serde_json::json;
use tungstenite::{WebSocket, stream::MaybeTlsStream};

use uuid::Uuid;

use rust_ocpp::v1_6::{
    messages::set_charging_profile::SetChargingProfileRequest,
    types::{
        ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
        ChargingSchedulePeriod,
    },
};

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

fn validate_request_message(
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
            message_type: "TriggerMessage".to_owned(),
            json: json!({"requestedMessage": "StatusNotification"}),
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
            message_type: "SetChargingProfile".to_owned(),
            json: json!({"connectorId":0,"csChargingProfiles":{"chargingProfileId":3,"chargingProfileKind":"Absolute","chargingProfilePurpose":"ChargePointMaxProfile","chargingSchedule":{"chargingRateUnit":"A","chargingSchedulePeriod":[{"limit":16,"startPeriod":0}]},"stackLevel":0}}),
        },
        ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "TriggerMessage".to_owned(),
            json: json!({ "requestedMessage": "MeterValues" }),
        },
    ] {
        let uuid = validate_request_message(websocket, &expected_message)?;
        websocket.send(tungstenite::Message::text(format!(
            "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
            uuid
        )))?;
    }

    Ok(())
}

fn send_status_notification(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    payload: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StatusNotification\",{}]",
        payload
    )))?;

    match serde_json::from_str::<ExpectedJSONResponseFormat>(websocket.read()?.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(response.json, json!({}));
        }
        _ => assert!(false),
    }

    Ok(())
}

//-------------------------------------------------------------------------------------------------

static CHARGING_STATUS_NOTIFCATION: &str = r#"{
    "connectorId": 1,
    "errorCode": "NoError",
    "info": "",
    "status": "Charging",
    "timestamp": "2026-01-18T14:09:24Z",
    "vendorId": "Schneider Electric",
    "vendorErrorCode": "0.0"
}"#;

static START_TRANSACTION_REQUEST_WITH_INVALID_ID: &str = r#"{
    "connectorId": 1,
    "idTag": "INVALID_ID_TAG",
    "meterStart": 0,
    "timestamp": "2026-01-18T14:09:24Z"
}"#;

static AVAILABLE_STATUS_NOTIFCATION: &str = r#"{
    "connectorId": 1,
    "errorCode": "NoError",
    "info": "",
    "status": "Available",
    "timestamp": "2026-01-18T14:09:24Z",
    "vendorId": "Schneider Electric",
    "vendorErrorCode": "0.0"
}"#;

static SUSPENDEDEV_STATUS_NOTIFCATION: &str = r#"{
    "connectorId": 1,
    "errorCode": "NoError",
    "info": "",
    "status": "SuspendedEV",
    "timestamp": "2026-01-18T14:09:24Z",
    "vendorId": "Schneider Electric",
    "vendorErrorCode": "0.0"
}"#;

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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StartTransaction\",{}]",
        START_TRANSACTION_REQUEST_WITH_INVALID_ID
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
            smart_charging_mode: config::SmartChargingMode::Instant,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    send_status_notification(&mut websocket, AVAILABLE_STATUS_NOTIFCATION)?;
    send_status_notification(&mut websocket, CHARGING_STATUS_NOTIFCATION)?;

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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    send_status_notification(&mut websocket, AVAILABLE_STATUS_NOTIFCATION)?;

    assert!(
        !integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    );
    assert!(
        !integration_test
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 15,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    send_status_notification(&mut websocket, AVAILABLE_STATUS_NOTIFCATION)?;
    send_status_notification(&mut websocket, CHARGING_STATUS_NOTIFCATION)?;
    send_status_notification(&mut websocket, SUSPENDEDEV_STATUS_NOTIFCATION)?;

    assert!(
        integration_test
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
fn grid_based_smart_charging() -> Result<(), Box<dyn Error>> {
    static GRID_BASED_SMART_CHARGING_ID: &str = "GRID_BASED_SMART_CHARGING";
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    let config = config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: 8090,
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
        id_tags: vec![IdTag {
            id: GRID_BASED_SMART_CHARGING_ID.to_owned(),
            smart_charging_mode: config::SmartChargingMode::PVOverProductionAndGridBased,
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
        photo_voltaic: config::PhotoVoltaic {
            moving_window_size_in_minutes: 1,
        },
    };

    let mut integration_test = common::IntegrationTest::new(config);

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    let now = Utc::now();
    let start_timestamp = now + Duration::hours(1);
    let end_timestamp = now + Duration::hours(5);

    {
        let handle = integration_test.awattar_mock.clone();
        let mut guard = handle.lock().unwrap();
        guard.set_response(Period {
            start_timestamp: start_timestamp.timestamp_millis(),
            end_timestamp: end_timestamp.timestamp_millis(),
            average_price: 20.0,
        });
    }

    send_status_notification(&mut websocket, AVAILABLE_STATUS_NOTIFCATION)?;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"Authorize\",{}]",
        json!({"idTag": GRID_BASED_SMART_CHARGING_ID})
    )))?;

    let response_message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(response_message.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(
                response.json,
                json!({ "idTagInfo": { "status": "Accepted" }})
            );
        }
        _ => assert!(false),
    }

    let set_charging_profile_message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONRequestFormat>(set_charging_profile_message.to_text()?)
    {
        Ok(response) => {
            assert_eq!(response.message_id, 2);
            assert_eq!(response.message_type, "SetChargingProfile");

            let set_charging_profile_request =
                serde_json::from_value::<SetChargingProfileRequest>(response.json)?;
            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_profile_id,
                2
            );
            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_profile_purpose,
                ChargingProfilePurposeType::TxProfile
            );
            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_profile_kind,
                ChargingProfileKindType::Absolute
            );

            assert!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .valid_from
                    .is_some()
            );

            assert!(
                (set_charging_profile_request
                    .cs_charging_profiles
                    .valid_from
                    .unwrap()
                    - now)
                    .abs()
                    <= TimeDelta::milliseconds(100)
            );

            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .valid_to
                    .unwrap()
                    .timestamp_millis(),
                end_timestamp.timestamp_millis()
            );

            assert!(
                (Duration::seconds(
                    set_charging_profile_request
                        .cs_charging_profiles
                        .charging_schedule
                        .duration
                        .unwrap() as i64
                ) - Duration::hours(5))
                .abs()
                    <= TimeDelta::seconds(1)
            );

            assert!(
                (set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .start_schedule
                    .unwrap()
                    - now)
                    .abs()
                    <= TimeDelta::milliseconds(100)
            );

            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .charging_rate_unit,
                ChargingRateUnitType::A
            );

            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .charging_schedule_period,
                vec![
                    ChargingSchedulePeriod {
                        start_period: 0,
                        limit: Decimal::new(0, 0),
                        number_phases: None
                    },
                    ChargingSchedulePeriod {
                        start_period: ((start_timestamp - now).num_seconds() - 1) as i32,
                        limit: Decimal::new(16, 0),
                        number_phases: None
                    },
                ]
            );
            websocket.send(tungstenite::Message::text(format!(
                "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                response.uuid
            )))?;
        }
        _ => assert!(false),
    }

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StartTransaction\",{}]",
        json!({
            "connectorId": 1,
            "idTag": GRID_BASED_SMART_CHARGING_ID,
            "meterStart": 0,
            "timestamp": "2026-01-18T14:09:24Z"
        })
    )))?;

    let start_transaction_response = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(start_transaction_response.to_text()?)
    {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(
                response.json,
                json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1})
            );
        }
        _ => assert!(false),
    }

    send_status_notification(&mut websocket, CHARGING_STATUS_NOTIFCATION)?;

    assert!(
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    );

    static METER_VALUES_REQUEST: &str = r#"{
        "connectorId": 1,
        "transactionId": 1,
        "meterValue": [
        {
            "timestamp": "2026-01-26T05:06:21Z",
            "sampledValue": [
                {
                    "value": "219",
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Voltage",
                    "phase": "L1",
                    "location": "Outlet",
                    "unit": "V"
                },
                {
                    "value": "219",
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Voltage",
                    "phase": "L2",
                    "location": "Outlet",
                    "unit": "V"
                },
                {
                    "value": "219",
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Voltage",
                    "phase": "L3",
                    "location": "Outlet",
                    "unit": "V"
                },
                {
                    "value": "11000",
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Power.Offered",
                    "location": "Outlet",
                    "unit": "kW"
                },
                {
                    "value": "16",
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Current.Offered",
                    "location": "Outlet",
                    "unit": "A"
                }
            ]
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

    let set_grid_based_smart_charging_profile_message = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONRequestFormat>(
        set_grid_based_smart_charging_profile_message.to_text()?,
    ) {
        Ok(response) => {
            assert_eq!(response.message_id, 2);
            assert_eq!(response.message_type, "SetChargingProfile");

            let set_charging_profile_request =
                serde_json::from_value::<SetChargingProfileRequest>(response.json)?;
            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_profile_id,
                2
            );
            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_profile_purpose,
                ChargingProfilePurposeType::TxProfile
            );
            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_profile_kind,
                ChargingProfileKindType::Absolute
            );

            assert!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .valid_from
                    .is_some()
            );

            assert!(
                (set_charging_profile_request
                    .cs_charging_profiles
                    .valid_from
                    .unwrap()
                    - now)
                    .abs()
                    <= TimeDelta::milliseconds(100)
            );

            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .valid_to
                    .unwrap()
                    .timestamp_millis(),
                end_timestamp.timestamp_millis()
            );

            assert!(
                (Duration::seconds(
                    set_charging_profile_request
                        .cs_charging_profiles
                        .charging_schedule
                        .duration
                        .unwrap() as i64
                ) - Duration::hours(5))
                .abs()
                    <= TimeDelta::seconds(1)
            );

            assert!(
                (set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .start_schedule
                    .unwrap()
                    - now)
                    .abs()
                    <= TimeDelta::milliseconds(100)
            );

            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .charging_rate_unit,
                ChargingRateUnitType::A
            );

            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .charging_schedule_period,
                vec![
                    ChargingSchedulePeriod {
                        start_period: 0,
                        limit: Decimal::new(0, 0),
                        number_phases: None
                    },
                    ChargingSchedulePeriod {
                        start_period: ((start_timestamp - now).num_seconds() - 1) as i32,
                        limit: Decimal::new(6, 0),
                        number_phases: None
                    },
                ]
            );

            websocket.send(tungstenite::Message::text(format!(
                "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                response.uuid
            )))?;
        }
        _ => assert!(false),
    }

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StopTransaction\",{}]",
        json!({"meterStop": 253580, "reason": "EVDisconnected", "timestamp": "2026-02-04T05:39:05Z", "transactionId": 1})
    )))?;

    let stop_transaction_response = websocket.read()?;
    match serde_json::from_str::<ExpectedJSONResponseFormat>(stop_transaction_response.to_text()?) {
        Ok(response) => {
            assert_eq!(response.message_id, 3);
            assert_eq!(response.uuid, "12345");
            assert_eq!(response.json, json!({"idTagInfo": {"status": "Accepted"}}));
        }
        _ => assert!(false),
    }

    send_status_notification(&mut websocket, AVAILABLE_STATUS_NOTIFCATION)?;

    validate_request_message(
        &mut websocket,
        &ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({"connectorId": 1, "chargingProfilePurpose": "TxProfile", "id": 2, "stackLevel": 0}),
        },
    )?;

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
