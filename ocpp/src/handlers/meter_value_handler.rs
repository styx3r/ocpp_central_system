use crate::ocpp_types::{CustomError, MessageTypeName};
use crate::{ChargePointState, OcppMeterValuesHook, PhaseMeasurand};
use std::sync::{Arc, Mutex};

use rust_ocpp::v1_6::messages::meter_values;
use rust_ocpp::v1_6::types::{Measurand, UnitOfMeasure};

use log::{error, info};

use uom::si::{
    electric_current::ampere, electric_potential::volt, energy::watt_hour, f64::*,
    frequency::hertz, power::watt, temperature_interval::degree_celsius,
};

//-------------------------------------------------------------------------------------------------

macro_rules! sample_value {
    ($measurand_type:pat, $sampled_value:expr, $destination:expr, $conversion:expr) => {
        match $sampled_value.measurand {
            Some($measurand_type) => {
                match $sampled_value.value.parse::<f64>() {
                    Ok(v) => match $sampled_value.unit {
                        Some(UnitOfMeasure::Kvarh)
                        | Some(UnitOfMeasure::Kw)
                        | Some(UnitOfMeasure::Kva)
                        | Some(UnitOfMeasure::Kvar) => {
                            $destination = Some($conversion(v * 1000.0));
                        }
                        _ => {
                            $destination = Some($conversion(v));
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
    ($measurand_type:pat, $sampled_value:expr, $destination:expr, $conversion:expr) => {
        match $sampled_value.measurand {
            Some($measurand_type) => {
                match $sampled_value.value.parse::<f64>() {
                    Ok(v) => match $sampled_value.unit {
                        Some(UnitOfMeasure::Kvarh)
                        | Some(UnitOfMeasure::Kw)
                        | Some(UnitOfMeasure::Kva)
                        | Some(UnitOfMeasure::Kvar) => {
                            $destination.measurands.push(PhaseMeasurand {
                                value: $conversion(v * 1000.0),
                                phase: $sampled_value.phase.clone().unwrap().into(),
                            });
                        }
                        _ => $destination.measurands.push(PhaseMeasurand {
                            value: $conversion(v),
                            phase: $sampled_value.phase.clone().unwrap().into(),
                        }),
                    },
                    _ => {}
                };
            }
            _ => {}
        }
    };
}

fn to_ampere(v: f64) -> ElectricCurrent {
    ElectricCurrent::new::<ampere>(v)
}

fn to_watt_hour(v: f64) -> Energy {
    Energy::new::<watt_hour>(v)
}

fn to_watt(v: f64) -> Power {
    Power::new::<watt>(v)
}

fn to_frequency(v: f64) -> Frequency {
    Frequency::new::<hertz>(v)
}

fn to_degree_celsius(v: f64) -> TemperatureInterval {
    TemperatureInterval::new::<degree_celsius>(v)
}

fn to_volt(v: f64) -> ElectricPotential {
    ElectricPotential::new::<volt>(v)
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
                charge_point_state.measurand.current_export,
                to_ampere
            );
            sample_value_from_all_phases!(
                Measurand::CurrentImport,
                sampled_value,
                charge_point_state.measurand.current_import,
                to_ampere
            );
            sample_value!(
                Measurand::CurrentOffered,
                sampled_value,
                charge_point_state.measurand.current_offered,
                to_ampere
            );
            sample_value!(
                Measurand::EnergyActiveExportRegister,
                sampled_value,
                charge_point_state.measurand.energy_active_export_register,
                to_watt_hour
            );
            sample_value!(
                Measurand::EnergyActiveImportRegister,
                sampled_value,
                charge_point_state.measurand.energy_active_import_register,
                to_watt_hour
            );
            sample_value!(
                Measurand::EnergyReactiveExportRegister,
                sampled_value,
                charge_point_state.measurand.energy_reactive_export_register,
                to_watt_hour
            );
            sample_value!(
                Measurand::EnergyReactiveImportRegister,
                sampled_value,
                charge_point_state.measurand.energy_reactive_import_register,
                to_watt_hour
            );
            sample_value!(
                Measurand::EnergyActiveExportInterval,
                sampled_value,
                charge_point_state.measurand.energy_active_export_interval,
                to_watt_hour
            );
            sample_value!(
                Measurand::EnergyActiveImportInterval,
                sampled_value,
                charge_point_state.measurand.energy_active_import_interval,
                to_watt_hour
            );
            sample_value!(
                Measurand::EnergyReactiveExportInterval,
                sampled_value,
                charge_point_state.measurand.energy_reactive_export_interval,
                to_watt_hour
            );
            sample_value!(
                Measurand::EnergyReactiveImportInterval,
                sampled_value,
                charge_point_state.measurand.energy_reactive_import_interval,
                to_watt_hour
            );
            sample_value!(
                Measurand::Frequency,
                sampled_value,
                charge_point_state.measurand.frequency,
                to_frequency
            );
            sample_value_from_all_phases!(
                Measurand::PowerActiveExport,
                sampled_value,
                charge_point_state.measurand.power_active_export,
                to_watt
            );
            sample_value_from_all_phases!(
                Measurand::PowerActiveImport,
                sampled_value,
                charge_point_state.measurand.power_active_import,
                to_watt
            );
            sample_value!(
                Measurand::PowerFactor,
                sampled_value,
                charge_point_state.measurand.power_factor,
                |v| { v }
            );
            sample_value!(
                Measurand::PowerOffered,
                sampled_value,
                charge_point_state.measurand.power_offered,
                to_watt
            );
            sample_value_from_all_phases!(
                Measurand::PowerReactiveExport,
                sampled_value,
                charge_point_state.measurand.power_reactive_export,
                to_watt
            );
            sample_value_from_all_phases!(
                Measurand::PowerReactiveImport,
                sampled_value,
                charge_point_state.measurand.power_reactive_import,
                to_watt
            );
            sample_value!(
                Measurand::Rpm,
                sampled_value,
                charge_point_state.measurand.rpm,
                |v| { v }
            );
            sample_value!(
                Measurand::SoC,
                sampled_value,
                charge_point_state.measurand.state_of_charge,
                |v| { v }
            );
            sample_value!(
                Measurand::Temperature,
                sampled_value,
                charge_point_state.measurand.temperature,
                to_degree_celsius
            );
            sample_value_from_all_phases!(
                Measurand::Voltage,
                sampled_value,
                charge_point_state.measurand.voltage,
                to_volt
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

    use crate::MultiPhaseMeasurand;

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

        assert_eq!(
            charge_point_state.measurand.current_offered,
            Some(ElectricCurrent::new::<ampere>(9.0))
        );
        assert_eq!(
            charge_point_state.measurand.voltage,
            MultiPhaseMeasurand {
                measurands: vec![
                    PhaseMeasurand {
                        value: ElectricPotential::new::<volt>(231.7),
                        phase: crate::Phase::L1
                    },
                    PhaseMeasurand {
                        value: ElectricPotential::new::<volt>(231.8),
                        phase: crate::Phase::L2
                    },
                    PhaseMeasurand {
                        value: ElectricPotential::new::<volt>(232.4),
                        phase: crate::Phase::L3
                    }
                ]
            }
        );
        assert_eq!(
            charge_point_state.measurand.power_offered,
            Some(Power::new::<watt>(6255.9))
        );

        assert_eq!(charge_point_state.max_current, None);

        Ok(())
    }
}
