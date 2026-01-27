use crate::builders::{MessageBuilder, set_charging_profile_builder::SetChargingProfileBuilder};
use crate::ocpp_types::{CustomError, MessageTypeName};
use crate::{ChargePointState, OcppMeterValuesHook, RequestToSend};
use config::config;
use std::sync::{Arc, Mutex};

use rust_ocpp::v1_6::messages::meter_values;
use rust_ocpp::v1_6::types::{
    ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType, Location,
    MeterValue, Phase, SampledValue, UnitOfMeasure,
};

use rust_decimal::Decimal;

use log::{error, info};

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_meter_values_request<T: OcppMeterValuesHook>(
    meter_values_request: &meter_values::MeterValuesRequest,
    charge_point_state: &mut ChargePointState,
    hook: Arc<Mutex<T>>,
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
                    charge_point_state.latest_current =
                        sampled_value.value.parse::<f64>().ok();
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerOffered) => {
                    charge_point_state.latest_power =
                        match sampled_value.value.parse::<f64>() {
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

    match hook
        .lock()
        .unwrap()
        .evaluate(charge_point_state)
    {
        Err(err) => error!("Hook failed: {}", err),
        _ => {}
    }

    Ok(meter_values::MeterValuesResponse {})
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

    struct Hook {
        pub called: bool,
    }

    impl Hook {
        pub fn default() -> Self {
            Self { called: false }
        }
    }

    impl OcppMeterValuesHook for Hook {
        fn evaluate(
            &mut self,
            _meter_values: &mut ChargePointState,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.called = true;
            Ok(())
        }
    }

    #[test]
    fn meter_values_request_empty() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        let hook = Arc::new(Mutex::new(Hook::default()));
        let response = handle_meter_values_request(
            &meter_values::MeterValuesRequest {
                connector_id: UNITTEST_CONNECTOR_ID,
                transaction_id: None,
                meter_value: vec![],
            },
            &mut charge_point_state,
            Arc::clone(&hook)
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
        let hook = Arc::new(Mutex::new(Hook::default()));

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
            &mut charge_point_state,
            Arc::clone(&hook)
        )?;

        assert_eq!(response, meter_values::MeterValuesResponse {});

        assert_eq!(charge_point_state.latest_current, Some(9.0));
        assert_eq!(charge_point_state.latest_voltage, Some(695.9));
        assert_eq!(charge_point_state.latest_power, Some(6255.9));

        assert_eq!(charge_point_state.latest_cos_phi, None);
        assert_eq!(charge_point_state.max_current, None);

        Ok(())
    }
}
