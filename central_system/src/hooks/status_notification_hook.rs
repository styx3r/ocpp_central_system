use chrono::{DateTime, Utc};
use log::{error, info};
use ocpp::{
    ChargePointState, ChargePointStatus, ChargingProfileKindType, ChargingProfilePurposeType,
    ChargingRateUnitType, Decimal, MessageBuilder, MessageTypeName, StatusNotificationRequest,
    charging_profile_builder::ChargingProfileBuilder,
    clear_charging_profile_builder::ClearChargingProfileBuilder,
    remote_start_transaction_builder::RemoteStartTransactionBuilder,
};
use std::{char, time::Duration};

use crate::{
    OcppHooks,
    hooks::{CONNECTOR_ID, calculate_max_current},
};

use awattar::update_price_chart;
use uuid::Uuid;

//-------------------------------------------------------------------------------------------------

static BATTERY_BLOCKING_TIME: u64 = 12;
static TX_CHARGING_PROFILE_ID: i32 = 2;

//-------------------------------------------------------------------------------------------------

impl ocpp::OcppStatusNotificationHook for OcppHooks {
    fn evaluate(
        &mut self,
        status_notification: &StatusNotificationRequest,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Evaluating OcppStatusNotificationHook {:?}",
            status_notification.status
        );

        match status_notification.status {
            ChargePointStatus::Charging => {
                self.fronius_api
                    .block_battery_for_duration(&Duration::from_hours(BATTERY_BLOCKING_TIME))?;
            }
            ChargePointStatus::Available | ChargePointStatus::SuspendedEV => {
                self.fronius_api.fully_unblock_battery()?;

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
                charge_point_state.clear_remote_start_transaction_id_tags();
            }
            ChargePointStatus::Preparing => match update_price_chart(&self.config) {
                Ok(cheapest_period) => {
                    let max_current = calculate_max_current(&self.config, charge_point_state)?;

                    if let Some(start_timestamp) =
                        DateTime::from_timestamp_millis(cheapest_period.start_timestamp)
                        && let Some(end_timestamp) =
                            DateTime::from_timestamp_millis(cheapest_period.end_timestamp)
                        && let Some(limit) = Decimal::from_f64_retain(calculate_max_current(
                            &self.config,
                            charge_point_state,
                        )?)
                    {
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
                        .add_charging_schedule_period(
                            (start_timestamp - now).num_seconds() as i32,
                            limit,
                            None,
                        )
                        .get();

                        let remote_transaction_id_tag = Uuid::new_v4().to_string();
                        let (uuid, remote_start_transaction_request) =
                            RemoteStartTransactionBuilder::new(
                                CONNECTOR_ID as u32,
                                remote_transaction_id_tag.as_str(),
                            )
                            .set_charging_profile(charging_profile)
                            .build()
                            .serialize()?;

                        charge_point_state.set_max_current(max_current);
                        charge_point_state.add_remote_transaction_id_tag(remote_transaction_id_tag);
                        charge_point_state.add_request_to_send(ocpp::RequestToSend {
                            uuid: uuid.clone(),
                            message_type: MessageTypeName::RemoteStartTransaction,
                            payload: remote_start_transaction_request,
                        });
                    }
                }
                _ => {
                    error!("Could not retrieve cheapest charging period!");
                }
            },
            _ => {}
        }

        Ok(())
    }
}
