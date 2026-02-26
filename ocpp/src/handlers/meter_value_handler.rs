use crate::ocpp_types::{CustomError, MessageTypeName};
use crate::{ChargePointState, OcppMeterValuesHook};
use std::sync::{Arc, Mutex};

use rust_ocpp::v1_6::messages::meter_values;
use rust_ocpp::v1_6::types::UnitOfMeasure;

use log::{error, info};

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_meter_values_request<T: OcppMeterValuesHook>(
    meter_values_request: &meter_values::MeterValuesRequest,
    charge_point_state: &mut ChargePointState,
    hook: Arc<Mutex<T>>,
) -> Result<meter_values::MeterValuesResponse, CustomError> {
    info!("Received {}", MessageTypeName::MeterValues);

    let mut current_export: Option<f64> = None;
    let mut current_import: Option<f64> = None;

    let mut power_active_export: Option<f64> = None;
    let mut power_active_import: Option<f64> = None;

    let mut power_reactive_export: Option<f64> = None;
    let mut power_reactive_import: Option<f64> = None;

    let mut system_voltage: Option<f64> = None;

    for meter_value in &meter_values_request.meter_value {
        for sampled_value in &meter_value.sampled_value {
            match sampled_value.measurand {
                Some(rust_ocpp::v1_6::types::Measurand::CurrentExport) => {
                    match (current_export, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            current_export = Some(v);
                        }
                        (Some(current_export_measurand), Ok(v)) => {
                            current_export = Some(current_export_measurand + v);
                        }
                        _ => {}
                    };
                }
                Some(rust_ocpp::v1_6::types::Measurand::CurrentImport) => {
                    match (current_import, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            current_import = Some(v);
                        }
                        (Some(current_import_measurand), Ok(v)) => {
                            current_import = Some(current_import_measurand + v);
                        }
                        _ => {}
                    };
                }
                Some(rust_ocpp::v1_6::types::Measurand::CurrentOffered) => {
                    charge_point_state.measurand.current_offered =
                        sampled_value.value.parse::<f64>().ok();
                }
                Some(rust_ocpp::v1_6::types::Measurand::EnergyActiveImportRegister) => {
                    charge_point_state.measurand.energy_active_import_register =
                        sampled_value.value.parse::<f64>().ok();
                }
                Some(rust_ocpp::v1_6::types::Measurand::EnergyReactiveImportRegister) => {
                    charge_point_state.measurand.energy_reactive_import_register =
                        sampled_value.value.parse::<f64>().ok();
                }
                Some(rust_ocpp::v1_6::types::Measurand::Frequency) => {
                    charge_point_state.measurand.frequency =
                        sampled_value.value.parse::<f64>().ok();
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerActiveExport) => {
                    match (power_active_export, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            power_active_export = Some(v);
                        }
                        (Some(power_active_export_measurand), Ok(v)) => {
                            power_active_export = Some(power_active_export_measurand + v);
                        }
                        _ => {}
                    };
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerActiveImport) => {
                    match (power_active_import, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            power_active_import = Some(v);
                        }
                        (Some(power_active_import_measurand), Ok(v)) => {
                            power_active_import = Some(power_active_import_measurand + v);
                        }
                        _ => {}
                    };
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerFactor) => {
                    charge_point_state.measurand.power_factor =
                        match sampled_value.value.parse::<f64>() {
                            Ok(v) => Some(v),
                            _ => None,
                        }
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerOffered) => {
                    charge_point_state.measurand.power_offered =
                        match sampled_value.value.parse::<f64>() {
                            Ok(v) => match sampled_value.unit {
                                Some(UnitOfMeasure::Kw) => Some(v * 1000.0),
                                _ => Some(v),
                            },
                            _ => None,
                        }
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerReactiveExport) => {
                    match (power_reactive_export, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            power_reactive_export = Some(v);
                        }
                        (Some(power_reactive_export_measurand), Ok(v)) => {
                            power_reactive_export = Some(power_reactive_export_measurand + v);
                        }
                        _ => {}
                    };
                }
                Some(rust_ocpp::v1_6::types::Measurand::PowerReactiveImport) => {
                    match (power_reactive_import, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            power_reactive_import = Some(v);
                        }
                        (Some(power_reactive_import_measurand), Ok(v)) => {
                            power_reactive_import = Some(power_reactive_import_measurand + v);
                        }
                        _ => {}
                    };
                }
                Some(rust_ocpp::v1_6::types::Measurand::Rpm) => {
                    charge_point_state.measurand.rpm = match sampled_value.value.parse::<f64>() {
                        Ok(v) => Some(v),
                        _ => None,
                    }
                }
                Some(rust_ocpp::v1_6::types::Measurand::SoC) => {
                    charge_point_state.measurand.state_of_charge =
                        match sampled_value.value.parse::<f64>() {
                            Ok(v) => Some(v),
                            _ => None,
                        }
                }
                Some(rust_ocpp::v1_6::types::Measurand::Temperature) => {
                    charge_point_state.measurand.temperature =
                        match sampled_value.value.parse::<f64>() {
                            Ok(v) => Some(v),
                            _ => None,
                        }
                }
                Some(rust_ocpp::v1_6::types::Measurand::Voltage) => {
                    match (system_voltage, sampled_value.value.parse::<f64>()) {
                        (None, Ok(v)) => {
                            system_voltage = Some(v);
                        }
                        (Some(latest_voltage_measurand), Ok(v)) => {
                            system_voltage = Some(latest_voltage_measurand + v);
                        }
                        _ => {}
                    };
                }
                _ => {}
            }
        }
    }

    charge_point_state.measurand.current_import = current_import;
    charge_point_state.measurand.power_active_import = power_active_import;
    charge_point_state.measurand.voltage = system_voltage;
    charge_point_state.measurand.power_reactive_export = power_reactive_export;
    charge_point_state.measurand.power_reactive_import = power_reactive_import;

    if let Some(current_offered) = charge_point_state.measurand.current_offered
        && let Some(power_offered) = charge_point_state.measurand.power_offered
        && let Some(voltage) = charge_point_state.measurand.voltage
        && power_offered != 0.0
        && voltage != 0.0
        && current_offered != 0.0
    {
        charge_point_state.latest_cos_phi = Some(power_offered / (voltage * current_offered));

        info!(
            "Calculated cos(phi): {} / ({} * {}) = {}",
            power_offered,
            voltage,
            current_offered,
            charge_point_state.get_latest_cos_phi().unwrap_or(1.0)
        );
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
        assert_eq!(charge_point_state.latest_cos_phi, Some(0.9988504095416009));

        assert_eq!(charge_point_state.max_current, None);

        Ok(())
    }
}
