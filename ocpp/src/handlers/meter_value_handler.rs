use crate::ocpp_types::{CustomError, MessageTypeName};
use crate::{ChargePointState, OcppMeterValuesHook};
use std::sync::{Arc, Mutex};

use rust_ocpp::v1_6::messages::meter_values;
use rust_ocpp::v1_6::types::{UnitOfMeasure, Measurand};

use log::{error, info};

//-------------------------------------------------------------------------------------------------

macro_rules! sample_value {
    ($measurand_type:pat, $sampled_value:expr, $destination:expr) => {
        match $sampled_value.measurand {
            Some($measurand_type) => {
                match $sampled_value.value.parse::<f64>() {
                    Ok(v) => match $sampled_value.unit {
                        Some(UnitOfMeasure::Kw) => {
                            $destination = Some(v * 1000.0);
                        }
                        _ => {
                            $destination = Some(v);
                        }
                    },
                    _ => {}
                };
            }
            _ => {}
        }
    };
}

macro_rules! sample_value_from_all_phases {
    ($measurand_type:pat, $sampled_value:expr, $destination:expr) => {
        match $sampled_value.measurand {
            Some($measurand_type) => {
                match ($destination, $sampled_value.value.parse::<f64>()) {
                    (None, Ok(v)) => match $sampled_value.unit {
                        Some(UnitOfMeasure::Kw) => {
                            $destination = Some(v * 1000.0);
                        },
                        _ => {
                            $destination = Some(v);
                        }
                    }
                    (Some(destination), Ok(v)) => match $sampled_value.unit {
                        Some(UnitOfMeasure::Kw) => {
                            $destination = Some(destination + (v * 1000.0));
                        },
                        _ => {
                            $destination = Some(destination + v);
                        }
                    }
                    _ => {}
                };
            }
            _ => {}
        }
    };
}

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_meter_values_request<T: OcppMeterValuesHook>(
    meter_values_request: &meter_values::MeterValuesRequest,
    charge_point_state: &mut ChargePointState,
    hook: Arc<Mutex<T>>,
) -> Result<meter_values::MeterValuesResponse, CustomError> {
    info!("Received {}", MessageTypeName::MeterValues);

    charge_point_state.measurand = crate::Measurand::default();

    for meter_value in &meter_values_request.meter_value {
        for sampled_value in &meter_value.sampled_value {
            sample_value_from_all_phases!(
                Measurand::CurrentExport,
                sampled_value,
                charge_point_state.measurand.current_export
            );
            sample_value_from_all_phases!(
                Measurand::CurrentImport,
                sampled_value,
                charge_point_state.measurand.current_import
            );
            sample_value!(
                Measurand::CurrentOffered,
                sampled_value,
                charge_point_state.measurand.current_offered
            );
            sample_value!(
                Measurand::EnergyActiveExportRegister,
                sampled_value,
                charge_point_state.measurand.energy_active_export_register
            );
            sample_value!(
                Measurand::EnergyActiveImportRegister,
                sampled_value,
                charge_point_state.measurand.energy_active_import_register
            );
            sample_value!(
                Measurand::EnergyReactiveExportRegister,
                sampled_value,
                charge_point_state.measurand.energy_reactive_export_register
            );
            sample_value!(
                Measurand::EnergyReactiveImportRegister,
                sampled_value,
                charge_point_state.measurand.energy_reactive_import_register
            );
            sample_value!(
                Measurand::EnergyActiveExportInterval,
                sampled_value,
                charge_point_state.measurand.energy_active_export_interval
            );
            sample_value!(
                Measurand::EnergyActiveImportInterval,
                sampled_value,
                charge_point_state.measurand.energy_active_import_interval
            );
            sample_value!(
                Measurand::EnergyReactiveExportInterval,
                sampled_value,
                charge_point_state.measurand.energy_reactive_export_interval
            );
            sample_value!(
                Measurand::EnergyReactiveImportInterval,
                sampled_value,
                charge_point_state.measurand.energy_reactive_import_interval
            );
            sample_value!(
                Measurand::Frequency,
                sampled_value,
                charge_point_state.measurand.frequency
            );
            sample_value_from_all_phases!(
                Measurand::PowerActiveExport,
                sampled_value,
                charge_point_state.measurand.power_active_export
            );
            sample_value_from_all_phases!(
                Measurand::PowerActiveImport,
                sampled_value,
                charge_point_state.measurand.power_active_import
            );
            sample_value!(
                Measurand::PowerFactor,
                sampled_value,
                charge_point_state.measurand.power_factor
            );
            sample_value!(
                Measurand::PowerOffered,
                sampled_value,
                charge_point_state.measurand.power_offered
            );
            sample_value_from_all_phases!(
                Measurand::PowerReactiveExport,
                sampled_value,
                charge_point_state.measurand.power_reactive_export
            );
            sample_value_from_all_phases!(
                Measurand::PowerReactiveImport,
                sampled_value,
                charge_point_state.measurand.power_reactive_import
            );
            sample_value!(
                Measurand::Rpm,
                sampled_value,
                charge_point_state.measurand.rpm
            );
            sample_value!(
                Measurand::SoC,
                sampled_value,
                charge_point_state.measurand.state_of_charge
            );
            sample_value!(
                Measurand::Temperature,
                sampled_value,
                charge_point_state.measurand.temperature
            );
            sample_value_from_all_phases!(
                Measurand::Voltage,
                sampled_value,
                charge_point_state.measurand.voltage
            );
        }
    }

    match hook.lock().unwrap().evaluate(charge_point_state) {
        Err(err) => error!("Hook failed: {}", err),
        _ => {}
    }

    Ok(meter_values::MeterValuesResponse {})
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rust_ocpp::v1_6::types::{
        Location, Measurand, MeterValue, Phase, ReadingContext, SampledValue, ValueFormat,
    };

    use super::*;

    static UNITTEST_CONNECTOR_ID: u32 = 1;

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
            _charge_point_state: &mut ChargePointState,
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
            Arc::clone(&hook),
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
            Arc::clone(&hook),
        )?;

        assert_eq!(response, meter_values::MeterValuesResponse {});

        assert_eq!(charge_point_state.measurand.current_offered, Some(9.0));
        assert_eq!(charge_point_state.measurand.voltage, Some(695.9));
        assert_eq!(charge_point_state.measurand.power_offered, Some(6255.9));

        assert_eq!(charge_point_state.max_current, None);

        Ok(())
    }
}
