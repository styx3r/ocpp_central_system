mod meter_values_hooks;
mod status_notification_hook;

use config::config::ChargePoint;
use fronius::FroniusApi;

pub struct OcppHooks {
    fronius_api: FroniusApi,
    charge_point_config: ChargePoint,
}

impl OcppHooks {
    pub fn new(fronius_api: FroniusApi, charge_point_config: ChargePoint) -> Self {
        Self {
            fronius_api,
            charge_point_config,
        }
    }
}
