use crate::ocpp_types::{CustomError, MessageTypeName};

use crate::builders::MessageBuilder;

use rust_ocpp::v1_6::messages;

//-------------------------------------------------------------------------------------------------

pub struct RemoteStopTransactionBuilder {
    message_type_name: MessageTypeName,
    remote_stop_transaction_request:
        Option<messages::remote_stop_transaction::RemoteStopTransactionRequest>,
}

impl RemoteStopTransactionBuilder {
    pub fn new(transaction_id: i32) -> Self {
        Self {
            message_type_name: MessageTypeName::RemoteStopTransaction,
            remote_stop_transaction_request: Some(
                messages::remote_stop_transaction::RemoteStopTransactionRequest {
                    transaction_id: transaction_id,
                },
            ),
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl MessageBuilder<messages::remote_stop_transaction::RemoteStopTransactionRequest>
    for RemoteStopTransactionBuilder
{
    fn get_message_type_name(&self) -> MessageTypeName {
        self.message_type_name.to_owned()
    }

    fn get_message_request(
        &self,
    ) -> Result<messages::remote_stop_transaction::RemoteStopTransactionRequest, CustomError> {
        self.remote_stop_transaction_request
            .clone()
            .ok_or(CustomError::Common(
                ".build() has not been called!".to_owned(),
            ))
    }

    fn build(
        self,
    ) -> impl MessageBuilder<messages::remote_stop_transaction::RemoteStopTransactionRequest> {
        self
    }
}
