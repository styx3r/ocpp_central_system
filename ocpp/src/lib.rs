mod builders;
mod handlers;
mod ocpp_central_system;
mod ocpp_types;
mod visitor;

use log::{debug, error, info};
use rusqlite::{Connection, Result};
use std::process::exit;
use std::{error::Error, net::TcpListener};
use tungstenite::{Utf8Bytes, accept};

use config::config::Config;
pub use ocpp_types::MessageTypeName;

use ocpp_central_system::OCPPCentralSystem;

use std::sync::{Arc, Mutex};

//-------------------------------------------------------------------------------------------------

pub use builders::{
    MessageBuilder, charging_profile_builder, clear_charging_profile_builder,
    remote_start_transaction_builder, remote_stop_transaction_builder,
    set_charging_profile_builder,
};
pub use rust_ocpp::v1_6::messages::{
    authorize::AuthorizeRequest, status_notification::StatusNotificationRequest,
};
pub use rust_ocpp::v1_6::types::ChargePointStatus;

pub trait OcppStatusNotificationHook {
    fn evaluate(
        &mut self,
        status_notification: &StatusNotificationRequest,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

//-------------------------------------------------------------------------------------------------

pub use crate::ocpp_types::CustomError;
pub use rust_decimal::Decimal;
pub use rust_ocpp::v1_6::types::{
    ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType, RecurrencyKindType,
};

pub trait OcppMeterValuesHook {
    fn evaluate(
        &mut self,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

//-------------------------------------------------------------------------------------------------

pub trait OcppAuthorizationHook {
    fn evaluate(
        &mut self,
        authorization_request: &AuthorizeRequest,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub id_tag: Option<String>,
    pub transaction_id: i32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RequestToSend {
    pub uuid: String,
    pub message_type: MessageTypeName,
    pub payload: String,
}

#[derive(Default, Clone)]
pub struct ChargePointState {
    latest_cos_phi: Option<f64>,
    latest_power: Option<f64>,
    latest_current: Option<f64>,
    latest_voltage: Option<f64>,
    max_current: Option<f64>,

    requests_to_send: Vec<RequestToSend>,
    requests_awaiting_confirmation: Vec<RequestToSend>,
    running_transactions: Vec<Transaction>,

    remote_start_transaction_id_tags: Vec<String>,
}

impl ChargePointState {
    pub fn new(cos_phi: f64, power: f64, current: f64, voltage: f64) -> Self {
        Self {
            latest_cos_phi: Some(cos_phi),
            latest_power: Some(power),
            latest_current: Some(current),
            latest_voltage: Some(voltage),
            max_current: None,
            requests_to_send: vec![],
            requests_awaiting_confirmation: vec![],
            running_transactions: vec![],
            remote_start_transaction_id_tags: vec![],
        }
    }

    pub fn get_latest_cos_phi(&self) -> Option<f64> {
        self.latest_cos_phi
    }

    pub fn get_latest_power(&self) -> Option<f64> {
        self.latest_power
    }

    pub fn get_latest_current(&self) -> Option<f64> {
        self.latest_current
    }

    pub fn get_latest_voltage(&self) -> Option<f64> {
        self.latest_voltage
    }

    pub fn get_max_current(&self) -> Option<f64> {
        self.max_current
    }

    pub fn get_requests_to_send(&self) -> &Vec<RequestToSend> {
        &self.requests_to_send
    }

    pub fn get_remote_start_transaction_id_tags(&self) -> &Vec<String> {
        &self.remote_start_transaction_id_tags
    }

    pub fn get_running_transaction_ids(&self) -> &Vec<Transaction> {
        &self.running_transactions
    }

    pub fn set_latest_cos_phi(&mut self, cos_phi: f64) {
        self.latest_cos_phi = Some(cos_phi);
    }

    pub fn set_max_current(&mut self, max_current: f64) {
        self.max_current = Some(max_current);
    }

    pub fn add_request_to_send(&mut self, request_to_send: RequestToSend) {
        self.requests_to_send.push(request_to_send);
    }

    pub fn add_remote_transaction_id_tag(&mut self, id_tag: String) {
        self.remote_start_transaction_id_tags.push(id_tag);
    }

    pub fn clear_remote_start_transaction_id_tags(&mut self) {
        self.remote_start_transaction_id_tags.clear();
    }
}

//-------------------------------------------------------------------------------------------------

fn dispatch_message<T>(
    ocpp_central_system: &mut OCPPCentralSystem<T>,
    text: &Utf8Bytes,
) -> Vec<String>
where
    T: OcppStatusNotificationHook + OcppMeterValuesHook + OcppAuthorizationHook,
{
    let mut response_messages: Vec<String> = vec![];
    match ocpp_central_system.process_text_message(&text) {
        Ok(result) => {
            if !result.is_empty() {
                response_messages.push(result);
            }
        }
        Err(e) => {
            error!("Failed to process message: {}", e);
        }
    }

    match ocpp_central_system.get_pending_message() {
        Some(pending_message) => {
            response_messages.push(pending_message.payload);
        }
        _ => {}
    }

    response_messages
}

//-------------------------------------------------------------------------------------------------

pub fn run<T: OcppStatusNotificationHook + OcppMeterValuesHook + OcppAuthorizationHook>(
    config: &Config,
    ocpp_hooks: Arc<Mutex<T>>,
) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(&config.log_directory)?;

    let db_connection = match Connection::open(format!("{}/ocpp.sqlite", &config.log_directory)) {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Could not open DB connection with reason: \"{}\"",
                e.to_string()
            );
            exit(1);
        }
    };

    let listen_address = format!("{}:{}", config.websocket.ip, config.websocket.port);
    let server = match TcpListener::bind(&listen_address) {
        Ok(l) => l,
        Err(e) => {
            error!(
                "Could not bind TCP listener with reason: \"{}\"",
                e.to_string()
            );
            exit(1);
        }
    };

    info!(
        "OCPP server listening on {} with HeartBeatInterval {}, MaxChargingPower {} and AllowedIdTags {:?}",
        listen_address,
        config.charging_point.heartbeat_interval,
        config.charging_point.max_charging_power,
        config.id_tags
    );

    for stream in server.incoming() {
        let handle = stream?;
        let peer_address = handle.peer_addr()?.ip().to_string();

        let charge_point_state = Arc::new(Mutex::new(ChargePointState::default()));
        let mut ocpp_central_system = OCPPCentralSystem::new(
            db_connection,
            peer_address,
            config.to_owned(),
            charge_point_state,
            ocpp_hooks,
        );

        let mut websocket = accept(handle)?;

        // Request BootNotification on connection
        let boot_notification_request = ocpp_central_system.get_boot_message_request()?;
        info!(
            "Sending pending message {}",
            boot_notification_request.payload
        );

        websocket.send(tungstenite::Message::text(
            boot_notification_request.payload,
        ))?;

        loop {
            let msg = websocket.read().unwrap();
            match msg {
                tungstenite::Message::Text(text) => {
                    dispatch_message(&mut ocpp_central_system, &text)
                        .iter()
                        .try_for_each(|message| -> Result<(), Box<dyn Error>> {
                            info!("Sending {}", message);
                            Ok(websocket.send(tungstenite::Message::text(message))?)
                        })?;
                }
                tungstenite::Message::Ping(ping) => {
                    websocket.send(tungstenite::Message::Pong(ping))?;
                }
                tungstenite::Message::Pong(_) => {
                    panic!("Pong message should not be received!");
                }
                tungstenite::Message::Binary(binary) => {
                    debug!("Got binary message {:#01x}", binary);
                }
                tungstenite::Message::Close(_) => {
                    debug!("Got close message");
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    Ok(())
}
