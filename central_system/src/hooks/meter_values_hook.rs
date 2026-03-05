use crate::OcppHooks;
use awattar::AwattarApi;
use config::config::SmartChargingMode;
use fronius::FroniusApi;

use std::sync::Arc;

use ocpp::{
    ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    CustomError, Decimal, MessageBuilder, MessageTypeName, RequestToSend,
    charging_profile_builder::ChargingProfileBuilder,
    clear_charging_profile_builder::ClearChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
};

use crate::hooks::{CONNECTOR_ID, TX_PV_CHARGING_PROFILE_ID, TX_PV_CHARGING_STACK_LEVEL};

//-------------------------------------------------------------------------------------------------

impl<T: FroniusApi, U: AwattarApi> ocpp::OcppMeterValuesHook for OcppHooks<T, U> {
    /// Handles charging profile adjustments depending on the current used charging profile.
    ///
    /// Following smart charging modes are possible:
    ///
    ///   * Instant: Adjusts the maximum charging current for the TxDefaultProfile.
    ///
    ///   * PVOverProductionAndGridBased: If there is enough PV overproduction a charging profile
    ///                                   with the maximum possible charging current will be set.
    ///                                   If there is not enough overproduction and a PV charging
    ///                                   profile is currently in use it will be cleared.
    ///                                   Additionally the grid based TxProfile max current will be
    ///                                   updated if necessary.
    ///
    ///   * PVOverProduction: AFAIK this is the same as `PVOverProductionAndGridBased` without
    ///                       updating the grid based TxProfile.
    fn evaluate(
        &mut self,
        charging_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.calculate_cos_phi(charging_point_state)?;

        let possible_pv_charging_current = self.calculate_possible_pv_charging_current(
            charging_point_state,
            Arc::clone(&self.fronius_api),
        );

        let charging_profile_max_current =
            self.get_updated_max_charging_current(charging_point_state);

        if charging_profile_max_current.is_none() && possible_pv_charging_current.is_none() {
            return Ok(());
        }

        let smart_charging_mode = charging_point_state.get_smart_charging_mode();

        match smart_charging_mode {
            SmartChargingMode::Instant => {
                if let Some(charging_profile_max_current) = charging_profile_max_current {
                    calculate_default_tx_profile(
                        charging_point_state,
                        charging_profile_max_current,
                    )?;
                }
            }
            SmartChargingMode::PVOverProductionAndGridBased
            | SmartChargingMode::PVOverProduction => {
                if let Some(possible_pv_charging_current) = possible_pv_charging_current {
                    if possible_pv_charging_current
                        > self.config.charging_point.minimum_charging_current
                    {
                        let possible_charging_current_decimal = Decimal::from_f64_retain(
                            possible_pv_charging_current.clamp(
                                self.config.charging_point.minimum_charging_current,
                                self.config.charging_point.default_current,
                            ),
                        )
                        .ok_or(CustomError::Common(
                            "Could not convert possible charging current into decimal".to_string(),
                        ))?
                        .round_dp(1);

                        self.build_pv_tx_profile(
                            charging_point_state,
                            possible_charging_current_decimal,
                        )?;
                    } else if charging_point_state
                        .get_active_charging_profile(TX_PV_CHARGING_PROFILE_ID)
                        .is_some()
                    {
                        let (uuid, clear_charging_profile) = ClearChargingProfileBuilder::new(
                            Some(TX_PV_CHARGING_PROFILE_ID),
                            Some(CONNECTOR_ID),
                            Some(ChargingProfilePurposeType::TxProfile),
                            Some(TX_PV_CHARGING_STACK_LEVEL as i32),
                        )
                        .build()
                        .serialize()?;

                        charging_point_state.add_request_to_send(ocpp::RequestToSend {
                            uuid: uuid.clone(),
                            message_type: MessageTypeName::ClearChargingProfile,
                            payload: clear_charging_profile,
                        });

                        charging_point_state.remove_charging_profile(TX_PV_CHARGING_PROFILE_ID);
                    }
                }
            }
        }

        if let Some(charging_profile_max_current) = charging_profile_max_current
            && smart_charging_mode == SmartChargingMode::PVOverProductionAndGridBased
        {
            self.build_grid_based_smart_charging_tx_profile(
                charging_point_state,
                charging_profile_max_current,
            )?;
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
    use ocpp::{ChargingProfile, OcppMeterValuesHook};
    use serde::de::DeserializeOwned;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use super::*;
    use rust_ocpp::v1_6::{
        messages::{
            clear_charging_profile::ClearChargingProfileRequest,
            set_charging_profile::SetChargingProfileRequest,
        },
        types::{ChargingSchedule, ChargingSchedulePeriod},
    };

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

    fn test_config() -> config::Config {
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
        }
    }

    fn parse_message_request<T: DeserializeOwned>(payload: &str) -> T {
        let message_request =
            serde_json::from_str::<(u32, String, String, serde_json::Value)>(payload)
                .expect("Could not deserialize request");

        serde_json::from_value::<T>(message_request.3).expect("Could not deserialize payload")
    }

    #[test]
    fn empty_meter_values_request() -> Result<(), Box<dyn std::error::Error>> {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            test_config(),
        )));

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(default_powerflow_realtime_data());

        let mut charge_point_state = ChargePointState::default();
        assert!(hook.lock().unwrap().evaluate(&mut charge_point_state).is_err());

        Ok(())
    }

    #[test]
    fn valid_meter_values_request() -> Result<(), Box<dyn std::error::Error>> {
        let config = test_config();
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            config.clone(),
        )));

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(default_powerflow_realtime_data());

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let request_to_send = charge_point_state.get_requests_to_send().first();

        assert!(request_to_send.is_some());
        assert_eq!(
            request_to_send.unwrap().message_type,
            MessageTypeName::SetChargingProfile
        );

        let set_charging_profile_request = parse_message_request::<SetChargingProfileRequest>(
            request_to_send.unwrap().payload.as_str(),
        );

        static EXPECTED_INSTANT_PV_PROFILE_STACK_LEVEL: u32 = 0;
        static EXPECTED_CURRENT_LIMIT: i64 = 15;
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .stack_level,
            EXPECTED_INSTANT_PV_PROFILE_STACK_LEVEL
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_kind,
            ChargingProfileKindType::Relative
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_purpose,
            ChargingProfilePurposeType::TxDefaultProfile
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_rate_unit,
            ChargingRateUnitType::A
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_schedule_period,
            vec![ChargingSchedulePeriod {
                start_period: 0,
                limit: Decimal::new(EXPECTED_CURRENT_LIMIT, 0),
                number_phases: None
            }]
        );

        Ok(())
    }

    #[test]
    fn no_power_flow_and_unchanged_max_charging_current() -> Result<(), Box<dyn std::error::Error>>
    {
        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            test_config(),
        )));

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        charge_point_state.set_max_current(15.0);

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        assert!(charge_point_state.get_requests_to_send().is_empty());
        Ok(())
    }

    #[test]
    fn max_power_flow_and_unchanged_max_charging_current() -> Result<(), Box<dyn std::error::Error>>
    {
        // Setting minimum charging current to 1A.
        let mut config = test_config();
        config.charging_point.minimum_charging_current = 1.0;

        // Setting intervals in a way that only ONE element is used as average
        config
            .charging_point
            .config_parameters
            .push(config::ConfigSetting {
                key: "MeterValueSampleInterval".to_owned(),
                value: "60".to_owned(),
            });
        config.photo_voltaic.moving_window_size_in_minutes = 1;

        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            config.clone(),
        )));

        let mut power_flow_realtime_data = default_powerflow_realtime_data();
        power_flow_realtime_data.body.data.site.p_pv = Some(14000.0);
        power_flow_realtime_data.body.data.site.p_load = Some(-100.0);
        power_flow_realtime_data.body.data.site.p_akku = Some(-100.0);

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(power_flow_realtime_data);

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        charge_point_state.set_max_current(15.0);
        charge_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProductionAndGridBased);

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let pv_charging_profile_request = charge_point_state.get_requests_to_send().first();

        assert!(pv_charging_profile_request.is_some());
        assert_eq!(
            pv_charging_profile_request.unwrap().message_type,
            MessageTypeName::SetChargingProfile
        );

        let set_charging_profile_request = parse_message_request::<SetChargingProfileRequest>(
            pv_charging_profile_request.unwrap().payload.as_str(),
        );

        static EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL: u32 = 1;
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .stack_level,
            EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_kind,
            ChargingProfileKindType::Relative
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_purpose,
            ChargingProfilePurposeType::TxProfile
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_rate_unit,
            ChargingRateUnitType::A
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_schedule_period,
            vec![ChargingSchedulePeriod {
                start_period: 0,
                limit: Decimal::from_f64_retain(config.charging_point.default_current)
                    .unwrap()
                    .round_dp(1),
                number_phases: None
            }]
        );
        Ok(())
    }

    #[test]
    fn max_power_flow_and_changed_max_charging_current() -> Result<(), Box<dyn std::error::Error>> {
        // Setting minimum charging current to 1A.
        let mut config = test_config();
        config.charging_point.minimum_charging_current = 1.0;

        // Setting intervals in a way that only ONE element is used as average
        config
            .charging_point
            .config_parameters
            .push(config::ConfigSetting {
                key: "MeterValueSampleInterval".to_owned(),
                value: "60".to_owned(),
            });
        config.photo_voltaic.moving_window_size_in_minutes = 1;

        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            config.clone(),
        )));

        let mut power_flow_realtime_data = default_powerflow_realtime_data();
        power_flow_realtime_data.body.data.site.p_pv = Some(14000.0);
        power_flow_realtime_data.body.data.site.p_load = Some(-100.0);
        power_flow_realtime_data.body.data.site.p_akku = Some(-100.0);

        static PERIOD_START: i64 = 300;
        static PERIOD_END: i64 = 500;
        hook.lock()
            .unwrap()
            .awattar_api
            .lock()
            .unwrap()
            .set_response(awattar::Period {
                start_timestamp: PERIOD_START,
                end_timestamp: PERIOD_END,
                average_price: 20.0,
            });

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(power_flow_realtime_data);

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        charge_point_state.set_max_current(4.0);
        charge_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProductionAndGridBased);
        charge_point_state.add_charging_profile(&ChargingProfile {
            charging_profile_id: 2,
            transaction_id: None,
            stack_level: 0,
            charging_profile_purpose: ChargingProfilePurposeType::TxProfile,
            charging_profile_kind: ChargingProfileKindType::Absolute,
            recurrency_kind: None,
            valid_from: None,
            valid_to: None,
            charging_schedule: ChargingSchedule {
                duration: None,
                start_schedule: None,
                charging_rate_unit: ChargingRateUnitType::A,
                charging_schedule_period: vec![
                    ChargingSchedulePeriod {
                        start_period: 0,
                        limit: Decimal::new(0, 0),
                        number_phases: None,
                    },
                    ChargingSchedulePeriod {
                        start_period: PERIOD_START as i32,
                        limit: Decimal::new(5, 0),
                        number_phases: None,
                    },
                ],
                min_charging_rate: None,
            },
        });

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let mut handle = charge_point_state.get_requests_to_send().iter();
        let pv_charging_profile_request = handle.next();

        assert!(pv_charging_profile_request.is_some());
        assert_eq!(
            pv_charging_profile_request.unwrap().message_type,
            MessageTypeName::SetChargingProfile
        );

        let set_charging_profile_request = parse_message_request::<SetChargingProfileRequest>(
            pv_charging_profile_request.unwrap().payload.as_str(),
        );

        static EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL: u32 = 1;
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .stack_level,
            EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_kind,
            ChargingProfileKindType::Relative
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_purpose,
            ChargingProfilePurposeType::TxProfile
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_rate_unit,
            ChargingRateUnitType::A
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_schedule_period,
            vec![ChargingSchedulePeriod {
                start_period: 0,
                limit: Decimal::from_f64_retain(config.charging_point.default_current)
                    .unwrap()
                    .round_dp(1),
                number_phases: None
            }]
        );

        let grid_charging_profile_request = handle.next();

        assert!(grid_charging_profile_request.is_some());
        assert_eq!(
            grid_charging_profile_request.unwrap().message_type,
            MessageTypeName::SetChargingProfile
        );

        let set_charging_profile_for_grid_request =
            parse_message_request::<SetChargingProfileRequest>(
                grid_charging_profile_request.unwrap().payload.as_str(),
            );

        static EXPECTED_GRID_CHARGING_PROFILE_STACK_LEVEL: u32 = 0;
        assert_eq!(
            set_charging_profile_for_grid_request
                .cs_charging_profiles
                .stack_level,
            EXPECTED_GRID_CHARGING_PROFILE_STACK_LEVEL
        );
        assert_eq!(
            set_charging_profile_for_grid_request
                .cs_charging_profiles
                .charging_profile_kind,
            ChargingProfileKindType::Absolute
        );
        assert_eq!(
            set_charging_profile_for_grid_request
                .cs_charging_profiles
                .charging_profile_purpose,
            ChargingProfilePurposeType::TxProfile
        );
        assert_eq!(
            set_charging_profile_for_grid_request
                .cs_charging_profiles
                .charging_schedule
                .charging_rate_unit,
            ChargingRateUnitType::A
        );
        assert_eq!(
            set_charging_profile_for_grid_request
                .cs_charging_profiles
                .charging_schedule
                .charging_schedule_period,
            vec![
                ChargingSchedulePeriod {
                    start_period: 0,
                    limit: Decimal::new(0, 0),
                    number_phases: None
                },
                ChargingSchedulePeriod {
                    start_period: PERIOD_START as i32,
                    limit: Decimal::new(15, 0),
                    number_phases: None
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn pv_overproduction_charging_profile() -> Result<(), Box<dyn std::error::Error>> {
        // Setting minimum charging current to 1A.
        let mut config = test_config();
        config.charging_point.minimum_charging_current = 1.0;

        // Setting intervals in a way that only ONE element is used as average
        config
            .charging_point
            .config_parameters
            .push(config::ConfigSetting {
                key: "MeterValueSampleInterval".to_owned(),
                value: "60".to_owned(),
            });
        config.photo_voltaic.moving_window_size_in_minutes = 1;

        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            config.clone(),
        )));

        let mut power_flow_realtime_data = default_powerflow_realtime_data();
        power_flow_realtime_data.body.data.site.p_pv = Some(14000.0);
        power_flow_realtime_data.body.data.site.p_load = Some(-100.0);
        power_flow_realtime_data.body.data.site.p_akku = Some(-100.0);

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(power_flow_realtime_data);

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        charge_point_state.set_max_current(4.0);
        charge_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProduction);

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let mut handle = charge_point_state.get_requests_to_send().iter();
        let pv_charging_profile_request = handle.next();

        assert!(pv_charging_profile_request.is_some());
        assert_eq!(
            pv_charging_profile_request.unwrap().message_type,
            MessageTypeName::SetChargingProfile
        );

        let set_charging_profile_request = parse_message_request::<SetChargingProfileRequest>(
            pv_charging_profile_request.unwrap().payload.as_str(),
        );

        static EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL: u32 = 1;
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .stack_level,
            EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_kind,
            ChargingProfileKindType::Relative
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_purpose,
            ChargingProfilePurposeType::TxProfile
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_rate_unit,
            ChargingRateUnitType::A
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_schedule_period,
            vec![ChargingSchedulePeriod {
                start_period: 0,
                limit: Decimal::from_f64_retain(config.charging_point.default_current)
                    .unwrap()
                    .round_dp(1),
                number_phases: None
            }]
        );

        assert!(handle.next().is_none());
        Ok(())
    }

    #[test]
    fn insufficient_power_flow_and_unchanged_max_charging_current()
    -> Result<(), Box<dyn std::error::Error>> {
        // Setting minimum charging current to 1A.
        let mut config = test_config();
        config.charging_point.minimum_charging_current = 1.0;

        // Setting intervals in a way that only ONE element is used as average
        config
            .charging_point
            .config_parameters
            .push(config::ConfigSetting {
                key: "MeterValueSampleInterval".to_owned(),
                value: "60".to_owned(),
            });
        config.photo_voltaic.moving_window_size_in_minutes = 1;

        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            config.clone(),
        )));

        let mut power_flow_realtime_data = default_powerflow_realtime_data();
        power_flow_realtime_data.body.data.site.p_pv = Some(500.0);
        power_flow_realtime_data.body.data.site.p_load = Some(-100.0);
        power_flow_realtime_data.body.data.site.p_akku = Some(-100.0);

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(power_flow_realtime_data);

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        charge_point_state.set_max_current(15.0);
        charge_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProductionAndGridBased);

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        assert!(charge_point_state.get_requests_to_send().is_empty());
        Ok(())
    }

    #[test]
    fn insufficient_power_flow_with_existing_pv_profile_and_unchanged_max_charging_current()
    -> Result<(), Box<dyn std::error::Error>> {
        // Setting minimum charging current to 1A.
        let mut config = test_config();
        config.charging_point.minimum_charging_current = 1.0;

        // Setting intervals in a way that only ONE element is used as average
        config
            .charging_point
            .config_parameters
            .push(config::ConfigSetting {
                key: "MeterValueSampleInterval".to_owned(),
                value: "60".to_owned(),
            });
        config.photo_voltaic.moving_window_size_in_minutes = 1;

        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            config.clone(),
        )));

        let mut power_flow_realtime_data = default_powerflow_realtime_data();
        power_flow_realtime_data.body.data.site.p_pv = Some(500.0);
        power_flow_realtime_data.body.data.site.p_load = Some(-100.0);
        power_flow_realtime_data.body.data.site.p_akku = Some(-100.0);

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(power_flow_realtime_data);

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        charge_point_state.set_max_current(15.0);
        charge_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProductionAndGridBased);
        charge_point_state.add_charging_profile(&ChargingProfile {
            charging_profile_id: 5,
            transaction_id: None,
            stack_level: 1,
            charging_profile_purpose: ChargingProfilePurposeType::TxProfile,
            charging_profile_kind: ChargingProfileKindType::Relative,
            recurrency_kind: None,
            valid_from: None,
            valid_to: None,
            charging_schedule: ChargingSchedule {
                duration: None,
                start_schedule: None,
                charging_rate_unit: ChargingRateUnitType::A,
                charging_schedule_period: vec![ChargingSchedulePeriod {
                    start_period: 0,
                    limit: Decimal::new(6, 0),
                    number_phases: None,
                }],
                min_charging_rate: None,
            },
        });

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let message_request = charge_point_state.get_requests_to_send().first();
        assert!(message_request.is_some());

        assert_eq!(
            message_request.unwrap().message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile_request = parse_message_request::<ClearChargingProfileRequest>(
            message_request.unwrap().payload.as_str(),
        );

        assert_eq!(
            clear_charging_profile_request,
            ClearChargingProfileRequest {
                id: Some(5),
                connector_id: Some(1),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(1)
            }
        );

        Ok(())
    }

    #[test]
    fn changed_power_flow_with_existing_pv_profile_and_unchanged_max_charging_current()
    -> Result<(), Box<dyn std::error::Error>> {
        // Setting minimum charging current to 1A.
        let mut config = test_config();
        config.charging_point.minimum_charging_current = 1.0;

        // Setting intervals in a way that only ONE element is used as average
        config
            .charging_point
            .config_parameters
            .push(config::ConfigSetting {
                key: "MeterValueSampleInterval".to_owned(),
                value: "60".to_owned(),
            });
        config.photo_voltaic.moving_window_size_in_minutes = 1;

        let hook = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            config.clone(),
        )));

        let mut power_flow_realtime_data = default_powerflow_realtime_data();
        power_flow_realtime_data.body.data.site.p_pv = Some(9000.0);
        power_flow_realtime_data.body.data.site.p_load = Some(-100.0);
        power_flow_realtime_data.body.data.site.p_akku = Some(-100.0);

        hook.lock()
            .unwrap()
            .fronius_api
            .lock()
            .unwrap()
            .power_flow_realtime_data = Some(power_flow_realtime_data);

        static POWER: f64 = 6255.9;
        static CURRENT: f64 = 9.0;
        static VOLTAGE: f64 = 695.9;

        let mut charge_point_state = ChargePointState::new(POWER, CURRENT, VOLTAGE);
        charge_point_state.set_max_current(15.0);
        charge_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProductionAndGridBased);
        charge_point_state.add_charging_profile(&ChargingProfile {
            charging_profile_id: 5,
            transaction_id: None,
            stack_level: 1,
            charging_profile_purpose: ChargingProfilePurposeType::TxProfile,
            charging_profile_kind: ChargingProfileKindType::Relative,
            recurrency_kind: None,
            valid_from: None,
            valid_to: None,
            charging_schedule: ChargingSchedule {
                duration: None,
                start_schedule: None,
                charging_rate_unit: ChargingRateUnitType::A,
                charging_schedule_period: vec![ChargingSchedulePeriod {
                    start_period: 0,
                    limit: Decimal::new(16, 0),
                    number_phases: None,
                }],
                min_charging_rate: None,
            },
        });

        hook.lock().unwrap().evaluate(&mut charge_point_state)?;

        let mut handle = charge_point_state.get_requests_to_send().iter();
        let pv_charging_profile_request = handle.next();

        assert!(pv_charging_profile_request.is_some());
        assert_eq!(
            pv_charging_profile_request.unwrap().message_type,
            MessageTypeName::SetChargingProfile
        );

        let set_charging_profile_request = parse_message_request::<SetChargingProfileRequest>(
            pv_charging_profile_request.unwrap().payload.as_str(),
        );

        static EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL: u32 = 1;
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .stack_level,
            EXPECTED_PV_CHARGING_PROFILE_STACK_LEVEL
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_kind,
            ChargingProfileKindType::Relative
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_profile_purpose,
            ChargingProfilePurposeType::TxProfile
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_rate_unit,
            ChargingRateUnitType::A
        );
        assert_eq!(
            set_charging_profile_request
                .cs_charging_profiles
                .charging_schedule
                .charging_schedule_period,
            vec![ChargingSchedulePeriod {
                start_period: 0,
                limit: Decimal::new(127, 1),
                number_phases: None
            }]
        );

        Ok(())
    }
}
