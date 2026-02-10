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
