use awattar::AwattarApi;
use fronius::FroniusApi;
use log::info;
use ocpp::{
    ChargePointState, ChargePointStatus, ChargingProfilePurposeType,
    MessageBuilder, MessageTypeName, StatusNotificationRequest,
    clear_charging_profile_builder::ClearChargingProfileBuilder,
};
use std::time::Duration;

use crate::{
    OcppHooks,
    hooks::{CONNECTOR_ID, TX_CHARGING_PROFILE_ID},
};

//-------------------------------------------------------------------------------------------------

static BATTERY_BLOCKING_TIME: u64 = 12;

//-------------------------------------------------------------------------------------------------

impl<T: FroniusApi, U: AwattarApi> ocpp::OcppStatusNotificationHook for OcppHooks<T, U> {
    fn evaluate(
        &mut self,
        status_notification: &StatusNotificationRequest,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Evaluating OcppStatusNotificationHook {:?}",
            status_notification.status
        );

        let handle = self.fronius_api.clone();
        let mut fronius_api = handle.lock().unwrap();

        match status_notification.status {
            ChargePointStatus::Charging => {
                fronius_api
                    .block_battery_for_duration(&Duration::from_hours(BATTERY_BLOCKING_TIME))?;
            }
            ChargePointStatus::Available | ChargePointStatus::SuspendedEV => {
                fronius_api.fully_unblock_battery()?;
                if charge_point_state.get_smart_charging() {
                    let (uuid, clear_charging_profile) = ClearChargingProfileBuilder::new(
                        Some(TX_CHARGING_PROFILE_ID),
                        Some(CONNECTOR_ID),
                        Some(ChargingProfilePurposeType::TxProfile),
                        Some(0),
                    )
                    .build()
                    .serialize()?;

                    charge_point_state.add_request_to_send(ocpp::RequestToSend {
                        uuid: uuid.clone(),
                        message_type: MessageTypeName::ClearChargingProfile,
                        payload: clear_charging_profile,
                    });
                    charge_point_state.disable_smart_charging();
                }
            }
            _ => {}
        }

        Ok(())
    }
}
