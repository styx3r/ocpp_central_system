use log::info;
use ocpp::{ChargePointStatus, StatusNotificationRequest};
use std::time::Duration;

use crate::OcppHooks;

//-------------------------------------------------------------------------------------------------

impl ocpp::OcppStatusNotificationHook for OcppHooks {
    fn evaluate(
        &mut self,
        status_notification: &StatusNotificationRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Evaluating OcppStatusNotificationHook {:?}",
            status_notification.status
        );
        match status_notification.status {
            ChargePointStatus::Charging => {
                self.fronius_api
                    .block_battery_for_duration(&Duration::from_hours(12))?;
            }
            ChargePointStatus::SuspendedEV => {
                self.fronius_api.fully_unblock_battery()?;
            }
            _ => {}
        }

        Ok(())
    }
}
