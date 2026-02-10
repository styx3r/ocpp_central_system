use awattar::AwattarApi;
use fronius::FroniusApi;
use log::info;
use ocpp::{
    ChargePointState, ChargePointStatus, ChargingProfilePurposeType, MessageBuilder,
    MessageTypeName, StatusNotificationRequest,
    clear_charging_profile_builder::ClearChargingProfileBuilder,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{OcppHooks, hooks::CONNECTOR_ID};

//-------------------------------------------------------------------------------------------------

static BATTERY_BLOCKING_TIME_IN_HOURS: u64 = 12;

//-------------------------------------------------------------------------------------------------

fn clear_tx_charging_profiles(
    charge_point_state: &mut ChargePointState,
    charging_profile_id: i32,
    connector_id: i32,
    charging_profile_purpose_type: ChargingProfilePurposeType,
    stack_level: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (uuid, clear_charging_profile) = ClearChargingProfileBuilder::new(
        Some(charging_profile_id),
        Some(connector_id),
        Some(charging_profile_purpose_type),
        Some(stack_level as i32),
    )
    .build()
    .serialize()?;

    charge_point_state.add_request_to_send(ocpp::RequestToSend {
        uuid: uuid.clone(),
        message_type: MessageTypeName::ClearChargingProfile,
        payload: clear_charging_profile,
    });

    Ok(())
}

//-------------------------------------------------------------------------------------------------

fn unblock_battery_and_clear_tx_profiles<T: FroniusApi>(
    charge_point_state: &mut ChargePointState,
    fronius_api: Arc<Mutex<T>>,
) -> Result<(), Box<dyn std::error::Error>> {
    fronius_api.lock().unwrap().fully_unblock_battery()?;
    for charging_profile in charge_point_state.get_active_charging_profiles().clone() {
        clear_tx_charging_profiles(
            charge_point_state,
            charging_profile.charging_profile_id,
            CONNECTOR_ID,
            charging_profile.charging_profile_purpose,
            charging_profile.stack_level,
        )?
    }

    charge_point_state.disable_smart_charging();

    Ok(())
}

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

        let charge_point_status = charge_point_state.get_charge_point_status();
        if charge_point_status.is_none() {
            info!("Setting initial ChargePointStatus");
            charge_point_state.set_charge_point_status(status_notification.status.clone());
            return Ok(());
        }

        let block_battery = Box::new(
            |_: &mut ChargePointState,
             fronius_api: Arc<Mutex<T>>|
             -> Result<(), Box<dyn std::error::Error>> {
                fronius_api
                    .lock()
                    .unwrap()
                    .block_battery_for_duration(&Duration::from_hours(
                        BATTERY_BLOCKING_TIME_IN_HOURS,
                    ))
            },
        );

        let unblock_battery = Box::new(
            |_: &mut ChargePointState,
             fronius_api: Arc<Mutex<T>>|
             -> Result<(), Box<dyn std::error::Error>> {
                fronius_api.lock().unwrap().fully_unblock_battery()
            },
        );

        let mut state_transitions: Vec<(
            ChargePointStatus,
            Vec<(
                ChargePointStatus,
                Box<
                    dyn FnMut(
                        &mut ChargePointState,
                        Arc<Mutex<T>>,
                    ) -> Result<(), Box<dyn std::error::Error>>,
                >,
            )>,
        )> = vec![
            (
                ChargePointStatus::Available,
                vec![(ChargePointStatus::Charging, block_battery.clone())],
            ),
            (
                ChargePointStatus::Preparing,
                vec![
                    (
                        ChargePointStatus::Available,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                    (ChargePointStatus::Charging, block_battery.clone()),
                    (
                        ChargePointStatus::Finishing,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                ],
            ),
            (
                ChargePointStatus::Charging,
                vec![
                    (
                        ChargePointStatus::Available,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                    (ChargePointStatus::SuspendedEV, unblock_battery.clone()),
                    (ChargePointStatus::SuspendedEVSE, unblock_battery.clone()),
                    (
                        ChargePointStatus::Finishing,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                ],
            ),
            (
                ChargePointStatus::SuspendedEV,
                vec![
                    (
                        ChargePointStatus::Available,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                    (ChargePointStatus::Charging, block_battery.clone()),
                    (
                        ChargePointStatus::Finishing,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                ],
            ),
            (
                ChargePointStatus::SuspendedEVSE,
                vec![
                    (
                        ChargePointStatus::Available,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                    (ChargePointStatus::Charging, block_battery.clone()),
                    (
                        ChargePointStatus::Finishing,
                        Box::new(unblock_battery_and_clear_tx_profiles),
                    ),
                ],
            ),
            (
                ChargePointStatus::Finishing,
                vec![(
                    ChargePointStatus::Available,
                    Box::new(unblock_battery_and_clear_tx_profiles),
                )],
            ),
        ];

        if let Some((_, possible_next_states)) = state_transitions
            .iter_mut()
            .find(|(current_state, _)| *current_state == charge_point_status.clone().unwrap())
        {
            if let Some((_, next_state_action)) = possible_next_states
                .iter_mut()
                .find(|(next_state, _)| *next_state == status_notification.status)
            {
                next_state_action(charge_point_state, Arc::clone(&self.fronius_api))?;
                charge_point_state.set_charge_point_status(status_notification.status.clone());
            } else {
                info!(
                    "No special action for state transition from {:?} to {:?}",
                    charge_point_status.as_ref().unwrap(),
                    status_notification.status
                );
                return Ok(());
            }
        }

        Ok(())
    }
}
