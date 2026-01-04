use crate::config;
use crate::ocpp::builders::{
    MessageBuilder, set_charging_profile_builder::SetChargingProfileBuilder,
};
use crate::ocpp::ocpp_types::{CustomError, MessageTypeName, serialze_ocpp_request};
use crate::ocpp::{ChargePointState, RequestToSend};

use env_logger::builder;
use rust_ocpp::v1_6::messages::{meter_values, set_charging_profile};
use rust_ocpp::v1_6::types::{
    ChargingProfile, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    ChargingSchedule, ChargingSchedulePeriod, Location, MeterValue, Phase, SampledValue,
    UnitOfMeasure,
};

use rust_decimal::Decimal;
use uuid::Uuid;

use log::info;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_meter_values_request(
    meter_values_request: &meter_values::MeterValuesRequest,
    charge_point_config: &config::ChargePoint,
    charge_point_state: &mut ChargePointState,
) -> Result<meter_values::MeterValuesResponse, CustomError> {
    info!(
        "Received {} with content: {:?}",
        MessageTypeName::MeterValues,
        meter_values_request
    );

    let mut system_voltage: Option<f64> = None;
    for meter_value in &meter_values_request.meter_value {
        for sampled_value in &meter_value.sampled_value {
            match sampled_value.measurand {
                Some(rust_ocpp::v1_6::types::Measurand::CurrentOffered) => {
                    charge_point_state.latest_current = sampled_value.value.parse::<f64>().ok();
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerOffered) => {
                    charge_point_state.latest_power = match sampled_value.value.parse::<f64>() {
                        Ok(v) => match sampled_value.unit {
                            Some(UnitOfMeasure::Kw) => Some(v * 1000.0),
                            _ => Some(v),
                        },
                        _ => None,
                    }
                }
                Some(rust_ocpp::v1_6::types::Measurand::Voltage) => {
                    match (system_voltage, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            system_voltage = Some(v);
                        }
                        (Some(latest_voltage), Ok(v)) => {
                            system_voltage = Some(latest_voltage + v);
                        }
                        _ => {}
                    };
                }
                _ => {}
            }
        }
    }

    charge_point_state.latest_voltage = system_voltage;

    if let Some(latest_current) = charge_point_state.latest_current
        && let Some(latest_power) = charge_point_state.latest_power
        && let Some(latest_voltage) = charge_point_state.latest_voltage
    {
        charge_point_state.latest_cos_phi = Some(latest_power / (latest_voltage * latest_current));

        info!(
            "Calculated cos(phi): {} / ({} * {}) = {}",
            latest_power,
            latest_voltage,
            latest_current,
            charge_point_state.latest_cos_phi.unwrap_or(1.0)
        );

        let _ = calculate_max_charging_current(charge_point_config, charge_point_state);
        // TODO(styx3r): If transaction is running and SmartCharging is enabled set TxProfile
        // accordingly.
    }

    Ok(meter_values::MeterValuesResponse {})
}

//-------------------------------------------------------------------------------------------------

fn calculate_max_charging_current(
    charge_point_config: &config::ChargePoint,
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
        ChargingProfilePurposeType::ChargePointMaxProfile,
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
    use rust_ocpp::v1_6::types::{Measurand, ReadingContext, ValueFormat};

    use super::*;

    static UNITTEST_CONNECTOR_ID: u32 = 1;
    static UNITTEST_CHARGING_POINT_MODEL: &str = "MODEL";
    static UNITTEST_CHARGE_POINT_VENDOR: &str = "VENDOR";
    static UNITTEST_CHARGING_POINT_SERIAL: &str = "SERIAL_NUMBER";

    static UNITTEST_HEARTBEAT_INTERVAL: u32 = 60;
    static UNITTEST_MAX_CHARGING_POWER: f64 = 11000.0;
    static UNITTEST_SYSTEM_VOLTAGE: f64 = 400.0;
    static UNITTEST_DEFAULT_CURRENT: f64 = 16.0;
    static UNITTEST_COS_PHI: f64 = 0.86;
    static UNITTEST_MINIMUM_CHARGING_CURRENT: f64 = 6.0;

    #[test]
    fn meter_values_request_empty() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        let response = handle_meter_values_request(
            &meter_values::MeterValuesRequest {
                connector_id: UNITTEST_CONNECTOR_ID,
                transaction_id: None,
                meter_value: vec![],
            },
            &config::ChargePoint {
                serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
                heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
                max_charging_power: UNITTEST_MAX_CHARGING_POWER,
                default_system_voltage: UNITTEST_SYSTEM_VOLTAGE,
                default_current: UNITTEST_DEFAULT_CURRENT,
                default_cos_phi: UNITTEST_COS_PHI,
                minimum_charging_current: UNITTEST_MINIMUM_CHARGING_CURRENT,
                config_parameters: vec![],
            },
            &mut charge_point_state,
        )?;

        assert_eq!(response, meter_values::MeterValuesResponse {});

        Ok(())
    }

    /*
     * [2, "402411419", "MeterValues", {
     *     "transactionId": 1768416593,
     *     "meterValue": [
     *     {
     *         "timestamp": "2026-01-14T18:53:53Z",
     *         "sampledValue": [
     *             {
     *                 "format": "Raw",
     *                 "context": "Sample.Periodic",
     *                 "measurand": "Power.Offered",
     *                 "unit": "kW",
     *                 "value": "6.2559"
     *             },
     *             {
     *                 "context": "Sample.Periodic",
     *                 "measurand": "Current.Offered",
     *                 "unit": "A",
     *                 "value": "9"
     *             },
     *             {
     *                 "format": "Raw",
     *                 "location": "Outlet",
     *                 "context": "Sample.Periodic",
     *                 "phase": "L1",
     *                 "measurand": "Voltage",
     *                 "unit": "V",
     *                 "value": "231.7"
     *             },
     *             {
     *                 "format": "Raw",
     *                 "location": "Outlet",
     *                 "context": "Sample.Periodic",
     *                 "phase": "L2",
     *                 "measurand": "Voltage",
     *                 "unit": "V",
     *                 "value": "231.8"
     *             },
     *             {
     *                 "format": "Raw",
     *                 "location": "Outlet",
     *                 "context": "Sample.Periodic",
     *                 "phase": "L3",
     *                 "measurand": "Voltage",
     *                 "unit": "V",
     *                 "value": "232.4"
     *             }
     *         ]
     *     }],
     *     "connectorId": 1
     * }]
     */
    #[test]
    fn meter_values_request() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        let response = handle_meter_values_request(
            &meter_values::MeterValuesRequest {
                connector_id: UNITTEST_CONNECTOR_ID,
                transaction_id: None,
                meter_value: vec![MeterValue {
                    timestamp: chrono::offset::Utc::now(),
                    sampled_value: vec![
                        SampledValue {
                            location: None,
                            phase: None,
                            format: Some(ValueFormat::Raw),
                            context: Some(ReadingContext::SamplePeriodic),
                            measurand: Some(Measurand::PowerOffered),
                            unit: Some(UnitOfMeasure::Kw),
                            value: "6.2559".to_owned(),
                        },
                        SampledValue {
                            location: None,
                            phase: None,
                            format: None,
                            context: Some(ReadingContext::SamplePeriodic),
                            measurand: Some(Measurand::CurrentOffered),
                            unit: Some(UnitOfMeasure::A),
                            value: "9".to_owned(),
                        },
                        SampledValue {
                            location: Some(Location::Outlet),
                            phase: Some(Phase::L1),
                            format: Some(ValueFormat::Raw),
                            context: Some(ReadingContext::SamplePeriodic),
                            measurand: Some(Measurand::Voltage),
                            unit: Some(UnitOfMeasure::V),
                            value: "231.7".to_owned(),
                        },
                        SampledValue {
                            location: Some(Location::Outlet),
                            phase: Some(Phase::L2),
                            format: Some(ValueFormat::Raw),
                            context: Some(ReadingContext::SamplePeriodic),
                            measurand: Some(Measurand::Voltage),
                            unit: Some(UnitOfMeasure::V),
                            value: "231.8".to_owned(),
                        },
                        SampledValue {
                            location: Some(Location::Outlet),
                            phase: Some(Phase::L3),
                            format: Some(ValueFormat::Raw),
                            context: Some(ReadingContext::SamplePeriodic),
                            measurand: Some(Measurand::Voltage),
                            unit: Some(UnitOfMeasure::V),
                            value: "232.4".to_owned(),
                        },
                    ],
                }],
            },
            &config::ChargePoint {
                serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
                heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
                max_charging_power: UNITTEST_MAX_CHARGING_POWER,
                default_system_voltage: UNITTEST_SYSTEM_VOLTAGE,
                default_current: UNITTEST_DEFAULT_CURRENT,
                default_cos_phi: UNITTEST_COS_PHI,
                minimum_charging_current: UNITTEST_MINIMUM_CHARGING_CURRENT,
                config_parameters: vec![],
            },
            &mut charge_point_state,
        )?;

        assert_eq!(response, meter_values::MeterValuesResponse {});

        assert_eq!(charge_point_state.latest_current, Some(9.0));
        assert_eq!(charge_point_state.latest_voltage, Some(695.9));
        assert_eq!(charge_point_state.latest_power, Some(6255.9));
        assert_eq!(charge_point_state.latest_cos_phi, Some(0.9988504095416009));

        assert_eq!(charge_point_state.max_current, Some(15.0));

        Ok(())
    }
}
