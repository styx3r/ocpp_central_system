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

pub use rust_ocpp::v1_6::messages::status_notification::StatusNotificationRequest;
pub use rust_ocpp::v1_6::types::ChargePointStatus;

pub trait OcppStatusNotificationHook {
    fn evaluate(
        &mut self,
        status_notification: &StatusNotificationRequest,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

//-------------------------------------------------------------------------------------------------

pub use crate::ocpp_types::CustomError;
pub use builders::MessageBuilder;
pub use builders::set_charging_profile_builder::SetChargingProfileBuilder;
pub use rust_decimal::Decimal;
pub use rust_ocpp::v1_6::types::{
    ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
};

pub trait OcppMeterValuesHook {
    fn evaluate(
        &mut self,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
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

#[derive(Default)]
pub struct ChargePointState {
    pub latest_cos_phi: Option<f64>,
    pub latest_power: Option<f64>,
    pub latest_current: Option<f64>,
    pub latest_voltage: Option<f64>,
    pub max_current: Option<f64>,

    pub requests_to_send: Vec<RequestToSend>,
    pub requests_awaiting_confirmation: Vec<RequestToSend>,
    pub running_transactions: Vec<Transaction>,
}

//-------------------------------------------------------------------------------------------------

fn dispatch_message<T>(
    ocpp_central_system: &mut OCPPCentralSystem<T>,
    text: &Utf8Bytes,
) -> Vec<String>
where
    T: OcppStatusNotificationHook + OcppMeterValuesHook,
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

pub fn run<T: OcppStatusNotificationHook + OcppMeterValuesHook>(
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
