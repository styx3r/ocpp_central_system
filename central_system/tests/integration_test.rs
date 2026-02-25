mod common;

use std::{collections::HashMap, error::Error, net::TcpStream, vec};

use awattar::Period;
use chrono::{Duration, TimeDelta, Utc};
use config::config;

use ::config::config::IdTag;
use fronius::{
    Data, PowerFlowRealtimeData, PowerFlowRealtimeDataBody, PowerFlowRealtimeDataHeader, Site,
    Smartloads, Status,
};
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

#[derive(Deserialize, Debug)]
struct ExpectedJSONRequestFormat {
    message_id: u32,
    uuid: String,
    message_type: String,
    json: serde_json::Value,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
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

fn validate_response_message(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    expected_message: &ExpectedJSONResponseFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match serde_json::from_str::<ExpectedJSONResponseFormat>(websocket.read()?.to_text()?) {
        Ok(response) => {
            assert_eq!(response, *expected_message);
        }
        _ => assert!(false),
    }

    Ok(())
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

fn validate_initial_messages_with_config_parameters(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    config_parameters: &Vec<config::ConfigSetting>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut expected_messages = vec![
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
    ];

    config_parameters.iter().for_each(|config_parameter| {
        expected_messages.insert(
            expected_messages.len() - 1,
            ExpectedJSONRequestFormat {
                message_id: 2,
                uuid: "".to_owned(),
                message_type: "ChangeConfiguration".to_owned(),
                json: json!({ "key": config_parameter.key.clone(), "value": config_parameter.value.clone() }),
            },
        );
    });

    for expected_message in expected_messages {
        let uuid = validate_request_message(websocket, &expected_message)?;
        websocket.send(tungstenite::Message::text(format!(
            "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
            uuid
        )))?;
    }

    Ok(())
}

//-------------------------------------------------------------------------------------------------

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

fn default_config(websocket_port: u32, id_tags: Vec<IdTag>) -> config::Config {
    let log_directory = format!("/tmp/integration_tests/{}", Uuid::new_v4());
    config::Config {
        websocket: config::Websocket {
            ip: "127.0.0.1".to_owned(),
            port: websocket_port,
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
        id_tags,
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
    }
}

//-------------------------------------------------------------------------------------------------

fn default_powerflow_realtime_data(
    p_load: Option<f64>,
    p_akku: Option<f64>,
    p_pv: Option<f64>,
) -> PowerFlowRealtimeData {
    PowerFlowRealtimeData {
        body: PowerFlowRealtimeDataBody {
            data: Data {
                inverters: HashMap::default(),
                site: Site {
                    mode: String::default(),
                    battery_standby: false,
                    backup_mode: false,
                    p_grid: None,
                    p_load,
                    p_akku,
                    p_pv,
                    rel_self_consumption: None,
                    rel_autonomy: None,
                    meter_location: String::default(),
                    e_day: None,
                    e_year: None,
                    e_total: None,
                },
                smartloads: Smartloads {
                    ohmpilots: HashMap::default(),
                    ohmpilot_ecos: HashMap::default(),
                },
                secondart_meters: HashMap::default(),
                version: String::default(),
            },
        },
        head: PowerFlowRealtimeDataHeader {
            request_arguments: HashMap::default(),
            status: Status {
                code: 0,
                reason: String::default(),
                user_message: String::default(),
            },
            timestamp: Utc::now(),
        },
    }
}

fn build_meter_values_request(
    voltage_per_phase: (f64, f64, f64),
    power_offered: f64,
    current_offered: f64,
) -> serde_json::Value {
    json!({
        "connectorId": 1,
        "transactionId": 1,
        "meterValue": [
        {
            "timestamp": "2026-01-26T05:06:21Z",
            "sampledValue": [
                {
                    "value": voltage_per_phase.0.to_string(),
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Voltage",
                    "phase": "L1",
                    "location": "Outlet",
                    "unit": "V"
                },
                {
                    "value": voltage_per_phase.1.to_string(),
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Voltage",
                    "phase": "L2",
                    "location": "Outlet",
                    "unit": "V"
                },
                {
                    "value": voltage_per_phase.2.to_string(),
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Voltage",
                    "phase": "L3",
                    "location": "Outlet",
                    "unit": "V"
                },
                {
                    "value": power_offered.to_string(),
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Power.Offered",
                    "location": "Outlet",
                    "unit": "kW"
                },
                {
                    "value": current_offered.to_string(),
                    "context": "Sample.Periodic",
                    "format": "Raw",
                    "measurand": "Current.Offered",
                    "location": "Outlet",
                    "unit": "A"
                }
            ]
        } ]
    })
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
    let config = default_config(8080, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    static AUTHORIZE_REQUEST: &str = r#"{"idTag": "1"}"#;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"Authorize\",{}]",
        AUTHORIZE_REQUEST
    )))?;

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({"idTagInfo": { "status": "Blocked" }}),
        },
    )?;

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn boot_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8081, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn meter_values_request() -> Result<(), Box<dyn Error>> {
    let config = default_config(8082, vec![]);

    let mut integration_test = common::IntegrationTest::new(config.clone());
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

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({}),
        },
    )?;

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn start_transaction_blocked() -> Result<(), Box<dyn Error>> {
    let config = default_config(8083, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());

    let mut websocket = integration_test.setup();
    validate_initial_messages(&mut websocket)?;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StartTransaction\",{}]",
        START_TRANSACTION_REQUEST_WITH_INVALID_ID
    )))?;

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({ "idTagInfo": { "status": "Invalid"}, "transactionId": 1 }),
        },
    )?;

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn start_transaction_accepted() -> Result<(), Box<dyn Error>> {
    let config = default_config(
        8084,
        vec![config::IdTag {
            id: "VALID_ID_TAG".to_string(),
            smart_charging_mode: config::SmartChargingMode::Instant,
        }],
    );
    let mut integration_test = common::IntegrationTest::new(config.clone());

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

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({ "idTagInfo": { "status": "Accepted"}, "transactionId": 1 }),
        },
    )?;

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn charging_status_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8085, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());

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
    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn stop_transaction_blocked() -> Result<(), Box<dyn Error>> {
    let config = default_config(8086, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());

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

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({ "idTagInfo": { "status": "Invalid" } }),
        },
    )?;

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn heartbeat() -> Result<(), Box<dyn Error>> {
    let config = default_config(8087, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());

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

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn available_status_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8088, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());

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
    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn suspendedev_status_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8089, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());

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
    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn grid_based_smart_charging() -> Result<(), Box<dyn Error>> {
    static GRID_BASED_SMART_CHARGING_ID: &str = "GRID_BASED_SMART_CHARGING";
    let config = default_config(
        8090,
        vec![IdTag {
            id: GRID_BASED_SMART_CHARGING_ID.to_owned(),
            smart_charging_mode: config::SmartChargingMode::PVOverProductionAndGridBased,
        }],
    );

    let mut integration_test = common::IntegrationTest::new(config.clone());

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

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({"idTagInfo": { "status": "Accepted" }}),
        },
    )?;

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

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1}),
        },
    )?;

    send_status_notification(&mut websocket, CHARGING_STATUS_NOTIFCATION)?;

    assert!(
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    );

    // Sending updated MeterValues request to simulate a change of cos(phi)
    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"MeterValues\",{}]",
        build_meter_values_request((180.0, 180.0, 180.0), 11.0, 5.0)
    )))?;

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({}),
        },
    )?;

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

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({"idTagInfo": {"status": "Accepted"}}),
        },
    )?;

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

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn grid_based_smart_charging_with_pv_overproduction() -> Result<(), Box<dyn Error>> {
    static GRID_BASED_SMART_CHARGING_ID: &str = "GRID_BASED_SMART_CHARGING";
    let mut config = default_config(
        8091,
        vec![IdTag {
            id: GRID_BASED_SMART_CHARGING_ID.to_owned(),
            smart_charging_mode: config::SmartChargingMode::PVOverProductionAndGridBased,
        }],
    );

    // Setting ChargingPoint interval to 60s and PV moving window size to 1minute for the sake of
    // the test.
    let config_setting = config::ConfigSetting {
        key: "MeterValueSampleInterval".to_owned(),
        value: "60".to_owned(),
    };
    config
        .charging_point
        .config_parameters
        .push(config_setting.clone());
    config.photo_voltaic.moving_window_size_in_minutes = 1;

    let mut integration_test = common::IntegrationTest::new(config.clone());

    let mut websocket = integration_test.setup();
    validate_initial_messages_with_config_parameters(
        &mut websocket,
        &vec![config_setting],
    )?;

    let now = Utc::now();
    let start_timestamp = now + Duration::hours(1);
    let end_timestamp = now + Duration::hours(5);

    {
        integration_test
            .awattar_mock
            .lock()
            .unwrap()
            .set_response(Period {
                start_timestamp: start_timestamp.timestamp_millis(),
                end_timestamp: end_timestamp.timestamp_millis(),
                average_price: 20.0,
            });

        // Simulating a load of 400W where 100W are used to charge the battery.
        // PV production is set to 14kW.
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(default_powerflow_realtime_data(
            Some(-300.0),
            Some(-100.0),
            Some(14000.0),
        ));
    }

    send_status_notification(&mut websocket, AVAILABLE_STATUS_NOTIFCATION)?;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"Authorize\",{}]",
        json!({"idTag": GRID_BASED_SMART_CHARGING_ID})
    )))?;

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({"idTagInfo": { "status": "Accepted" }}),
        },
    )?;

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

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1}),
        },
    )?;

    send_status_notification(&mut websocket, CHARGING_STATUS_NOTIFCATION)?;

    assert!(
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    );

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"MeterValues\",{}]",
        build_meter_values_request((219.0, 219.0, 219.0), 11.0, 16.0)
    )))?;

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({}),
        },
    )?;

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
                5
            );
            assert_eq!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .stack_level,
                1
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
                ChargingProfileKindType::Relative
            );

            assert!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .valid_from
                    .is_none()
            );

            assert!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .valid_to
                    .is_none()
            );

            assert!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .duration
                    .is_none()
            );

            assert!(
                set_charging_profile_request
                    .cs_charging_profiles
                    .charging_schedule
                    .start_schedule
                    .is_none()
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
                vec![ChargingSchedulePeriod {
                    start_period: 0,
                    limit: Decimal::new(16, 0),
                    number_phases: None
                }]
            );

            websocket.send(tungstenite::Message::text(format!(
                "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                response.uuid
            )))?;
        }
        _ => assert!(false),
    }

    {
        // Simulating a load of 400W where 100W are used to charge the battery.
        // PV production is set to 1kW which is expected to clear the PV based ChargingProfile.
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(default_powerflow_realtime_data(
            Some(-300.0),
            Some(-100.0),
            Some(1000.0),
        ));
    }

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"MeterValues\",{}]",
        build_meter_values_request((219.0, 219.0, 219.0), 11.0, 16.0)
    )))?;

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({}),
        },
    )?;
    let reques_uuid = validate_request_message(
        &mut websocket,
        &ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({"connectorId": 1, "chargingProfilePurpose": "TxProfile", "id": 5, "stackLevel": 1}),
        },
    )?;

    websocket.send(tungstenite::Message::text(format!(
        "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
        reques_uuid
    )))?;

    websocket.send(tungstenite::Message::text(format!(
        "[2,\"12345\",\"StopTransaction\",{}]",
        json!({"meterStop": 253580, "reason": "EVDisconnected", "timestamp": "2026-02-04T05:39:05Z", "transactionId": 1})
    )))?;

    validate_response_message(
        &mut websocket,
        &ExpectedJSONResponseFormat {
            message_id: 3,
            uuid: "12345".to_owned(),
            json: json!({"idTagInfo": {"status": "Accepted"}}),
        },
    )?;

    send_status_notification(&mut websocket, AVAILABLE_STATUS_NOTIFCATION)?;

    assert!(
        integration_test
            .fronius_mock
            .lock()
            .unwrap()
            .unblock_battery_called
    );

    integration_test.teardown(config.log_directory.as_str(), &mut websocket);
    Ok(())
}
