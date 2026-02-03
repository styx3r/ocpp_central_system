mod authorize_hook;
mod meter_values_hooks;
mod status_notification_hook;

use chrono::{DateTime, Utc};
use config::config;
use fronius::FroniusApi;

use log::info;
use ocpp::{
    ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    CustomError, Decimal, MessageBuilder, MessageTypeName,
    charging_profile_builder::ChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
};

use awattar::AwattarApi;

use std::sync::{Arc, Mutex};

pub struct OcppHooks<T: FroniusApi, U: AwattarApi> {
    fronius_api: Arc<Mutex<T>>,
    awattar_api: Arc<Mutex<U>>,
    config: config::Config,
}

impl<T: FroniusApi, U: AwattarApi> OcppHooks<T, U> {
    pub fn new(
        fronius_api: Arc<Mutex<T>>,
        awattar_api: Arc<Mutex<U>>,
        config: config::Config,
    ) -> Self {
        Self {
            fronius_api,
            awattar_api,
            config,
        }
    }

    pub fn calculate_grid_based_smart_charging_tx_profile(
        &self,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let max_current = calculate_max_current(&self.config, charge_point_state)?;
        let limit = Decimal::from_f64_retain(max_current).ok_or(CustomError::Common(
            "Could not convert max_current to Decimal!".to_owned(),
        ))?;

        let grid_based_charging_profile = if let Some(grid_based_smart_charging_profile) =
            charge_point_state.get_grid_based_smart_charging_profile()
        {
            let mut grid_based_smart_charging_profile_handle = grid_based_smart_charging_profile.clone();
            grid_based_smart_charging_profile_handle
                .charging_schedule
                .charging_schedule_period[1] // NOTE: This RELIES on the fact that the second
                                             // charging schedule period is the cheapest_period
                .limit = limit;

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
                TX_CHARGING_PROFILE_ID,
                ChargingProfilePurposeType::TxProfile,
                ChargingProfileKindType::Absolute,
                ChargingRateUnitType::A,
            )
            .set_valid_from(now)
            .set_valid_to(end_timestamp)
            .set_charging_schedule_duration((end_timestamp - now).num_seconds() as i32)
            .set_start_schedule_timestamp(now)
            .add_charging_schedule_period(0, Decimal::new(0, 0), None)
            .add_charging_schedule_period((start_timestamp - now).num_seconds() as i32, limit, None)
            .get();

            charging_profile
        };

        charge_point_state.set_grid_based_smart_charging_profile(&grid_based_charging_profile);
        charge_point_state.set_max_current(max_current);
        charge_point_state.enable_smart_charging();

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
}

//-------------------------------------------------------------------------------------------------

static CONNECTOR_ID: i32 = 1;
static TX_CHARGING_PROFILE_ID: i32 = 2;

//-------------------------------------------------------------------------------------------------

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
