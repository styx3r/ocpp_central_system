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
            } else {
                info!(
                    "No special action for state transition from {:?} to {:?}",
                    charge_point_status.as_ref().unwrap(),
                    status_notification.status
                );
            }

            charge_point_state.set_charge_point_status(status_notification.status.clone());
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use awattar::awattar_mock::AwattarApiMock;
    use chrono::Utc;
    use config::config;
    use fronius::FroniusMock;
    use ocpp::OcppStatusNotificationHook;
    use rust_ocpp::v1_6::{
        messages::{
            clear_charging_profile::ClearChargingProfileRequest,
            status_notification::StatusNotificationRequest,
        },
        types::ChargePointErrorCode,
    };
    use serde::de::DeserializeOwned;

    fn parse_message_request<T: DeserializeOwned>(payload: &str) -> T {
        let message_request =
            serde_json::from_str::<(u32, String, String, serde_json::Value)>(payload)
                .expect("Could not deserialize request");

        serde_json::from_value::<T>(message_request.3).expect("Could not deserialize payload")
    }

    fn test_config() -> config::Config {
        config::Config {
            websocket: config::Websocket {
                ip: "127.0.0.1".to_owned(),
                port: 8080,
            },
            charging_point: config::ChargePoint {
                serial_number: "".to_owned(),
                heartbeat_interval: 30,
                max_charging_power: 11000.0,
                default_system_voltage: 696.0,
                default_current: 16.0,
                default_cos_phi: 1.0,
                minimum_charging_current: 6.0,
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

    fn charge_point_state_with_dummy_charge_profile(status: ChargePointStatus) -> ChargePointState {
        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(status);
        charge_point_state.add_charging_profile(&ocpp::ChargingProfile {
            charging_profile_id: 1,
            transaction_id: None,
            stack_level: 0,
            charging_profile_purpose: ChargingProfilePurposeType::TxProfile,
            charging_profile_kind: ocpp::ChargingProfileKindType::Relative,
            recurrency_kind: None,
            valid_from: None,
            valid_to: None,
            charging_schedule: rust_ocpp::v1_6::types::ChargingSchedule {
                duration: None,
                start_schedule: None,
                charging_rate_unit: ocpp::ChargingRateUnitType::A,
                charging_schedule_period: vec![],
                min_charging_rate: None,
            },
        });

        charge_point_state
    }

    #[test]
    fn initial_state_notification() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Available,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Available)
        );

        Ok(())
    }

    #[test]
    fn available_to_preparing() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(ChargePointStatus::Available);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Preparing,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .unblock_battery_called
        );

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Preparing)
        );

        Ok(())
    }
    #[test]
    fn available_to_charging() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(ChargePointStatus::Available);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Charging,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Charging)
        );

        Ok(())
    }

    #[test]
    fn preparing_to_available() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::Preparing);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Available,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Available)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );

        Ok(())
    }

    #[test]
    fn preparing_to_charging() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(ChargePointStatus::Preparing);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Charging,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(!fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Charging)
        );

        Ok(())
    }

    #[test]
    fn preparing_to_finishing() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::Preparing);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Finishing,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Finishing)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }

    #[test]
    fn charging_to_available() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::Charging);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Available,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Available)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }

    #[test]
    fn charging_to_suspended_ev() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(ChargePointStatus::Charging);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::SuspendedEV,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::SuspendedEV)
        );

        Ok(())
    }

    #[test]
    fn charging_to_suspended_evse() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(ChargePointStatus::Charging);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::SuspendedEVSE,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::SuspendedEVSE)
        );

        Ok(())
    }

    #[test]
    fn charging_to_finishing() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::Charging);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Finishing,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Finishing)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }

    #[test]
    fn suspended_ev_to_available() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::SuspendedEV);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Available,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Available)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }

    #[test]
    fn suspended_ev_to_charging() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(ChargePointStatus::SuspendedEV);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Charging,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(!fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Charging)
        );

        Ok(())
    }

    #[test]
    fn suspended_ev_to_finishing() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::SuspendedEV);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Finishing,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Finishing)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }

    #[test]
    fn suspended_evse_to_available() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::SuspendedEVSE);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Available,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Available)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }

    #[test]
    fn suspended_evse_to_charging() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.set_charge_point_status(ChargePointStatus::SuspendedEVSE);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Charging,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(!fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Charging)
        );

        Ok(())
    }

    #[test]
    fn suspended_evse_to_finishing() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::SuspendedEVSE);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Finishing,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Finishing)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }

    #[test]
    fn finishing_to_available() -> Result<(), Box<dyn std::error::Error>> {
        let fronius_mock = Arc::new(Mutex::new(FroniusMock::default()));
        let hook: Arc<Mutex<dyn OcppStatusNotificationHook>> =
            Arc::new(Mutex::new(OcppHooks::new(
                Arc::clone(&fronius_mock),
                Arc::new(Mutex::new(AwattarApiMock::default())),
                test_config(),
            )));

        let mut charge_point_state =
            charge_point_state_with_dummy_charge_profile(ChargePointStatus::Finishing);

        let status_notification_charging = StatusNotificationRequest {
            connector_id: CONNECTOR_ID as u32,
            error_code: ChargePointErrorCode::NoError,
            info: None,
            status: ChargePointStatus::Available,
            timestamp: Some(Utc::now()),
            vendor_id: None,
            vendor_error_code: None,
        };

        hook.lock()
            .unwrap()
            .evaluate(&status_notification_charging, &mut charge_point_state)?;

        assert!(
            !fronius_mock
                .lock()
                .unwrap()
                .block_battery_for_duration_called
        );

        assert!(fronius_mock.lock().unwrap().unblock_battery_called);

        assert_eq!(
            charge_point_state.get_charge_point_status(),
            &Some(ChargePointStatus::Available)
        );

        let requests_to_send = charge_point_state.get_requests_to_send();
        assert_eq!(requests_to_send.len(), 1);

        let clear_charging_profile_request = requests_to_send.first().unwrap();
        assert_eq!(
            clear_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_charging_profile_request.payload.as_str(),
        );
        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: Some(1),
                connector_id: Some(CONNECTOR_ID),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: Some(0)
            }
        );
        Ok(())
    }
}
