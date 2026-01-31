use crate::{OcppHooks, hooks::calculate_max_current};
use log::info;

use config::config;
use ocpp::{
    ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    CustomError, Decimal, MessageBuilder, MessageTypeName, RequestToSend,
    charging_profile_builder::ChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
};

//-------------------------------------------------------------------------------------------------

impl ocpp::OcppMeterValuesHook for OcppHooks {
    fn evaluate(
        &mut self,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(_latest_current) = charge_point_state.get_latest_current()
            && let Some(_latest_power) = charge_point_state.get_latest_power()
            && let Some(_latest_voltage) = charge_point_state.get_latest_voltage()
            && let Some(_latest_cos_phi) = charge_point_state.get_latest_cos_phi()
            && charge_point_state.get_remote_start_transaction_id_tags().is_empty()
        {
            let _ = calculate_default_tx_profile(&self.config, charge_point_state);
            // TODO(styx3r): If transaction is running and SmartCharging is enabled set TxProfile
            // accordingly.
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------

fn calculate_default_tx_profile(
    config: &config::Config,
    charging_point_state: &mut ChargePointState,
) -> Result<(), CustomError> {
    let limit = calculate_max_current(config, charging_point_state)?;

    // If the current calculated max charging current does not differ more than 1.0 A compared
    // to the cached max charging current nothing will be changed.
    if let Some(cached_max_charging_current) = charging_point_state.get_max_current()
        && cached_max_charging_current - limit < 1.0
    {
        info!("Max. charging current won't be changed because difference is < 1.0 A");
        return Ok(());
    }

    charging_point_state.set_max_current(limit);

    const CONNECTOR_ID: i32 = 1;
    const CHARGING_PROFILE_ID: i32 = 1;
    const CHARGING_SCHEDULE_START_PERIOD: i32 = 0;
    const CHARGING_SCHEDULE_PERIOD_NUMBER_PHASES: Option<i32> = None;

    let charging_profile = ChargingProfileBuilder::new(
        CHARGING_PROFILE_ID,
        ChargingProfilePurposeType::TxDefaultProfile,
        ChargingProfileKindType::Relative,
        ChargingRateUnitType::A,
    )
    .add_charging_schedule_period(
        CHARGING_SCHEDULE_START_PERIOD,
        Decimal::from_f64_retain(limit)
            .ok_or(CustomError::Common(
                "Could not convert to Decimal!".to_owned(),
            ))?
            .round_dp(1),
        CHARGING_SCHEDULE_PERIOD_NUMBER_PHASES,
    )
    .get();

    let (uuid, set_charging_profile_request) =
        SetChargingProfileBuilder::new(CONNECTOR_ID, charging_profile)
            .build()
            .serialize()?;

    charging_point_state.add_request_to_send(RequestToSend {
        uuid: uuid.clone(),
        message_type: MessageTypeName::SetChargingProfile,
        payload: set_charging_profile_request,
    });

    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use fronius::FroniusApi;
    use ocpp::OcppMeterValuesHook;
    use std::sync::{Arc, Mutex};

    use super::*;

    static UNITTEST_CHARGING_POINT_SERIAL: &str = "SERIAL_NUMBER";

    static UNITTEST_HEARTBEAT_INTERVAL: u32 = 60;
    static UNITTEST_MAX_CHARGING_POWER: f64 = 11000.0;
    static UNITTEST_SYSTEM_VOLTAGE: f64 = 400.0;
    static UNITTEST_DEFAULT_CURRENT: f64 = 16.0;
    static UNITTEST_COS_PHI: f64 = 0.86;
    static UNITTEST_MINIMUM_CHARGING_CURRENT: f64 = 6.0;

    #[test]
    fn meter_values_request_empty() -> Result<(), Box<dyn std::error::Error>> {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            FroniusApi::default(),
            config::Config {
                websocket: config::Websocket {
                    ip: "127.0.0.1".to_owned(),
                    port: 8080,
                },
                charging_point: config::ChargePoint {
                    serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
                    heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
                    max_charging_power: UNITTEST_MAX_CHARGING_POWER,
                    default_system_voltage: UNITTEST_SYSTEM_VOLTAGE,
                    default_current: UNITTEST_DEFAULT_CURRENT,
                    default_cos_phi: UNITTEST_COS_PHI,
                    minimum_charging_current: UNITTEST_MINIMUM_CHARGING_CURRENT,
                    config_parameters: vec![],
                },
                id_tags: vec![],
                log_directory: "".to_owned(),
                fronius: config::Fronius {
                    username: "TEST".into(),
                    password: "TEST".into(),
                    url: "127.0.0.1:8081".into(),
                },
                awattar: config::Awattar {
                    base_url: "".to_owned(),
                },
                electric_vehicle: config::Ev {
                    average_watt_hours_needed: 30000,
                },
            },
        )));

        let mut charge_point_state = ChargePointState::default();
        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        Ok(())
    }

    #[test]
    fn meter_values_request() -> Result<(), Box<dyn std::error::Error>> {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            FroniusApi::default(),
            config::Config {
                websocket: config::Websocket {
                    ip: "127.0.0.1".to_owned(),
                    port: 8080,
                },
                charging_point: config::ChargePoint {
                    serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
                    heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
                    max_charging_power: UNITTEST_MAX_CHARGING_POWER,
                    default_system_voltage: UNITTEST_SYSTEM_VOLTAGE,
                    default_current: UNITTEST_DEFAULT_CURRENT,
                    default_cos_phi: UNITTEST_COS_PHI,
                    minimum_charging_current: UNITTEST_MINIMUM_CHARGING_CURRENT,
                    config_parameters: vec![],
                },
                id_tags: vec![],
                log_directory: "".to_owned(),
                fronius: config::Fronius {
                    username: "TEST".into(),
                    password: "TEST".into(),
                    url: "127.0.0.1:8081".into(),
                },
                awattar: config::Awattar {
                    base_url: "".to_owned(),
                },
                electric_vehicle: config::Ev {
                    average_watt_hours_needed: 30000,
                },
            },
        )));

        let mut charge_point_state = ChargePointState::new(0.9988504095416009, 6255.9, 9.0, 695.9);
        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let request_to_send = charge_point_state.get_requests_to_send().first();

        assert!(request_to_send.is_some());
        assert_eq!(
            request_to_send.unwrap().message_type,
            MessageTypeName::SetChargingProfile
        );

        assert_eq!(charge_point_state.get_max_current(), Some(15.0));

        Ok(())
    }

    #[test]
    fn meter_values_request_with_running_remote_transaction() -> Result<(), Box<dyn std::error::Error>> {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            FroniusApi::default(),
            config::Config {
                websocket: config::Websocket {
                    ip: "127.0.0.1".to_owned(),
                    port: 8080,
                },
                charging_point: config::ChargePoint {
                    serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
                    heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
                    max_charging_power: UNITTEST_MAX_CHARGING_POWER,
                    default_system_voltage: UNITTEST_SYSTEM_VOLTAGE,
                    default_current: UNITTEST_DEFAULT_CURRENT,
                    default_cos_phi: UNITTEST_COS_PHI,
                    minimum_charging_current: UNITTEST_MINIMUM_CHARGING_CURRENT,
                    config_parameters: vec![],
                },
                id_tags: vec![],
                log_directory: "".to_owned(),
                fronius: config::Fronius {
                    username: "TEST".into(),
                    password: "TEST".into(),
                    url: "127.0.0.1:8081".into(),
                },
                awattar: config::Awattar {
                    base_url: "".to_owned(),
                },
                electric_vehicle: config::Ev {
                    average_watt_hours_needed: 30000,
                },
            },
        )));

        let mut charge_point_state = ChargePointState::new(0.9988504095416009, 6255.9, 9.0, 695.9);
        charge_point_state.add_remote_transaction_id_tag("TEST_TAG".to_owned());

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let request_to_send = charge_point_state.get_requests_to_send().first();

        assert!(request_to_send.is_none());
        assert!(charge_point_state.get_max_current().is_none());

        Ok(())
    }
}
