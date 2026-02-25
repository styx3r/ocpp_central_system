mod authorize_hook;
mod meter_values_hook;
mod status_notification_hook;

use ::config::config::SmartChargingMode;
use chrono::{DateTime, Duration, Utc};
use config::config;
use fronius::FroniusApi;

use log::{info, warn};
use ocpp::{
    ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    CustomError, Decimal, MessageBuilder, MessageTypeName,
    charging_profile_builder::ChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
};

use awattar::AwattarApi;

use std::sync::{Arc, Mutex};

//-------------------------------------------------------------------------------------------------

/// Default connector ID.
/// NOTE: This may only be used on WallBoxes with only one connector!
static CONNECTOR_ID: i32 = 1;

//-------------------------------------------------------------------------------------------------

static TX_GRID_BASED_CHARGING_PROFILE_ID: i32 = 2;
/// PV profile consists of two profiles. First profile which does not allow any power and the
/// second profile which sets the allowed power.
static TX_PV_PREPARATION_CHARGING_PROFILE_ID: i32 = 4;
static TX_PV_CHARGING_PROFILE_ID: i32 = 5;
/// Stack level for the second profile is one to be able to keep the first one.
static TX_PV_CHARGING_STACK_LEVEL: u32 = 1;

//-------------------------------------------------------------------------------------------------

/// Wrapper struct which encapsulated all necessary interfaces to implement the provided hooks by
/// [`ocpp`]
pub struct OcppHooks<T: FroniusApi, U: AwattarApi> {
    /// Interface to the FroniusApi
    fronius_api: Arc<Mutex<T>>,
    /// Interface to the awattar API
    awattar_api: Arc<Mutex<U>>,
    /// Overall config object
    config: config::Config,
    /// Vector of calculated PV overproduction
    pv_overproduction: Vec<f64>,
}

impl<T: FroniusApi, U: AwattarApi> OcppHooks<T, U> {
    /// Creates a new wrapper object.
    pub fn new(
        fronius_api: Arc<Mutex<T>>,
        awattar_api: Arc<Mutex<U>>,
        config: config::Config,
    ) -> Self {
        Self {
            fronius_api,
            awattar_api,
            config,
            pv_overproduction: vec![],
        }
    }

    /// Calculates current possible maximum charging current (A). Returns None if the difference
    /// between the current and the latest maximum charging current is < 1.0 A
    fn get_updated_max_charging_current(
        &mut self,
        charge_point_state: &mut ChargePointState,
    ) -> Option<Decimal> {
        let limit = calculate_max_current(&self.config, charge_point_state).ok();
        if limit.is_none() {
            return None;
        }

        let limit = limit.unwrap();

        // If the current calculated max charging current does not differ more than 1.0 A compared
        // to the cached max charging current nothing will be changed.
        if let Some(cached_max_charging_current) = charge_point_state.get_max_current()
            && (cached_max_charging_current - limit).abs() < 1.0
        {
            info!("Max. charging current won't be changed because difference is < 1.0 A");
            return None;
        }

        charge_point_state.set_max_current(limit);
        let charging_profile_max_current = Decimal::from_f64_retain(limit);
        if charging_profile_max_current.is_none() {
            return None;
        }

        charging_profile_max_current
    }

    /// Builds the grid based smart charging TX profile. There are two cases possible
    ///
    ///   * No grid based TX profile is currently in use and thus the cheapest charging period
    ///     will be calculated based on the awattar API.
    ///
    ///   * There is already a TX profile currently in use and thus the limit (A) will be updated.
    ///
    /// NOTE: The update logic within this method is needed because this profile uses valid_from
    ///       and valid_to. Those values MUST NOT be changed upon limit change.
    pub fn build_grid_based_smart_charging_tx_profile(
        &self,
        charge_point_state: &mut ChargePointState,
        charging_profile_max_current: Decimal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let grid_based_charging_profile = if let Some(grid_based_smart_charging_profile) =
            charge_point_state.get_active_charging_profile(TX_GRID_BASED_CHARGING_PROFILE_ID)
        {
            let mut grid_based_smart_charging_profile_handle =
                grid_based_smart_charging_profile.clone();

            // NOTE: This RELIES on the fact that the second charging schedule period is the cheapest_period
            grid_based_smart_charging_profile_handle
                .charging_schedule
                .charging_schedule_period[1]
                .limit = charging_profile_max_current;

            grid_based_smart_charging_profile_handle
        } else {
            let cheapest_period = self
                .awattar_api
                .clone()
                .lock()
                .unwrap()
                .update_price_chart(&self.config)?;

            let start_timestamp = DateTime::from_timestamp_millis(cheapest_period.start_timestamp)
                .ok_or(CustomError::Common(
                    "Could not convert start timestamp".to_owned(),
                ))?;
            let end_timestamp = DateTime::from_timestamp_millis(cheapest_period.end_timestamp)
                .ok_or(CustomError::Common(
                    "Could not convert end timestamp".to_owned(),
                ))?;

            let now = Utc::now();
            let charging_profile = ChargingProfileBuilder::new(
                TX_GRID_BASED_CHARGING_PROFILE_ID,
                ChargingProfilePurposeType::TxProfile,
                ChargingProfileKindType::Absolute,
                ChargingRateUnitType::A,
            )
            .set_valid_from(now)
            .set_valid_to(end_timestamp)
            .set_charging_schedule_duration((end_timestamp - now).num_seconds() as i32)
            .set_start_schedule_timestamp(now)
            .add_charging_schedule_period(0, Decimal::new(0, 0), None)
            .add_charging_schedule_period(
                (start_timestamp - now).num_seconds() as i32,
                charging_profile_max_current,
                None,
            )
            .get();

            charging_profile
        };

        charge_point_state.add_charging_profile(&grid_based_charging_profile);
        charge_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProductionAndGridBased);

        let (uuid, set_charging_profile_request) =
            SetChargingProfileBuilder::new(CONNECTOR_ID, grid_based_charging_profile)
                .build()
                .serialize()?;

        charge_point_state.add_request_to_send(ocpp::RequestToSend {
            uuid: uuid.clone(),
            message_type: MessageTypeName::SetChargingProfile,
            payload: set_charging_profile_request,
        });

        Ok(())
    }

    /// Builds the PV based smart charging profile. If there is already a PV charging profile in
    /// use the limit (A) will be updated. Additionally the charging profile will only be changed
    /// if the difference of the current limit and the new limit is < 1.0 A.
    fn build_pv_tx_profile(
        &mut self,
        charging_point_state: &mut ChargePointState,
        charging_profile_max_current: Decimal,
    ) -> Result<(), CustomError> {
        static CHARGING_SCHEDULE_START_PERIOD: i32 = 0;
        static CHARGING_SCHEDULE_PERIOD_NUMBER_PHASES: Option<i32> = None;

        if let Some(existing_pv_charging_profile) =
            charging_point_state.get_active_charging_profile(TX_PV_CHARGING_PROFILE_ID)
        {
            if (existing_pv_charging_profile
                .charging_schedule
                .charging_schedule_period
                .first()
                .unwrap() // NOTE: Unwrap only safe because TX_PV_CHARGING_PROFILE is guaranteed to
                //       consist of ONE schedule exclusively.
                .limit
                - charging_profile_max_current)
                .abs()
                <= Decimal::new(1, 0)
            {
                info!("PV ChargingProfile won't be updated because difference is < 1.0A");
                return Ok(());
            }

            charging_point_state.remove_charging_profile(TX_PV_CHARGING_PROFILE_ID);
        }

        let charging_profile = ChargingProfileBuilder::new(
            TX_PV_CHARGING_PROFILE_ID,
            ChargingProfilePurposeType::TxProfile,
            ChargingProfileKindType::Relative,
            ChargingRateUnitType::A,
        )
        .add_charging_schedule_period(
            CHARGING_SCHEDULE_START_PERIOD,
            charging_profile_max_current,
            CHARGING_SCHEDULE_PERIOD_NUMBER_PHASES,
        )
        .set_stack_level(TX_PV_CHARGING_STACK_LEVEL)
        .get();

        charging_point_state.add_charging_profile(&charging_profile);
        charging_point_state.set_smart_charging_mode(SmartChargingMode::PVOverProduction);

        let (uuid, set_charging_profile_request) =
            SetChargingProfileBuilder::new(CONNECTOR_ID, charging_profile)
                .build()
                .serialize()?;

        charging_point_state.add_request_to_send(ocpp::RequestToSend {
            uuid: uuid.clone(),
            message_type: MessageTypeName::SetChargingProfile,
            payload: set_charging_profile_request,
        });

        Ok(())
    }

    /// Calculates possible PV charging current w.r.t produced PV power, general load, power used
    /// to load the battery and if available power used by the charging EV.
    ///
    /// $                                       | P_pv + P_load + P_battery IFF P_battery < 0.0
    /// $                                       |
    /// $P_overproduction = P_active_imported + |
    /// $                                       |
    /// $                                       | P_pv + P_load             ELSE
    ///
    /// The possible charging current will only be calculated if enough measurements have been
    /// received. The amount of needed measurements is configurable with the
    /// `moving_window_size_in_minutes` config parameter.
    fn calculate_possible_pv_charging_current(
        &mut self,
        charging_point_state: &mut ChargePointState,
        fronius_api: Arc<Mutex<T>>,
    ) -> Option<f64> {
        if let Ok(powerflow_realtime_data) =
            fronius_api.lock().unwrap().get_power_flow_realtime_data()
        {
            let site_powerflow_realtime_data = powerflow_realtime_data.body.data.site;

            if let Some(power_pv) = site_powerflow_realtime_data.p_pv
                && let Some(power_load) = site_powerflow_realtime_data.p_load
                && let Some(power_akku) = site_powerflow_realtime_data.p_akku
            {
                let mut overproduction = if power_akku < 0.0 {
                    power_pv + power_load + power_akku
                } else {
                    power_pv + power_load
                };

                overproduction += if let Some(power_active_imported) =
                    charging_point_state.get_latest_power_active_imported()
                {
                    power_active_imported
                } else {
                    0.0
                };

                info!(
                    "Current PV overproduction {} + {} + {} + {} = {}W",
                    power_pv,
                    power_load,
                    if power_akku < 0.0 { power_akku } else { 0.0 },
                    charging_point_state
                        .get_latest_power_active_imported()
                        .unwrap_or(0.0),
                    overproduction
                );

                self.pv_overproduction.push(overproduction);
            }

            let moving_window_size =
                Duration::minutes(self.config.photo_voltaic.moving_window_size_in_minutes);

            static METER_VALUES_SAMPLE_INTERVAL_CONFIG_KEY: &str = "MeterValueSampleInterval";
            let meter_value_sample_interval = if let Some(meter_value_sample_interval) = self
                .config
                .charging_point
                .config_parameters
                .iter()
                .find(|config_parameter| {
                    config_parameter.key == METER_VALUES_SAMPLE_INTERVAL_CONFIG_KEY
                }) {
                match meter_value_sample_interval.value.parse::<i64>() {
                    Ok(meter_value_sample_interval) => meter_value_sample_interval,
                    _ => {
                        warn!(
                            "PV current can't be calculated because {} value {} could not be parsed as i64!",
                            METER_VALUES_SAMPLE_INTERVAL_CONFIG_KEY,
                            meter_value_sample_interval.value
                        );
                        return None;
                    }
                }
            } else {
                warn!(
                    "PV current can't be calculated because {} is not specified",
                    METER_VALUES_SAMPLE_INTERVAL_CONFIG_KEY
                );
                return None;
            };

            let expected_vector_size = (moving_window_size.as_seconds_f64()
                / meter_value_sample_interval as f64)
                .ceil() as usize;

            if self.pv_overproduction.len() != expected_vector_size {
                return None;
            }

            let pv_overproduction_average =
                self.pv_overproduction.iter().sum::<f64>() / self.pv_overproduction.len() as f64;
            self.pv_overproduction.remove(0);

            info!(
                "PV overproduction in the last {} minutes: {}",
                self.config.photo_voltaic.moving_window_size_in_minutes, pv_overproduction_average
            );

            if let Some(latest_cos_phi) = charging_point_state.get_latest_cos_phi()
                && let Some(latest_voltage) = charging_point_state.get_latest_voltage()
            {
                let possible_charging_current =
                    pv_overproduction_average / (latest_cos_phi * latest_voltage);

                info!(
                    "Resulting in {} A charging current",
                    possible_charging_current
                );

                return Some(possible_charging_current);
            }
        }

        None
    }
}

//-------------------------------------------------------------------------------------------------

/// Calculates the maximum possible current as P_max / (V_charging_point * cos(phi)).
/// NOTE: The calculated current will always be in the interval `[minimum_charging_current, default_current]`
///       where both values are configurable.
pub(crate) fn calculate_max_current(
    config: &config::Config,
    charging_point_state: &mut ChargePointState,
) -> Result<f64, CustomError> {
    let max_charging_power: f64 = config.charging_point.max_charging_power.into();

    let max_charging_current = (max_charging_power
        / (charging_point_state
            .get_latest_voltage()
            .unwrap_or(config.charging_point.default_system_voltage)
            * charging_point_state
                .get_latest_cos_phi()
                .unwrap_or(config.charging_point.default_cos_phi)))
    .clamp(
        config.charging_point.minimum_charging_current,
        config.charging_point.default_current,
    )
    .floor();

    info!(
        "Calculated max. charging current with {} A",
        max_charging_current
    );

    Ok(max_charging_current)
}
