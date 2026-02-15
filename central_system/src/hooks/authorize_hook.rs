use crate::OcppHooks;
use awattar::AwattarApi;
use config::config::SmartChargingMode;
use fronius::FroniusApi;
use log::info;

use ocpp::{
    AuthorizeRequest, ChargePointState, ChargingProfileKindType, ChargingProfilePurposeType,
    ChargingRateUnitType, CustomError, Decimal, MessageBuilder, MessageTypeName, RequestToSend,
    charging_profile_builder::ChargingProfileBuilder,
    clear_charging_profile_builder::ClearChargingProfileBuilder,
    set_charging_profile_builder::SetChargingProfileBuilder,
};

use chrono::Utc;

use crate::hooks::{CONNECTOR_ID, TX_PV_PREPARATION_CHARGING_PROFILE_ID};

//-------------------------------------------------------------------------------------------------

fn clear_tx_charging_profiles(
    charge_point_state: &mut ChargePointState,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Clearing TxChargingProfiles!");
    let (uuid, clear_tx_charging_profile) = ClearChargingProfileBuilder::new(
        None,
        Some(CONNECTOR_ID),
        Some(ChargingProfilePurposeType::TxProfile),
        None,
    )
    .build()
    .serialize()?;

    charge_point_state.add_request_to_send(ocpp::RequestToSend {
        uuid: uuid.clone(),
        message_type: MessageTypeName::ClearChargingProfile,
        payload: clear_tx_charging_profile,
    });

    charge_point_state.disable_smart_charging();

    Ok(())
}

//-------------------------------------------------------------------------------------------------

impl<T: FroniusApi, U: AwattarApi> ocpp::OcppAuthorizationHook for OcppHooks<T, U> {
    fn evaluate(
        &mut self,
        authorization_request: &AuthorizeRequest,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id_tag = self
            .config
            .id_tags
            .iter()
            .find(|id_tag| id_tag.id == authorization_request.id_tag)
            .ok_or(CustomError::Common(
                "Given IdTag is not configured!".to_owned(),
            ))?;

        if !charge_point_state.get_running_transaction_ids().is_empty() {
            clear_tx_charging_profiles(charge_point_state)?;
        }

        match id_tag.smart_charging_mode {
            SmartChargingMode::Instant => {}
            SmartChargingMode::PVOverProductionAndGridBased => {
                let max_charging_current =
                    self.get_updated_max_charging_current(charge_point_state);

                if max_charging_current.is_none() {
                    return Ok(());
                }

                self.calculate_grid_based_smart_charging_tx_profile(
                    charge_point_state,
                    max_charging_current.unwrap(),
                )?;
            }
            SmartChargingMode::PVOverProduction => {
                let start_timestamp = Utc::now();
                let pv_over_production_profile = ChargingProfileBuilder::new(
                    TX_PV_PREPARATION_CHARGING_PROFILE_ID,
                    ChargingProfilePurposeType::TxProfile,
                    ChargingProfileKindType::Absolute,
                    ChargingRateUnitType::A,
                )
                .set_valid_from(start_timestamp)
                .set_start_schedule_timestamp(start_timestamp)
                .add_charging_schedule_period(0, Decimal::new(0, 0), None)
                .get();

                let (uuid, payload) =
                    SetChargingProfileBuilder::new(CONNECTOR_ID, pv_over_production_profile)
                        .build()
                        .serialize()?;

                charge_point_state.add_request_to_send(RequestToSend {
                    uuid: uuid.clone(),
                    message_type: MessageTypeName::SetChargingProfile,
                    payload,
                });
            }
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use ::config::config::IdTag;
    use awattar::awattar_mock::AwattarApiMock;
    use chrono::Utc;
    use config::config;
    use fronius::{
        Data, FroniusMock, PowerFlowRealtimeData, PowerFlowRealtimeDataBody,
        PowerFlowRealtimeDataHeader, Site, Smartloads, Status,
    };
    use ocpp::{ChargingProfile, OcppAuthorizationHook, OcppMeterValuesHook};
    use serde::de::DeserializeOwned;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use rust_ocpp::v1_6::messages::{
        authorize::AuthorizeRequest, clear_charging_profile::ClearChargingProfileRequest,
    };

    use super::*;

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

    #[test]
    fn instant_charging_mode() -> Result<(), Box<dyn std::error::Error>> {
        let mut test_config = test_config();
        test_config.id_tags = vec![IdTag {
            id: "UNITTEST".to_string(),
            smart_charging_mode: SmartChargingMode::Instant,
        }];

        let hook: Arc<Mutex<dyn OcppAuthorizationHook>> = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            test_config,
        )));

        let mut charge_point_state = ChargePointState::default();
        let authorize_request = AuthorizeRequest {
            id_tag: "UNITTEST".to_string(),
        };
        hook.lock()
            .unwrap()
            .evaluate(&authorize_request, &mut charge_point_state)?;

        assert!(charge_point_state.get_requests_to_send().is_empty());
        Ok(())
    }

    #[test]
    fn instant_charging_mode_with_running_transaction() -> Result<(), Box<dyn std::error::Error>> {
        let mut test_config = test_config();
        test_config.id_tags = vec![IdTag {
            id: "UNITTEST".to_string(),
            smart_charging_mode: SmartChargingMode::Instant,
        }];

        let hook: Arc<Mutex<dyn OcppAuthorizationHook>> = Arc::new(Mutex::new(OcppHooks::new(
            Arc::new(Mutex::new(FroniusMock::default())),
            Arc::new(Mutex::new(AwattarApiMock::default())),
            test_config,
        )));

        let mut charge_point_state = ChargePointState::default();
        charge_point_state.add_running_transaction_id(ocpp::Transaction {
            id_tag: Some("UNITTEST".to_string()),
            transaction_id: 1,
            meter_value_start: 0,
            meter_value_stop: 0,
        });

        let authorize_request = AuthorizeRequest {
            id_tag: "UNITTEST".to_string(),
        };
        hook.lock()
            .unwrap()
            .evaluate(&authorize_request, &mut charge_point_state)?;

        let mut handle = charge_point_state.get_requests_to_send().iter();
        let clear_tx_charging_profile_request = handle.next().unwrap();
        assert_eq!(
            clear_tx_charging_profile_request.message_type,
            MessageTypeName::ClearChargingProfile
        );

        let clear_charging_profile = parse_message_request::<ClearChargingProfileRequest>(
            clear_tx_charging_profile_request.payload.as_str(),
        );

        assert_eq!(
            clear_charging_profile,
            ClearChargingProfileRequest {
                id: None,
                connector_id: Some(1),
                charging_profile_purpose: Some(ChargingProfilePurposeType::TxProfile),
                stack_level: None
            }
        );

        Ok(())
    }
}
