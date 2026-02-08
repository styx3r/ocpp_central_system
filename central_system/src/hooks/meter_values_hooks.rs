use crate::OcppHooks;
use awattar::AwattarApi;
use fronius::FroniusApi;

use std::sync::Arc;

use ocpp::{
    ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    CustomError, Decimal, MessageBuilder, MessageTypeName, RequestToSend,
    charging_profile_builder::ChargingProfileBuilder,
    clear_charging_profile_builder::ClearChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
};

//-------------------------------------------------------------------------------------------------

static CONNECTOR_ID: i32 = 1;

//-------------------------------------------------------------------------------------------------

impl<T: FroniusApi, U: AwattarApi> ocpp::OcppMeterValuesHook for OcppHooks<T, U> {
    fn evaluate(
        &mut self,
        charging_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let possible_charging_current = self.calculate_power_flow_realtime_data(
            charging_point_state,
            Arc::clone(&self.fronius_api),
        );

        if let Some(_) = charging_point_state.get_latest_current_offered()
            && let Some(_) = charging_point_state.get_latest_power_offered()
            && let Some(_) = charging_point_state.get_latest_voltage()
            && let Some(_) = charging_point_state.get_latest_cos_phi()
        {
            let charging_profile_max_current =
                self.get_updated_max_charging_current(charging_point_state)?;

            if !charging_point_state.get_smart_charging() {
                calculate_default_tx_profile(charging_point_state, charging_profile_max_current)?;
            } else if charging_point_state.get_smart_charging() {
                if let Some(possible_charging_current) = possible_charging_current {
                    if possible_charging_current
                        > self.config.charging_point.minimum_charging_current
                    {
                        let possible_charging_current_decimal =
                            Decimal::from_f64_retain(possible_charging_current)
                                .ok_or(CustomError::Common(
                                    "Could not convert possible charging current into decimal"
                                        .to_string(),
                                ))?
                                .round_dp(1);

                        self.calculate_pv_tx_profile(
                            charging_point_state,
                            possible_charging_current_decimal,
                        )?;
                    } else {
                        let (uuid, clear_charging_profile) = ClearChargingProfileBuilder::new(
                            Some(crate::hooks::TX_PV_CHARGING_PROFILE_ID),
                            Some(CONNECTOR_ID),
                            Some(ChargingProfilePurposeType::TxProfile),
                            Some(crate::hooks::TX_PV_CHARGING_STACK_LEVEL as i32),
                        )
                        .build()
                        .serialize()?;

                        charging_point_state.add_request_to_send(ocpp::RequestToSend {
                            uuid: uuid.clone(),
                            message_type: MessageTypeName::ClearChargingProfile,
                            payload: clear_charging_profile,
                        });
                    }
                }

                self.calculate_grid_based_smart_charging_tx_profile(
                    charging_point_state,
                    charging_profile_max_current,
                )?;
            }
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------

fn calculate_default_tx_profile(
    charging_point_state: &mut ChargePointState,
    charging_profile_max_current: Decimal,
) -> Result<(), CustomError> {
    static CHARGING_PROFILE_ID: i32 = 1;
    static CHARGING_SCHEDULE_START_PERIOD: i32 = 0;
    static CHARGING_SCHEDULE_PERIOD_NUMBER_PHASES: Option<i32> = None;

    let charging_profile = ChargingProfileBuilder::new(
        CHARGING_PROFILE_ID,
        ChargingProfilePurposeType::TxDefaultProfile,
        ChargingProfileKindType::Relative,
        ChargingRateUnitType::A,
    )
    .add_charging_schedule_period(
        CHARGING_SCHEDULE_START_PERIOD,
        charging_profile_max_current,
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
    use awattar::awattar_mock::AwattarApiMock;
    use chrono::Utc;
    use config::config;
    use fronius::{
        Data, FroniusMock, PowerFlowRealtimeData, PowerFlowRealtimeDataBody,
        PowerFlowRealtimeDataHeader, Site, Smartloads, Status,
    };
    use ocpp::OcppMeterValuesHook;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use super::*;

    static UNITTEST_CHARGING_POINT_SERIAL: &str = "SERIAL_NUMBER";

    static UNITTEST_HEARTBEAT_INTERVAL: u32 = 60;
    static UNITTEST_MAX_CHARGING_POWER: f64 = 11000.0;
    static UNITTEST_SYSTEM_VOLTAGE: f64 = 400.0;
    static UNITTEST_DEFAULT_CURRENT: f64 = 16.0;
    static UNITTEST_COS_PHI: f64 = 0.86;
    static UNITTEST_MINIMUM_CHARGING_CURRENT: f64 = 6.0;

    fn default_powerflow_realtime_data() -> PowerFlowRealtimeData {
        PowerFlowRealtimeData {
            body: PowerFlowRealtimeDataBody {
                data: Data {
                    inverters: HashMap::default(),
                    site: Site {
                        mode: String::default(),
                        battery_standby: false,
                        backup_mode: false,
                        p_grid: None,
                        p_load: None,
                        p_akku: None,
                        p_pv: None,
                        rel_self_consumption: None,
                        rel_autonomy: None,
                        meter_location: String::default(),
                        e_day: None,
                        e_year: None,
                        e_total: None,
                    },
                    smartloads: Smartloads {
                        ohmpilots: HashMap::default(),
                        ohmpilot_ecos: HashMap::default(),
                    },
                    secondart_meters: HashMap::default(),
                    version: String::default(),
                },
            },
            head: PowerFlowRealtimeDataHeader {
                request_arguments: HashMap::default(),
                status: Status {
                    code: 0,
                    reason: String::default(),
                    user_message: String::default(),
                },
                timestamp: Utc::now(),
            },
        }
    }

    #[test]
    fn meter_values_request_empty() -> Result<(), Box<dyn std::error::Error>> {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
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
                photo_voltaic: config::PhotoVoltaic {
                    moving_window_size_in_minutes: 15,
                },
            },
        )));

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(default_powerflow_realtime_data());

        let mut charge_point_state = ChargePointState::default();
        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        Ok(())
    }

    #[test]
    fn meter_values_request() -> Result<(), Box<dyn std::error::Error>> {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
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
                photo_voltaic: config::PhotoVoltaic {
                    moving_window_size_in_minutes: 15,
                },
            },
        )));

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(default_powerflow_realtime_data());

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
}
