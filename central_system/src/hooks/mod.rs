mod authorize_hook;
mod meter_values_hooks;
mod status_notification_hook;

use config::config;
use fronius::FroniusApi;

use log::info;
use ocpp::{ChargePointState, CustomError};

pub struct OcppHooks {
    fronius_api: FroniusApi,
    config: config::Config,
}

impl OcppHooks {
    pub fn new(fronius_api: FroniusApi, config: config::Config) -> Self {
        Self {
            fronius_api,
            config,
        }
    }
}

//-------------------------------------------------------------------------------------------------

static CONNECTOR_ID: i32 = 1;

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
