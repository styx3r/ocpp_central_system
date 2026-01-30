use crate::OcppHooks;
use log::info;

use config::config;
use ocpp::{
    AuthorizeRequest, ChargePointState, ChargingProfilePurposeType, MessageBuilder,
    MessageTypeName, clear_charging_profile_builder::ClearChargingProfileBuilder,
    remote_stop_transaction_builder::RemoteStopTransactionBuilder,
};

use crate::hooks::CONNECTOR_ID;

//-------------------------------------------------------------------------------------------------

impl ocpp::OcppAuthorizationHook for OcppHooks {
    fn evaluate(
        &mut self,
        authorization_request: &AuthorizeRequest,
        charge_point_state: &mut ChargePointState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let remote_start_transaction_id_tag = charge_point_state
            .get_remote_start_transaction_id_tags()
            .iter()
            .any(|e| *e == authorization_request.id_tag);

        if !remote_start_transaction_id_tag {
            return Ok(());
        }

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

        for running_transaction in charge_point_state.clone().get_running_transaction_ids() {
            info!("Stoping transaction with ID {}", running_transaction.transaction_id);

            let (uuid, remote_stop_transaction) =
                RemoteStopTransactionBuilder::new(running_transaction.transaction_id)
                    .build()
                    .serialize()?;

            charge_point_state.add_request_to_send(ocpp::RequestToSend {
                uuid: uuid.clone(),
                message_type: MessageTypeName::RemoteStopTransaction,
                payload: remote_stop_transaction,
            });
        }

        charge_point_state.clear_remote_start_transaction_id_tags();
        Ok(())
    }
}
