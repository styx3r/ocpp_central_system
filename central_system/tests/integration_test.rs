mod common;

use std::{error::Error, vec};

use awattar::Period;
use chrono::{Duration, Utc};
use config::config;

use ::config::config::IdTag;
use ocpp::Decimal;
use serde_json::json;

use uuid::Uuid;

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

static CHARGING_STATUS_NOTIFCATION: &str = "Charging";
static AVAILABLE_STATUS_NOTIFCATION: &str = "Available";
static SUSPENDEDEV_STATUS_NOTIFCATION: &str = "SuspendedEV";

//-------------------------------------------------------------------------------------------------

#[test]
fn authorize_request() -> Result<(), Box<dyn Error>> {
    let config = default_config(8080, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_authorize_request("1")?;
    integration_test.validate_response_message(json!({"idTagInfo": { "status": "Blocked" }}))?;

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn boot_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8081, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;
    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn meter_values_request() -> Result<(), Box<dyn Error>> {
    let config = default_config(8082, vec![]);

    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_meter_value_readings(
        (230.0, 230.0, 230.0),
        11000.0,
        16.0,
        (0.0, 0.0, 0.0),
    )?;
    integration_test.validate_response_message(json!({}))?;

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn start_transaction_blocked() -> Result<(), Box<dyn Error>> {
    let config = default_config(8083, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_start_transaction_request("INVALID_ID_TAG")?;
    integration_test.validate_response_message(
        json!({ "idTagInfo": { "status": "Invalid"}, "transactionId": 1 }),
    )?;

    integration_test.teardown(config.log_directory.as_str());
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
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_start_transaction_request("VALID_ID_TAG")?;
    integration_test.validate_response_message(
        json!({ "idTagInfo": { "status": "Accepted"}, "transactionId": 1 }),
    )?;

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn charging_status_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8085, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;
    integration_test.send_status_notification(CHARGING_STATUS_NOTIFCATION)?;

    assert!(integration_test.block_battery_for_duration_called());
    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn stop_transaction_blocked() -> Result<(), Box<dyn Error>> {
    let config = default_config(8086, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_stop_transaction_request()?;
    integration_test.validate_response_message(json!({ "idTagInfo": { "status": "Invalid" } }))?;

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn heartbeat() -> Result<(), Box<dyn Error>> {
    let config = default_config(8087, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_heartbeat_request()?;
    integration_test.validate_heartbeat_response()?;

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn available_status_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8088, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;

    assert!(!integration_test.block_battery_for_duration_called());
    assert!(!integration_test.unblock_battery_called());
    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn suspendedev_status_notification() -> Result<(), Box<dyn Error>> {
    let config = default_config(8089, vec![]);
    let mut integration_test = common::IntegrationTest::new(config.clone());
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;
    integration_test.send_status_notification(CHARGING_STATUS_NOTIFCATION)?;
    integration_test.send_status_notification(SUSPENDEDEV_STATUS_NOTIFCATION)?;

    assert!(integration_test.block_battery_for_duration_called());
    assert!(integration_test.unblock_battery_called());
    integration_test.teardown(config.log_directory.as_str());
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
    integration_test.setup();
    integration_test.validate_initial_messages()?;

    let now = Utc::now();
    let start_timestamp = now + Duration::hours(1);
    let end_timestamp = now + Duration::hours(5);

    integration_test.set_awattar_response(Period {
        start_timestamp: start_timestamp.timestamp_millis(),
        end_timestamp: end_timestamp.timestamp_millis(),
        average_price: 20.0,
    });

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;
    integration_test.send_authorize_request(GRID_BASED_SMART_CHARGING_ID)?;
    integration_test.validate_response_message(json!({"idTagInfo": { "status": "Accepted" }}))?;

    integration_test.validate_grid_based_profile(
        now,
        start_timestamp,
        end_timestamp,
        Decimal::new(16, 0),
    )?;
    integration_test.send_start_transaction_request(GRID_BASED_SMART_CHARGING_ID)?;

    integration_test.validate_response_message(
        json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1}),
    )?;

    integration_test.send_status_notification(CHARGING_STATUS_NOTIFCATION)?;

    assert!(integration_test.block_battery_for_duration_called());

    // Sending updated MeterValues request to simulate a change of cos(phi)
    integration_test.send_meter_value_readings(
        (180.0, 180.0, 180.0),
        11.0,
        5.0,
        (0.0, 0.0, 0.0),
    )?;

    integration_test.validate_response_message(json!({}))?;

    integration_test.validate_grid_based_profile(
        now,
        start_timestamp,
        end_timestamp,
        Decimal::new(6, 0),
    )?;
    integration_test.send_stop_transaction_request()?;
    integration_test.validate_response_message(json!({"idTagInfo": {"status": "Accepted"}}))?;

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;

    integration_test.validate_request_message(
        &common::ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({"connectorId": 1, "chargingProfilePurpose": "TxProfile", "id": 2, "stackLevel": 0}),
        },
    )?;

    assert!(integration_test.unblock_battery_called());

    integration_test.teardown(config.log_directory.as_str());
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

    integration_test.setup();
    integration_test.validate_initial_messages_with_config_parameters(&vec![config_setting])?;

    let now = Utc::now();
    let start_timestamp = now + Duration::hours(1);
    let end_timestamp = now + Duration::hours(5);

    integration_test.set_awattar_response(Period {
        start_timestamp: start_timestamp.timestamp_millis(),
        end_timestamp: end_timestamp.timestamp_millis(),
        average_price: 20.0,
    });

    // Simulating a load of 400W where 100W are used to charge the battery.
    // PV production is set to 14kW.
    integration_test.set_power_flow_realtime_data(-300.0, -100.0, 14000.0);

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;
    integration_test.send_authorize_request(GRID_BASED_SMART_CHARGING_ID)?;

    integration_test.validate_response_message(json!({"idTagInfo": { "status": "Accepted" }}))?;
    integration_test.validate_grid_based_profile(
        now,
        start_timestamp,
        end_timestamp,
        Decimal::new(16, 0),
    )?;
    integration_test.send_start_transaction_request(GRID_BASED_SMART_CHARGING_ID)?;
    integration_test.validate_response_message(
        json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1}),
    )?;
    integration_test.send_status_notification(CHARGING_STATUS_NOTIFCATION)?;

    assert!(integration_test.block_battery_for_duration_called());

    integration_test.set_awattar_response(Period {
        start_timestamp: start_timestamp.timestamp_millis(),
        end_timestamp: end_timestamp.timestamp_millis(),
        average_price: 20.0,
    });

    // Simulating a load of 11400W where 100W are used to charge the battery.
    // PV production is set to 14kW.
    integration_test.set_power_flow_realtime_data(-11300.0, -100.0, 14000.0);

    integration_test.send_meter_value_readings(
        (219.0, 219.0, 219.0),
        11.0,
        16.0,
        (3.7, 3.7, 3.7),
    )?;

    integration_test.validate_response_message(json!({}))?;

    integration_test.validate_pv_based_profile(Decimal::new(16, 0))?;

    // Simulating a load of 11400W where 100W are used to charge the battery.
    // PV production is set to 1kW which is expected to clear the PV based ChargingProfile.
    integration_test.set_power_flow_realtime_data(-11300.0, -100.0, 1000.0);
    integration_test.send_meter_value_readings(
        (219.0, 219.0, 219.0),
        11.0,
        16.0,
        (3.7, 3.7, 3.7),
    )?;

    integration_test.validate_response_message(json!({}))?;
    let request_uuid = integration_test.validate_request_message(
        &common::ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({"connectorId": 1, "chargingProfilePurpose": "TxProfile", "id": 5, "stackLevel": 1}),
        },
    )?;

    integration_test.send_accepted_response(request_uuid.as_str())?;
    integration_test.send_stop_transaction_request()?;
    integration_test.validate_response_message(json!({"idTagInfo": {"status": "Accepted"}}))?;

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;

    assert!(integration_test.unblock_battery_called());

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn pv_smart_charging_with_pv_overproduction() -> Result<(), Box<dyn Error>> {
    static PV_SMART_CHARGING_ID: &str = "PV_SMART_CHARGING";
    let mut config = default_config(
        8092,
        vec![IdTag {
            id: PV_SMART_CHARGING_ID.to_owned(),
            smart_charging_mode: config::SmartChargingMode::PVOverProduction,
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

    integration_test.setup();
    integration_test.validate_initial_messages_with_config_parameters(&vec![config_setting])?;

    // Simulating a load of 400W where 100W are used to charge the battery.
    // PV production is set to 14kW.
    integration_test.set_power_flow_realtime_data(-300.0, -100.0, 1000.0);
    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;
    integration_test.send_authorize_request(PV_SMART_CHARGING_ID)?;

    integration_test.validate_response_message(json!({"idTagInfo": { "status": "Accepted" }}))?;
    integration_test.validate_pv_preparation_profile(Utc::now())?;
    integration_test.send_start_transaction_request(PV_SMART_CHARGING_ID)?;
    integration_test.validate_response_message(
        json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1}),
    )?;

    integration_test.send_status_notification(CHARGING_STATUS_NOTIFCATION)?;

    assert!(integration_test.block_battery_for_duration_called());

    // Simulating a load of 11400W where 100W are used to charge the battery.
    // PV production is set to 14kW.
    integration_test.set_power_flow_realtime_data(-11300.0, -100.0, 14000.0);
    integration_test.send_meter_value_readings(
        (219.0, 219.0, 219.0),
        11.0,
        16.0,
        (3.7, 3.7, 3.7),
    )?;

    integration_test.validate_response_message(json!({}))?;
    integration_test.validate_pv_based_profile(Decimal::new(16, 0))?;

    // Simulating a load of 11400W where 100W are used to charge the battery.
    // PV production is set to 1kW which is expected to clear the PV based ChargingProfile.
    integration_test.set_power_flow_realtime_data(-11300.0, -100.0, 1000.0);
    integration_test.send_meter_value_readings(
        (219.0, 219.0, 219.0),
        11.0,
        16.0,
        (3.7, 3.7, 3.7),
    )?;

    integration_test.validate_response_message(json!({}))?;
    let request_uuid = integration_test.validate_request_message(
        &common::ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({"connectorId": 1, "chargingProfilePurpose": "TxProfile", "id": 5, "stackLevel": 1}),
        },
    )?;

    integration_test.send_accepted_response(request_uuid.as_str())?;
    integration_test.send_stop_transaction_request()?;
    integration_test.validate_response_message(json!({"idTagInfo": {"status": "Accepted"}}))?;
    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;

    assert!(integration_test.unblock_battery_called());

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[test]
fn repeated_pv_smart_charging_with_pv_overproduction() -> Result<(), Box<dyn Error>> {
    static PV_SMART_CHARGING_ID: &str = "PV_SMART_CHARGING";
    let mut config = default_config(
        8093,
        vec![IdTag {
            id: PV_SMART_CHARGING_ID.to_owned(),
            smart_charging_mode: config::SmartChargingMode::PVOverProduction,
        }],
    );

    // Setting ChargingPoint interval to 60s and PV moving window size to 5 minutes for the sake of
    // the test.
    let config_setting = config::ConfigSetting {
        key: "MeterValueSampleInterval".to_owned(),
        value: "60".to_owned(),
    };
    config
        .charging_point
        .config_parameters
        .push(config_setting.clone());
    config.photo_voltaic.moving_window_size_in_minutes = 5;

    let mut integration_test = common::IntegrationTest::new(config.clone());

    integration_test.setup();
    integration_test.validate_initial_messages_with_config_parameters(&vec![config_setting])?;

    // Simulating a load of 400W where 100W are used to charge the battery.
    // PV production is set to 14kW.
    integration_test.set_power_flow_realtime_data(-300.0, -100.0, 1000.0);
    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;
    integration_test.send_authorize_request(PV_SMART_CHARGING_ID)?;

    integration_test.validate_response_message(json!({"idTagInfo": { "status": "Accepted" }}))?;
    integration_test.validate_pv_preparation_profile(Utc::now())?;
    integration_test.send_start_transaction_request(PV_SMART_CHARGING_ID)?;
    integration_test.validate_response_message(
        json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1}),
    )?;

    integration_test.send_status_notification(CHARGING_STATUS_NOTIFCATION)?;

    assert!(integration_test.block_battery_for_duration_called());

    // Simulating a load of 11400W where 100W are used to charge the battery.
    // PV production is set to 14kW.
    integration_test.set_power_flow_realtime_data(-11300.0, -100.0, 14000.0);
    for _ in 0..5 {
        integration_test.send_meter_value_readings(
            (219.0, 219.0, 219.0),
            11.0,
            16.0,
            (3.7, 3.7, 3.7),
        )?;
        integration_test.validate_response_message(json!({}))?;
    }

    integration_test.validate_pv_based_profile(Decimal::new(16, 0))?;
    integration_test.send_stop_transaction_request()?;
    integration_test.validate_response_message(json!({"idTagInfo": {"status": "Accepted"}}))?;

    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;
    let request_uuid = integration_test.validate_request_message(
        &common::ExpectedJSONRequestFormat {
            message_id: 2,
            uuid: "".to_owned(),
            message_type: "ClearChargingProfile".to_owned(),
            json: json!({"connectorId": 1, "chargingProfilePurpose": "TxProfile", "id": 5, "stackLevel": 1}),
        },
    )?;

    integration_test.send_accepted_response(request_uuid.as_str())?;

    assert!(integration_test.unblock_battery_called());

    // Start another transaction. It is expected that NO power will be supplied because not enough
    // measured values have been gathered.

    integration_test.send_authorize_request(PV_SMART_CHARGING_ID)?;
    integration_test.validate_response_message(json!({"idTagInfo": { "status": "Accepted" }}))?;
    integration_test.validate_pv_preparation_profile(Utc::now())?;

    integration_test.send_start_transaction_request(PV_SMART_CHARGING_ID)?;
    integration_test.validate_response_message(
        json!({"idTagInfo": {"status": "Accepted"}, "transactionId": 1}),
    )?;

    // Simulating a load of 11400W where 100W are used to charge the battery.
    // PV production is set to 14kW.
    integration_test.set_power_flow_realtime_data(-11300.0, -100.0, 14000.0);
    integration_test.send_meter_value_readings(
        (219.0, 219.0, 219.0),
        11.0,
        16.0,
        (3.7, 3.7, 3.7),
    )?;
    integration_test.validate_response_message(json!({}))?;

    integration_test.send_stop_transaction_request()?;
    integration_test.validate_response_message(json!({"idTagInfo": {"status": "Accepted"}}))?;
    integration_test.send_status_notification(AVAILABLE_STATUS_NOTIFCATION)?;

    integration_test.teardown(config.log_directory.as_str());
    Ok(())
}
