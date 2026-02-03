use crate::OcppHooks;
use awattar::AwattarApi;
use fronius::FroniusApi;
use log::info;

use ocpp::{
    AuthorizeRequest, ChargePointState, ChargingProfilePurposeType, MessageBuilder,
    MessageTypeName, clear_charging_profile_builder::ClearChargingProfileBuilder,
};

use crate::hooks::CONNECTOR_ID;

//-------------------------------------------------------------------------------------------------

fn clear_smart_charging_tx_charging_profile(
    charge_point_state: &mut ChargePointState,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Clearing TxChargingProfiles!");
    let (uuid, clear_tx_charging_profile) = ClearChargingProfileBuilder::new(
        None,
        Some(CONNECTOR_ID),
        Some(ChargingProfilePurposeType::TxProfile),
        Some(0),
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
            .find(|id_tag| id_tag.id == authorization_request.id_tag);

        if id_tag.is_none() || !id_tag.unwrap().smart_charging {
            return Ok(());
        }

        if !charge_point_state.get_running_transaction_ids().is_empty() {
            clear_smart_charging_tx_charging_profile(charge_point_state)?;
        } else {
            self.calculate_grid_based_smart_charging_tx_profile(charge_point_state)?;
        }

        Ok(())
    }
}
