use log::info;

use crate::OcppHooks;

use config::config::ChargePoint;
use ocpp::{
    ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    CustomError, Decimal, MessageBuilder, MessageTypeName, RequestToSend,
    SetChargingProfileBuilder,
};

//-------------------------------------------------------------------------------------------------

impl ocpp::OcppMeterValuesHook for OcppHooks {
    fn evaluate(
        &mut self,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(latest_current) = charge_point_state.latest_current
            && let Some(latest_power) = charge_point_state.latest_power
            && let Some(latest_voltage) = charge_point_state.latest_voltage
        {
            charge_point_state.latest_cos_phi =
                Some(latest_power / (latest_voltage * latest_current));

            info!(
                "Calculated cos(phi): {} / ({} * {}) = {}",
                latest_power,
                latest_voltage,
                latest_current,
                charge_point_state.latest_cos_phi.unwrap_or(1.0)
            );

            let _ =
                calculate_max_charging_current(&self.charge_point_config, charge_point_state);
            // TODO(styx3r): If transaction is running and SmartCharging is enabled set TxProfile
            // accordingly.
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------

fn calculate_max_charging_current(
    charge_point_config: &ChargePoint,
    charging_point_state: &mut ChargePointState,
) -> Result<(), CustomError> {
    let max_charging_power: f64 = charge_point_config.max_charging_power.into();

    let max_charging_current = (max_charging_power
        / (charging_point_state
            .latest_voltage
            .unwrap_or(charge_point_config.default_system_voltage)
            * charging_point_state
                .latest_cos_phi
                .unwrap_or(charge_point_config.default_cos_phi)))
    .clamp(
        charge_point_config.minimum_charging_current,
        charge_point_config.default_current,
    )
    .floor();

    // If the current calculated max charging current does not differ more than 1.0 A compared
    // to the cached max charging current nothing will be changed.
    if let Some(cached_max_charging_current) = charging_point_state.max_current
        && cached_max_charging_current - max_charging_current < 1.0
    {
        info!("Max. charging current won't be changed because difference is < 1.0 A");
        return Ok(());
    }

    charging_point_state.max_current = Some(max_charging_current);

    info!(
        "Setting max. charging current to {} A",
        max_charging_current
    );

    let limit = Decimal::from_f64_retain(max_charging_current)
        .ok_or(CustomError::Common(
            "Could not convert to Decimal!".to_owned(),
        ))?
        .round_dp(1);

    const CONNECTOR_ID: i32 = 0;
    const CHARGING_PROFILE_ID: i32 = 1;
    const CHARGING_SCHEDULE_START_PERIOD: i32 = 0;
    const CHARGING_SCHEDULE_PERIOD_NUMBER_PHASES: Option<i32> = None;

    let (uuid, set_charging_profile_request) = SetChargingProfileBuilder::new(
        CONNECTOR_ID,
        CHARGING_PROFILE_ID,
        ChargingProfilePurposeType::TxDefaultProfile,
        ChargingProfileKindType::Relative,
        ChargingRateUnitType::A,
    )
    .add_charging_schedule_period(
        CHARGING_SCHEDULE_START_PERIOD,
        &limit,
        CHARGING_SCHEDULE_PERIOD_NUMBER_PHASES,
    )
    .build()
    .serialize()?;

    charging_point_state.requests_to_send.push(RequestToSend {
        uuid: uuid.clone(),
        message_type: MessageTypeName::SetChargingProfile,
        payload: set_charging_profile_request,
    });

    Ok(())
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use ocpp::OcppMeterValuesHook;
    use fronius::FroniusApi;
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
            ChargePoint {
                serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
                heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
                max_charging_power: UNITTEST_MAX_CHARGING_POWER,
                default_system_voltage: UNITTEST_SYSTEM_VOLTAGE,
                default_current: UNITTEST_DEFAULT_CURRENT,
                default_cos_phi: UNITTEST_COS_PHI,
                minimum_charging_current: UNITTEST_MINIMUM_CHARGING_CURRENT,
                config_parameters: vec![],
            },
        )));

        let mut charge_point_state = ChargePointState::default();
        hook.lock()
            .unwrap()
            .evaluate(&mut charge_point_state)?;

        Ok(())
    }

    #[test]
    fn meter_values_request() -> Result<(), Box<dyn std::error::Error>> {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            FroniusApi::default(),
            ChargePoint {
                serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
                heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
                max_charging_power: UNITTEST_MAX_CHARGING_POWER,
                default_system_voltage: UNITTEST_SYSTEM_VOLTAGE,
                default_current: UNITTEST_DEFAULT_CURRENT,
                default_cos_phi: UNITTEST_COS_PHI,
                minimum_charging_current: UNITTEST_MINIMUM_CHARGING_CURRENT,
                config_parameters: vec![],
            },
        )));

        let mut charge_point_state = ChargePointState {
            latest_cos_phi: Some(0.9988504095416009),
            latest_power: Some(6255.9),
            latest_current: Some(9.0),
            latest_voltage: Some(695.9),
            max_current: None,
            requests_to_send: vec![],
            requests_awaiting_confirmation: vec![],
            running_transactions: vec![],
        };

        hook.lock()
            .unwrap()
            .evaluate(&mut charge_point_state)?;

        let request_to_send = charge_point_state
                .requests_to_send
                .first();

        assert!(request_to_send.is_some());
        assert_eq!(
            charge_point_state
                .requests_to_send
                .first()
                .unwrap()
                .message_type,
            MessageTypeName::SetChargingProfile
        );

        assert_eq!(charge_point_state.max_current, Some(15.0));

        Ok(())
    }
}
