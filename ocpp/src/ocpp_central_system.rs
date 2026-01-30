use rust_ocpp::v1_6::{
    messages::{
        authorize, boot_notification, change_configuration, clear_charging_profile, data_transfer,
        diagnostics_status_notification, firmware_status_notification, get_diagnostics, heart_beat,
        meter_values, remote_start_transaction, remote_stop_transaction, set_charging_profile,
        start_transaction, status_notification, stop_transaction, trigger_message,
    },
    types::MessageTrigger,
};

use rust_ocpp::v2_0_1::messages::{log_status_notification, security_event_notification};

use crate::{
    ChargePointState, OcppAuthorizationHook, OcppMeterValuesHook, OcppStatusNotificationHook, RequestToSend, builders::{
        MessageBuilder, change_configuration_builder::ChangeConfigurationBuilder,
        clear_charging_profile_builder::ClearChargingProfileBuilder,
        trigger_message_builder::TriggerMessageBuilder,
    }, handlers::{
        authorize_handler::handle_authorize_request,
        boot_notification_handler::handle_boot_notification_request,
        change_configuration_handler::handle_change_configuration_response,
        clear_charging_profile_handler::handle_clear_charging_profile_response,
        data_transfer_handler::handle_data_transfer_request,
        diagnostics_status_notification_handler::handle_diagnostic_status_notification_request,
        firmware_status_notification_handler::handle_firmware_status_notification_request,
        get_diagnostics_handler::handle_get_diagnostics_response,
        heartbeat_handler::handle_heartbeat_request,
        log_status_notification_handler::handle_log_status_notification_request,
        meter_value_handler::handle_meter_values_request,
        remote_start_transaction_handler::handle_remote_start_transaction_response,
        remote_stop_transaction_handler::handle_remote_stop_transaction_response,
        security_event_notification_handler::handle_security_event_notification_request,
        set_charging_profile_handler::handle_set_charging_profile_response,
        signed_firmware_status_notification::handle_signed_firmware_status_notification_request,
        start_transaction_handler::handle_start_transaction_request,
        status_notification_handler::handle_status_notification_request,
        stop_transaction_handler::handle_stop_transaction_request,
        trigger_message_handler::handle_trigger_message_response,
    }, ocpp_types::*, visitor::Visitor
};

use config::config::Config;
use log::info;

use rusqlite::Connection;
use tungstenite::Utf8Bytes;

use std::sync::{Arc, Mutex};

//-------------------------------------------------------------------------------------------------

pub struct OCPPCentralSystem<T: OcppStatusNotificationHook + OcppMeterValuesHook + OcppAuthorizationHook> {
    db_connection: Connection,
    charging_point_ip: String,
    config: Config,
    charge_point_state: Arc<Mutex<ChargePointState>>,

    charging_point_count: u32,

    ocpp_hooks: Arc<Mutex<T>>,
}

impl<T: OcppStatusNotificationHook + OcppMeterValuesHook + OcppAuthorizationHook> OCPPCentralSystem<T> {
    pub fn new(
        db_connection: Connection,
        charging_point_ip: String,
        config: Config,
        charge_point_state: Arc<Mutex<ChargePointState>>,
        status_notification_hook: Arc<Mutex<T>>,
    ) -> Self {
        let mut instance = Self {
            db_connection,
            charging_point_ip,
            config,
            charge_point_state,
            charging_point_count: 0,
            ocpp_hooks: status_notification_hook,
        };

        instance
            .setup_persistence()
            .expect("Could not create persistence DB!");

        instance
            .setup_initial_configuration()
            .expect("Could not setup initial configuration requests!");

        instance
    }

    fn setup_persistence(&self) -> Result<(), CustomError> {
        self.db_connection.execute(
            "CREATE TABLE IF NOT EXISTS charging_points (id INT PRIMARY KEY, charging_point_ip TEXT, charge_box_serial_number TEXT, charge_point_model TEXT, charge_point_serial_number TEXT, charge_point_vendor TEXT, firmware_version TEXT, iccid TEXT, imsi TEXT, meter_serial_number TEXT, meter_type TEXT, UNIQUE(charging_point_ip, charge_point_serial_number));",
            ()
        )?;

        self.db_connection.execute(
            "CREATE TABLE IF NOT EXISTS security_event_notifications (id INT PRIMARY KEY, kind TEXT, timestamp TEXT, tech_info TEXT);",
            ()
        )?;

        self.db_connection.execute(
            "CREATE TABLE IF NOT EXISTS log_status (id INT PRIMARY KEY, status TEXT, request_id REAL);",
            ()
        )?;

        self.db_connection.execute(
            "CREATE TABLE IF NOT EXISTS signed_firmware_status_notification (id INT PRIMARY KEY, status TEXT);",
            ()
        )?;

        Ok(())
    }

    pub fn get_boot_message_request(&mut self) -> Result<RequestToSend, CustomError> {
        let (uuid, payload) = TriggerMessageBuilder::new(MessageTrigger::BootNotification, None)
            .build()
            .serialize()?;

        let request_to_send = RequestToSend {
            message_type: MessageTypeName::TriggerMessage,
            uuid,
            payload,
        };

        self.charge_point_state
            .lock()
            .unwrap()
            .requests_awaiting_confirmation
            .push(request_to_send.clone());

        Ok(request_to_send)
    }

    fn setup_initial_configuration(&mut self) -> Result<(), CustomError> {
        let (uuid, payload) = TriggerMessageBuilder::new(MessageTrigger::MeterValues, None)
            .build()
            .serialize()?;

        let mut charge_point_state = self.charge_point_state.lock().unwrap();

        charge_point_state.requests_to_send.push(RequestToSend {
            message_type: MessageTypeName::TriggerMessage,
            uuid,
            payload,
        });

        for config_parameter in &self.config.charging_point.config_parameters {
            let (uuid, change_config_parameter_request) = ChangeConfigurationBuilder::new(
                config_parameter.key.clone(),
                config_parameter.value.clone(),
            )
            .build()
            .serialize()?;

            charge_point_state.requests_to_send.push(RequestToSend {
                uuid: uuid.clone(),
                message_type: MessageTypeName::ChangeConfiguration,
                payload: change_config_parameter_request,
            });
        }

        let (uuid, clear_charging_profile_request) =
            ClearChargingProfileBuilder::default().build().serialize()?;

        charge_point_state.requests_to_send.push(RequestToSend {
            uuid: uuid.clone(),
            message_type: MessageTypeName::ClearChargingProfile,
            payload: clear_charging_profile_request,
        });

        Ok(())
    }

    pub fn process_text_message(&mut self, text: &Utf8Bytes) -> Result<String, CustomError> {
        match parse_ocpp_message(text.as_str())? {
            OcppMessage::Request(request) => {
                info!(
                    "Received request message of type {} with uuid {}",
                    request.message_type, request.uuid
                );

                self.charging_point_count = self.db_connection.query_row(
                    format!(
                        "SELECT COUNT(*) FROM charging_points WHERE charging_point_ip = '{}';",
                        self.charging_point_ip
                    )
                    .as_str(),
                    [],
                    |r| r.get(0),
                )?;

                let response = self.visit_request_message(request);

                response
            }
            OcppMessage::Response(response) => self.visit_response_message(response),
            OcppMessage::Error(error) => Err(CustomError::Common(format!(
                "Received error from ChargePoint: {} {} {}",
                error.error_code, error.error_description, error.json
            ))),
        }
    }

    pub fn get_pending_message(&mut self) -> Option<RequestToSend> {
        if self.waiting_for_response() {
            return None;
        }

        let mut charge_point_state = self.charge_point_state.lock().unwrap();
        match charge_point_state.requests_to_send.pop() {
            Some(request_to_send) => {
                charge_point_state
                    .requests_awaiting_confirmation
                    .push(request_to_send.clone());
                Some(request_to_send)
            }
            None => None,
        }
    }

    fn waiting_for_response(&self) -> bool {
        !self
            .charge_point_state
            .lock()
            .unwrap()
            .requests_awaiting_confirmation
            .is_empty()
    }
}

//-------------------------------------------------------------------------------------------------

impl<T: OcppStatusNotificationHook + OcppMeterValuesHook + OcppAuthorizationHook> Visitor<Result<String, CustomError>>
    for OCPPCentralSystem<T>
{
    fn visit_request_message(
        &mut self,
        request: OcppRequestMessage,
    ) -> Result<String, CustomError> {
        let charge_point_handle = self.charge_point_state.clone();
        let mut charge_point_state = charge_point_handle.lock().unwrap();

        let response = match request.message_type {
            MessageTypeName::Authorize => {
                let authorize_request =
                    serde_json::from_value::<authorize::AuthorizeRequest>(request.json)?;

                let mut merged_id_tags: Vec<config::config::IdTag> = self.config.id_tags.clone();
                merged_id_tags.extend(
                    charge_point_state
                        .get_remote_start_transaction_id_tags()
                        .iter()
                        .map(|e| config::config::IdTag { id: e.clone() })
                        .collect::<Vec<_>>(),
                );

                serde_json::to_value(&handle_authorize_request(
                    &authorize_request,
                    &merged_id_tags,
                    &mut charge_point_state,
                    Arc::clone(&self.ocpp_hooks),
                )?)?
            }
            MessageTypeName::BootNotification => {
                let boot_notification_request = serde_json::from_value::<
                    boot_notification::BootNotificationRequest,
                >(request.json)?;

                let _ = self.db_connection.execute(
                    "INSERT OR IGNORE INTO charging_points (charging_point_ip, charge_box_serial_number, charge_point_model, charge_point_serial_number, charge_point_vendor, firmware_version, iccid, imsi, meter_serial_number, meter_type) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
                    (&self.charging_point_ip,
                     &boot_notification_request.charge_point_serial_number.clone().unwrap_or("".to_string()),
                     &boot_notification_request.charge_point_model,
                     &boot_notification_request.charge_point_serial_number.clone().unwrap_or("".to_string()),
                     &boot_notification_request.charge_point_vendor,
                     &boot_notification_request.firmware_version.clone().unwrap_or("".to_string()),
                     &boot_notification_request.iccid.clone().unwrap_or("".to_string()),
                     &boot_notification_request.imsi.clone().unwrap_or("".to_string()),
                     &boot_notification_request.meter_serial_number.clone().unwrap_or("".to_string()),
                     &boot_notification_request.meter_type.clone().unwrap_or("".to_string()))
                );

                serde_json::to_value(&handle_boot_notification_request(
                    &boot_notification_request,
                    &self.config.charging_point,
                )?)?
            }
            MessageTypeName::DataTransfer => {
                let data_transfer_request =
                    serde_json::from_value::<data_transfer::DataTransferRequest>(request.json)?;

                serde_json::to_value(&handle_data_transfer_request(&data_transfer_request)?)?
            }
            MessageTypeName::DiagnosticsStatusNotification => {
                let diagnostic_status_notification_request = serde_json::from_value::<
                    diagnostics_status_notification::DiagnosticsStatusNotificationRequest,
                >(request.json)?;

                serde_json::to_value(&handle_diagnostic_status_notification_request(
                    &diagnostic_status_notification_request,
                )?)?
            }
            MessageTypeName::FirmwareStatusNotification => {
                let firmware_status_notification_request = serde_json::from_value::<
                    firmware_status_notification::FirmwareStatusNotificationRequest,
                >(request.json)?;

                serde_json::to_value(&handle_firmware_status_notification_request(
                    &firmware_status_notification_request,
                )?)?
            }
            MessageTypeName::Heartbeat => {
                let _ = serde_json::from_value::<heart_beat::HeartbeatRequest>(request.json)?;

                serde_json::to_value(&handle_heartbeat_request()?)?
            }
            MessageTypeName::MeterValues => {
                let meter_values_request =
                    serde_json::from_value::<meter_values::MeterValuesRequest>(request.json)?;

                serde_json::to_value(&handle_meter_values_request(
                    &meter_values_request,
                    &mut charge_point_state,
                    Arc::clone(&self.ocpp_hooks),
                )?)?
            }
            MessageTypeName::StartTransaction => {
                let start_transaction_request = serde_json::from_value::<
                    start_transaction::StartTransactionRequest,
                >(request.json)?;

                serde_json::to_value(&handle_start_transaction_request(
                    &start_transaction_request,
                    &self.config.id_tags,
                    &mut charge_point_state,
                )?)?
            }
            MessageTypeName::StatusNotification => {
                let status_notification = serde_json::from_value::<
                    status_notification::StatusNotificationRequest,
                >(request.json)?;
                serde_json::to_value(&handle_status_notification_request(
                    &status_notification,
                    &mut charge_point_state,
                    Arc::clone(&self.ocpp_hooks),
                )?)?
            }
            MessageTypeName::StopTransaction => {
                let stop_transaction_request = serde_json::from_value::<
                    stop_transaction::StopTransactionRequest,
                >(request.json)?;
                serde_json::to_value(&handle_stop_transaction_request(
                    &stop_transaction_request,
                    &mut charge_point_state,
                )?)?
            }
            MessageTypeName::LogStatusNotification => {
                let log_status_notification_request = serde_json::from_value::<
                    log_status_notification::LogStatusNotificationRequest,
                >(request.json)?;

                let _ = self.db_connection.execute(
                    "INSERT INTO log_status (status, request_id) VALUES(?1, ?2);",
                    (
                        serde_json::to_string(&log_status_notification_request.status)
                            .unwrap_or("".to_string()),
                        &log_status_notification_request
                            .request_id
                            .clone()
                            .unwrap_or(-1),
                    ),
                );

                serde_json::to_value(&handle_log_status_notification_request(
                    &log_status_notification_request,
                )?)?
            }
            MessageTypeName::SecurityEventNotification => {
                let security_event_notification_request = serde_json::from_value::<
                    security_event_notification::SecurityEventNotificationRequest,
                >(request.json)?;

                let _ = self.db_connection.execute(
                    "INSERT INTO security_event_notifications (kind, timestamp, tech_info) VALUES(?1, ?2, ?3);",
                    (
                        &security_event_notification_request.kind,
                        &security_event_notification_request.timestamp.to_string(),
                        &security_event_notification_request
                            .tech_info
                            .clone()
                            .unwrap_or("".to_string()),
                    ),
                );

                serde_json::to_value(&handle_security_event_notification_request(
                    &security_event_notification_request,
                )?)?
            }
            MessageTypeName::SignedFirmwareStatusNotification => {
                let signed_firmware_status_notification_request = serde_json::from_value::<
                    firmware_status_notification::FirmwareStatusNotificationRequest,
                >(request.json)?;

                let _ = self.db_connection.execute(
                    "INSERT INTO signed_firmware_status_notification (status) VALUES(?1);",
                    (
                        serde_json::to_string(&signed_firmware_status_notification_request.status)
                            .unwrap_or("".to_string()),
                    ),
                );

                serde_json::to_value(&handle_signed_firmware_status_notification_request(
                    &signed_firmware_status_notification_request,
                )?)?
            }
            _ => panic!("Unknown message of type {} received", request.message_type),
        };

        serialze_ocpp_response(&request.uuid, &response)
    }

    fn visit_response_message(
        &mut self,
        response: OcppResponseMessage,
    ) -> Result<String, CustomError> {
        let handle = self.charge_point_state.clone();
        let mut charge_point_state = handle.lock().unwrap();

        let element = charge_point_state
            .requests_awaiting_confirmation
            .iter()
            .find(|e| e.uuid == response.uuid)
            .ok_or(CustomError::Common(format!(
                "Could not find request with uuid {}",
                response.uuid
            )))?
            .clone();

        info!(
            "Received response message of type {} with uuid {}",
            element.message_type, response.uuid
        );

        match element.message_type {
            MessageTypeName::TriggerMessage => {
                let trigger_message_response = serde_json::from_value::<
                    trigger_message::TriggerMessageResponse,
                >(response.json)?;
                handle_trigger_message_response(
                    &response.uuid,
                    &trigger_message_response,
                    &mut charge_point_state,
                );
            }
            MessageTypeName::SetChargingProfile => {
                let set_charging_profile_response = serde_json::from_value::<
                    set_charging_profile::SetChargingProfileResponse,
                >(response.json)?;
                handle_set_charging_profile_response(
                    &response.uuid,
                    &set_charging_profile_response,
                    &mut charge_point_state,
                );
            }
            MessageTypeName::ClearChargingProfile => {
                let clear_charging_profile_response = serde_json::from_value::<
                    clear_charging_profile::ClearChargingProfileResponse,
                >(response.json)?;
                handle_clear_charging_profile_response(
                    &response.uuid,
                    &clear_charging_profile_response,
                    &mut charge_point_state,
                );
            }
            MessageTypeName::ChangeConfiguration => {
                let change_configuration_response = serde_json::from_value::<
                    change_configuration::ChangeConfigurationResponse,
                >(response.json)?;

                handle_change_configuration_response(
                    &response.uuid,
                    &change_configuration_response,
                    &mut charge_point_state,
                );
            }
            MessageTypeName::RemoteStartTransaction => {
                let remote_start_transaction_response = serde_json::from_value::<
                    remote_start_transaction::RemoteStartTransactionResponse,
                >(response.json)?;

                handle_remote_start_transaction_response(
                    &response.uuid,
                    &remote_start_transaction_response,
                    &mut charge_point_state,
                );
            }
            MessageTypeName::RemoteStopTransaction => {
                let remote_stop_transaction_response = serde_json::from_value::<
                    remote_stop_transaction::RemoteStopTransactionResponse,
                >(response.json)?;

                handle_remote_stop_transaction_response(
                    &response.uuid,
                    &remote_stop_transaction_response,
                    &mut charge_point_state,
                );
            }
            MessageTypeName::GetDiagnostics => {
                let get_diagnostics_response = serde_json::from_value::<
                    get_diagnostics::GetDiagnosticsResponse,
                >(response.json)?;

                handle_get_diagnostics_response(
                    &response.uuid,
                    &get_diagnostics_response,
                    &mut charge_point_state,
                );
            }
            _ => {}
        }

        drop(charge_point_state);

        match self.get_pending_message() {
            Some(pending_message) => Ok(pending_message.payload),
            _ => Ok("".to_string()),
        }
    }
}
