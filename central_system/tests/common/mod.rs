use awattar::Period;
use awattar::awattar_mock::AwattarApiMock;
use chrono::{Duration, TimeDelta, Utc};
use config::config;
use fronius::{
    Data, FroniusMock, PowerFlowRealtimeData, PowerFlowRealtimeDataBody,
    PowerFlowRealtimeDataHeader, Site, Smartloads, Status,
};
use ocppcentral_system::setup_initial_configuration;
use serde::Deserialize;
use serde_json::json;
use std::error::Error;
use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};
use tungstenite::{WebSocket, connect, stream::MaybeTlsStream};

use ocpp::{Decimal, Power};
use rust_ocpp::v1_6::{
    messages::set_charging_profile::SetChargingProfileRequest,
    types::{
        ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
        ChargingSchedulePeriod,
    },
};

use rusqlite::Connection;

//-------------------------------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
pub struct ExpectedJSONRequestFormat {
    pub message_id: u32,
    pub uuid: String,
    pub message_type: String,
    pub json: serde_json::Value,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct ExpectedJSONResponseFormat {
    pub message_id: u32,
    pub uuid: String,
    pub json: serde_json::Value,
}

//-------------------------------------------------------------------------------------------------

pub struct IntegrationTest {
    pub config: config::Config,
    join_handles: Vec<JoinHandle<()>>,
    fronius_mock: Arc<Mutex<FroniusMock>>,
    awattar_mock: Arc<Mutex<AwattarApiMock>>,
    websocket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    connection: Arc<Mutex<Connection>>,
}

//-------------------------------------------------------------------------------------------------

impl IntegrationTest {
    pub fn new(config: config::Config) -> Self {
        let _ = env_logger::try_init();
        Self {
            config,
            join_handles: vec![],
            fronius_mock: Arc::new(Mutex::new(FroniusMock::default())),
            awattar_mock: Arc::new(Mutex::new(AwattarApiMock::default())),
            websocket: None,
            connection: Arc::new(Mutex::new(
                Connection::open_in_memory().expect("Could not create in-memory SQlite DB!"),
            )),
        }
    }

    pub fn setup(&mut self) {
        let config_clone = self.config.clone();

        self.fronius_mock.lock().unwrap().power_flow_realtime_data =
            Some(self.default_powerflow_realtime_data());

        let fronius_mock_handle = Arc::clone(&self.fronius_mock);
        let awattar_mock_handle = Arc::clone(&self.awattar_mock);
        let db_connection_handle = Arc::clone(&self.connection);

        self.join_handles.push(spawn(move || {
            let hooks = Arc::new(Mutex::new(ocppcentral_system::hooks::OcppHooks::new(
                fronius_mock_handle,
                awattar_mock_handle,
                config_clone.clone(),
                db_connection_handle,
            )));

            ocpp::run::<ocppcentral_system::hooks::OcppHooks<FroniusMock, AwattarApiMock>>(
                &config_clone,
                Arc::clone(&hooks),
                setup_initial_configuration(&config_clone)
                    .expect("Could not setup initial requests"),
            )
            .expect("Could not run OCPPCentralSystem");
        }));

        let websocket_address = format!(
            "ws://{}:{}",
            self.config.websocket.ip, self.config.websocket.port
        );

        // Websocket startup might take some time
        for i in 0..20 {
            match connect(websocket_address.to_owned()) {
                Ok((socket, _)) => {
                    self.websocket = Some(socket);
                    return;
                }
                _ => {}
            }

            std::thread::sleep(std::time::Duration::from_secs(i));
        }

        panic!("Could not connect!");
    }

    pub fn get_stored_meter_readings(
        &self,
    ) -> Result<Vec<(String, f64, String, String)>, Box<dyn std::error::Error>> {
        let handle = self.connection.lock().unwrap();
        let mut stmt = handle.prepare("SELECT * FROM meter_readings")?;
        let meter_readings_iter = stmt.query_map([], |row| {
            Ok((
                row.get::<usize, String>(1)?,
                row.get::<usize, f64>(3)?,
                row.get::<usize, String>(4)?,
                row.get::<usize, String>(5)?,
            ))
        })?;

        Ok(meter_readings_iter
            .map(|e| e.expect("Mismatched type"))
            .collect::<Vec<_>>())
    }

    pub fn get_stored_status_notifications(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let handle = self.connection.lock().unwrap();
        let mut stmt = handle.prepare("SELECT status FROM status_notifications")?;
        let status_notifiactions_iter =
            stmt.query_map([], |row| Ok(row.get::<usize, String>(0)?))?;

        Ok(status_notifiactions_iter
            .map(|e| e.expect("Mismatched type"))
            .collect::<Vec<_>>())
    }

    pub fn get_stored_authorize_requests(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let handle = self.connection.lock().unwrap();
        let mut stmt = handle.prepare("SELECT id_tag FROM authorize_requests")?;
        let authorize_requests_iter = stmt.query_map([], |row| Ok(row.get::<usize, String>(0)?))?;

        Ok(authorize_requests_iter
            .map(|e| e.expect("Mismatched type"))
            .collect::<Vec<_>>())
    }

    pub fn teardown(self, log_directory: &str) {
        self.websocket
            .unwrap()
            .write(tungstenite::Message::Close(None))
            .expect("Could not close connection!");

        for handle in self.join_handles {
            handle.join().expect("Could not join thread!");
        }

        std::fs::remove_dir_all(log_directory).expect("Cleanup failed");
    }

    pub fn validate_response_message(
        &mut self,
        json: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match serde_json::from_str::<ExpectedJSONResponseFormat>(
            self.websocket.as_mut().unwrap().read()?.to_text()?,
        ) {
            Ok(response) => {
                assert_eq!(
                    response,
                    ExpectedJSONResponseFormat {
                        message_id: 3,
                        uuid: "12345".to_owned(),
                        json,
                    }
                );
            }
            _ => assert!(false),
        }

        Ok(())
    }

    pub fn validate_initial_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            let uuid = self.validate_request_message(&expected_message)?;
            self.websocket
                .as_mut()
                .unwrap()
                .send(tungstenite::Message::text(format!(
                    "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                    uuid
                )))?;
        }

        Ok(())
    }

    pub fn validate_request_message(
        &mut self,
        expected_message: &ExpectedJSONRequestFormat,
    ) -> Result<String, Box<dyn std::error::Error>> {
        match serde_json::from_str::<ExpectedJSONRequestFormat>(
            self.websocket.as_mut().unwrap().read()?.to_text()?,
        ) {
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

    pub fn validate_initial_messages_with_config_parameters(
        &mut self,
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
            let uuid = self.validate_request_message(&expected_message)?;
            self.websocket
                .as_mut()
                .unwrap()
                .send(tungstenite::Message::text(format!(
                    "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                    uuid
                )))?;
        }

        Ok(())
    }

    pub fn validate_pv_preparation_profile(
        &mut self,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), Box<dyn Error>> {
        let set_charging_profile_message = self.websocket.as_mut().unwrap().read()?;
        match serde_json::from_str::<ExpectedJSONRequestFormat>(
            set_charging_profile_message.to_text()?,
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
                    4
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
                    vec![ChargingSchedulePeriod {
                        start_period: 0,
                        limit: Decimal::new(0, 0),
                        number_phases: None
                    },]
                );
                self.websocket
                    .as_mut()
                    .unwrap()
                    .send(tungstenite::Message::text(format!(
                        "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                        response.uuid
                    )))?;
            }
            _ => assert!(false),
        }

        Ok(())
    }

    pub fn validate_grid_based_profile(
        &mut self,
        now: chrono::DateTime<Utc>,
        start_timestamp: chrono::DateTime<Utc>,
        end_timestamp: chrono::DateTime<Utc>,
        limit: Decimal,
    ) -> Result<(), Box<dyn Error>> {
        match serde_json::from_str::<ExpectedJSONRequestFormat>(
            self.websocket.as_mut().unwrap().read()?.to_text()?,
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
                            limit,
                            number_phases: None
                        },
                    ]
                );
                self.websocket
                    .as_mut()
                    .unwrap()
                    .send(tungstenite::Message::text(format!(
                        "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                        response.uuid
                    )))?;
            }
            _ => assert!(false),
        }

        Ok(())
    }

    pub fn validate_pv_based_profile(&mut self, limit: Decimal) -> Result<(), Box<dyn Error>> {
        match serde_json::from_str::<ExpectedJSONRequestFormat>(
            self.websocket.as_mut().unwrap().read()?.to_text()?,
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
                        limit,
                        number_phases: None
                    }]
                );

                self.websocket
                    .as_mut()
                    .unwrap()
                    .send(tungstenite::Message::text(format!(
                        "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                        response.uuid
                    )))?;
            }
            _ => assert!(false),
        }

        Ok(())
    }

    pub fn validate_heartbeat_response(&mut self) -> Result<(), Box<dyn Error>> {
        // Heartbeat response contains the system time which can't be checked deterministically.
        // Only way would be to work with deltas but this could be instable as well.
        match serde_json::from_str::<ExpectedJSONResponseFormat>(
            self.websocket.as_mut().unwrap().read()?.to_text()?,
        ) {
            Ok(response) => {
                assert_eq!(response.message_id, 3);
                assert_eq!(response.uuid, "12345");
            }
            _ => assert!(false),
        }

        Ok(())
    }

    fn default_powerflow_realtime_data(&self) -> PowerFlowRealtimeData {
        PowerFlowRealtimeData {
            body: PowerFlowRealtimeDataBody {
                data: Data {
                    inverters: HashMap::default(),
                    site: Site {
                        mode: String::default(),
                        battery_standby: false,
                        backup_mode: false,
                        p_grid: None,
                        p_load: None,
                        p_akku: None,
                        p_pv: None,
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

    pub fn set_power_flow_realtime_data(&mut self, p_load: Power, p_akku: Power, p_pv: Power) {
        self.fronius_mock.lock().unwrap().power_flow_realtime_data = Some(PowerFlowRealtimeData {
            body: PowerFlowRealtimeDataBody {
                data: Data {
                    inverters: HashMap::default(),
                    site: Site {
                        mode: String::default(),
                        battery_standby: false,
                        backup_mode: false,
                        p_grid: None,
                        p_load: Some(p_load),
                        p_akku: Some(p_akku),
                        p_pv: Some(p_pv),
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
        });
    }

    pub fn block_battery_for_duration_called(&self) -> bool {
        self.fronius_mock
            .lock()
            .unwrap()
            .block_battery_for_duration_called
    }

    pub fn unblock_battery_called(&self) -> bool {
        self.fronius_mock.lock().unwrap().unblock_battery_called
    }

    pub fn set_awattar_response(&self, period: Period) {
        self.awattar_mock.lock().unwrap().set_response(period);
    }

    pub fn send_meter_value_readings(
        &mut self,
        voltage_per_phase: (f64, f64, f64),
        power_offered: f64,
        current_offered: f64,
        power_active_imported: (f64, f64, f64),
    ) -> Result<(), Box<dyn Error>> {
        Ok(self
            .websocket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::text(format!(
                "[2,\"12345\",\"MeterValues\",{}]",
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
                            },
                            {
                                "value": power_active_imported.0.to_string(),
                                "context": "Sample.Periodic",
                                "format": "Raw",
                                "measurand": "Power.Active.Import",
                                "location": "Outlet",
                                "phase": "L1",
                                "unit": "kW"
                            },
                            {
                                "value": power_active_imported.1.to_string(),
                                "context": "Sample.Periodic",
                                "format": "Raw",
                                "measurand": "Power.Active.Import",
                                "location": "Outlet",
                                "phase": "L2",
                                "unit": "kW"
                            },
                            {
                                "value": power_active_imported.2.to_string(),
                                "context": "Sample.Periodic",
                                "format": "Raw",
                                "measurand": "Power.Active.Import",
                                "location": "Outlet",
                                "phase": "L3",
                                "unit": "kW"
                            }
                        ]
                    } ]
                })
            )))?)
    }

    pub fn send_accepted_response(&mut self, uuid: &str) -> Result<(), Box<dyn Error>> {
        Ok(self
            .websocket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::text(format!(
                "[3,\"{}\",{{\"status\": \"Accepted\"}}]",
                uuid
            )))?)
    }

    pub fn send_authorize_request(&mut self, id_tag: &str) -> Result<(), Box<dyn Error>> {
        Ok(self
            .websocket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::text(format!(
                "[2,\"12345\",\"Authorize\",{{ \"idTag\": \"{}\"}}]",
                id_tag
            )))?)
    }

    pub fn send_status_notification(
        &mut self,
        status: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.websocket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::text(format!(
                "[2,\"12345\",\"StatusNotification\",{{\"connectorId\": 1,
                    \"errorCode\": \"NoError\",
                    \"info\": \"\",
                    \"status\": \"{}\",
                    \"timestamp\": \"2026-01-18T14:09:24Z\",
                    \"vendorId\": \"Schneider Electric\",
                    \"vendorErrorCode\": \"0.0\"}}]",
                status
            )))?;

        match serde_json::from_str::<ExpectedJSONResponseFormat>(
            self.websocket.as_mut().unwrap().read()?.to_text()?,
        ) {
            Ok(response) => {
                assert_eq!(response.message_id, 3);
                assert_eq!(response.uuid, "12345");
                assert_eq!(response.json, json!({}));
            }
            _ => assert!(false),
        }

        Ok(())
    }

    pub fn send_start_transaction_request(&mut self, id_tag: &str) -> Result<(), Box<dyn Error>> {
        let start_transaction_request = format!(
            "{{
                \"connectorId\": 1,
                \"idTag\": \"{}\",
                \"meterStart\": 0,
                \"timestamp\": \"2026-01-18T14:09:24Z\"
            }}",
            id_tag
        );
        Ok(self
            .websocket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::text(format!(
                "[2,\"12345\",\"StartTransaction\",{}]",
                start_transaction_request
            )))?)
    }

    pub fn send_stop_transaction_request(&mut self) -> Result<(), Box<dyn Error>> {
        static STOP_TRANSACTION_REQUEST: &str = r#"{
            "meterStop": 253580,
            "reason": "EVDisconnected",
            "timestamp": "2026-02-04T05:39:05Z",
            "transactionId": 1
        }"#;

        Ok(self
            .websocket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::text(format!(
                "[2,\"12345\",\"StopTransaction\",{}]",
                STOP_TRANSACTION_REQUEST
            )))?)
    }

    pub fn send_heartbeat_request(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(self
            .websocket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::text("[2,\"12345\",\"Heartbeat\",{}]"))?)
    }
}
