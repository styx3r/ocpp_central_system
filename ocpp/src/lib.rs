mod builders;
mod charge_point_state;
mod handlers;
mod ocpp_central_system;
mod ocpp_types;
mod visitor;

use log::{debug, error, info, trace};
use rusqlite::{Connection, Result};
use std::process::exit;
use std::{error::Error, net::TcpListener};
use tungstenite::{Utf8Bytes, accept};

pub use crate::charge_point_state::*;
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
        trace!(
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
                            trace!("Sending {}", message);
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
