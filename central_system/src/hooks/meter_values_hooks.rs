use crate::{OcppHooks, hooks::calculate_max_current};
use awattar::AwattarApi;
use fronius::FroniusApi;
use log::info;

use std::sync::{Arc, Mutex};

use chrono::Duration;

use config::config;
use ocpp::{
    ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    CustomError, Decimal, MessageBuilder, MessageTypeName, RequestToSend,
    charging_profile_builder::ChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
};

//-------------------------------------------------------------------------------------------------

impl<T: FroniusApi, U: AwattarApi> ocpp::OcppMeterValuesHook for OcppHooks<T, U> {
    fn evaluate(
        &mut self,
        charging_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        calculate_power_flow_realtime_data(
            &self.config,
            charging_point_state,
            Arc::clone(&self.fronius_api),
        );

        if let Some(_) = charging_point_state.get_latest_current()
            && let Some(_) = charging_point_state.get_latest_power()
            && let Some(_) = charging_point_state.get_latest_voltage()
            && let Some(_) = charging_point_state.get_latest_cos_phi()
        {
            if !charging_point_state.get_smart_charging() {
                calculate_default_tx_profile(&self.config, charging_point_state)?;
            } else if charging_point_state.get_smart_charging() {
                self.calculate_grid_based_smart_charging_tx_profile(charging_point_state)?;
            }
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------

fn calculate_power_flow_realtime_data<T: FroniusApi>(
    config: &config::Config,
    charging_point_state: &mut ChargePointState,
    fronius_api: Arc<Mutex<T>>,
) {
    if let Ok(powerflow_realtime_data) = fronius_api.lock().unwrap().get_power_flow_realtime_data()
    {
        let site_powerflow_realtime_data = powerflow_realtime_data.body.data.site;

        if let Some(power_pv) = site_powerflow_realtime_data.p_pv
            && let Some(power_load) = site_powerflow_realtime_data.p_load
            && let Some(power_akku) = site_powerflow_realtime_data.p_akku
        {
            let overproduction = if power_akku < 0.0 {
                power_pv + power_load + power_akku
            } else {
                power_pv + power_load
            };

            info!("Current PV overproduction {}W", overproduction);

            charging_point_state.add_pv_overproduction(overproduction);
        }

        let threshold = Duration::minutes(15);
        let expected_vector_size = (threshold.as_seconds_f64()
            / config.charging_point.heartbeat_interval as f64)
            .ceil() as usize;

        let pv_overproduction = charging_point_state.get_pv_overproduction();
        if pv_overproduction.len() == expected_vector_size {
            let pv_overproduction_average =
                pv_overproduction.iter().sum::<f64>() / pv_overproduction.len() as f64;

            info!(
                "PV overproduction in the last {}: {}",
                threshold, pv_overproduction_average
            );

            if let Some(latest_cos_phi) = charging_point_state.get_latest_cos_phi()
                && let Some(latest_voltage) = charging_point_state.get_latest_voltage()
            {
                info!(
                    "Resulting in {}A charging current",
                    pv_overproduction_average / (latest_cos_phi * latest_voltage)
                );
            }

            charging_point_state.remove_first_element_pv_overproduction();
        }
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
    use awattar::awattar_mock::AwattarApiMock;
    use chrono::Utc;
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
