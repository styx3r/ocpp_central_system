use crate::ocpp_types::{CustomError, MessageTypeName};

use crate::builders::MessageBuilder;

use rust_ocpp::v1_6::messages;
use rust_ocpp::v1_6::types::ChargingProfile;

//-------------------------------------------------------------------------------------------------

pub struct RemoteStartTransactionBuilder {
    connector_id: u32,
    id_tag: String,
    charging_profile: Option<ChargingProfile>,

    message_type_name: MessageTypeName,
    remote_start_transaction_request:
        Option<messages::remote_start_transaction::RemoteStartTransactionRequest>,
}

impl RemoteStartTransactionBuilder {
    pub fn new(connector_id: u32, id_tag: &str) -> Self {
        Self {
            connector_id,
            id_tag: id_tag.to_owned(),
            charging_profile: None,
            message_type_name: MessageTypeName::RemoteStartTransaction,
            remote_start_transaction_request: None,
        }
    }

    pub fn set_charging_profile(
        mut self,
        charging_profile: ChargingProfile,
    ) -> RemoteStartTransactionBuilder {
        self.charging_profile = Some(charging_profile);
        self
    }
}

//-------------------------------------------------------------------------------------------------

impl MessageBuilder<messages::remote_start_transaction::RemoteStartTransactionRequest>
    for RemoteStartTransactionBuilder
{
    fn get_message_type_name(&self) -> MessageTypeName {
        self.message_type_name.to_owned()
    }

    fn get_message_request(
        &self,
    ) -> Result<messages::remote_start_transaction::RemoteStartTransactionRequest, CustomError>
    {
        self.remote_start_transaction_request
            .clone()
            .ok_or(CustomError::Common(
                ".build() has not been called!".to_owned(),
            ))
    }

    fn build(
        mut self,
    ) -> impl MessageBuilder<messages::remote_start_transaction::RemoteStartTransactionRequest>
    {
        self.remote_start_transaction_request = Some(
            messages::remote_start_transaction::RemoteStartTransactionRequest {
                connector_id: Some(self.connector_id),
                id_tag: self.id_tag.clone(),
                charging_profile: self.charging_profile.clone(),
            },
        );
        self
    }
}
