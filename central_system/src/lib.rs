pub mod hooks;

use awattar::AwattarApiAdapter;
use config::config::Config;
use fronius::FroniusApiAdapter;
use hooks::OcppHooks;
use log::info;
use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;

use ocpp::{
    ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType, CustomError,
    Decimal, MessageBuilder, MessageTrigger, MessageTypeName, RequestToSend, ampere,
    change_configuration_builder::ChangeConfigurationBuilder,
    charging_profile_builder::ChargingProfileBuilder,
    clear_charging_profile_builder::ClearChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
    trigger_message_builder::TriggerMessageBuilder,
};

//-------------------------------------------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const GIT_COMMIT_HASH: &'static str = env!("GIT_COMMIT_HASH");

//-------------------------------------------------------------------------------------------------

pub fn setup_initial_configuration(config: &Config) -> Result<Vec<RequestToSend>, CustomError> {
    let (uuid, payload) = TriggerMessageBuilder::new(MessageTrigger::MeterValues, None)
        .build()
        .serialize()?;

    let mut requests_to_send = vec![];

    requests_to_send.push(RequestToSend {
        message_type: MessageTypeName::TriggerMessage,
        uuid,
        payload,
    });

    for config_parameter in &config.charging_point.config_parameters {
        let (uuid, change_config_parameter_request) = ChangeConfigurationBuilder::new(
            config_parameter.key.clone(),
            config_parameter.value.clone(),
        )
        .build()
        .serialize()?;

        requests_to_send.push(RequestToSend {
            uuid: uuid.clone(),
            message_type: MessageTypeName::ChangeConfiguration,
            payload: change_config_parameter_request,
        });
    }

    static CHARGE_POINT_MAX_PROFILE: i32 = 3;
    static CONNECTOR_ID: i32 = 0;

    let charging_profile = ChargingProfileBuilder::new(
        CHARGE_POINT_MAX_PROFILE,
        ChargingProfilePurposeType::ChargePointMaxProfile,
        ChargingProfileKindType::Absolute,
        ChargingRateUnitType::A,
    )
    .add_charging_schedule_period(
        0,
        Decimal::from_f64_retain(config.charging_point.default_current.get::<ampere>())
            .ok_or(CustomError::Common(
                "Could not convert to Decimal!".to_owned(),
            ))?
            .round_dp(1),
        None,
    )
    .get();

    let (uuid, set_charging_profile_request) =
        SetChargingProfileBuilder::new(CONNECTOR_ID, charging_profile)
            .build()
            .serialize()?;

    requests_to_send.push(RequestToSend {
        uuid: uuid.clone(),
        message_type: MessageTypeName::SetChargingProfile,
        payload: set_charging_profile_request,
    });

    let (uuid, clear_charging_profile_request) =
        ClearChargingProfileBuilder::default().build().serialize()?;

    requests_to_send.push(RequestToSend {
        uuid: uuid.clone(),
        message_type: MessageTypeName::ClearChargingProfile,
        payload: clear_charging_profile_request,
    });

    let (uuid, status_notification_request) =
        TriggerMessageBuilder::new(MessageTrigger::StatusNotification, None)
            .build()
            .serialize()?;

    requests_to_send.push(RequestToSend {
        message_type: MessageTypeName::TriggerMessage,
        uuid,
        payload: status_notification_request,
    });

    Ok(requests_to_send)
}

//-------------------------------------------------------------------------------------------------

/// Main entry point. Basically only a wrapper to enable integration tests
pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    info!("Starting OCPPCentralSystem v{} - {}", VERSION, GIT_COMMIT_HASH);
    let hooks = Arc::new(Mutex::new(OcppHooks::new(
        Arc::new(Mutex::new(FroniusApiAdapter::new(&config.fronius)?)),
        Arc::new(Mutex::new(AwattarApiAdapter::default())),
        config.clone(),
    )));

    ocpp::run::<OcppHooks<FroniusApiAdapter, AwattarApiAdapter>>(
        &config,
        Arc::clone(&hooks),
        setup_initial_configuration(&config)?,
    )
}
