use crate::ocpp_types::{CustomError, MessageTypeName};

use crate::builders::MessageBuilder;

use rust_ocpp::v1_6::messages;
use rust_ocpp::v1_6::types::MessageTrigger;

//-------------------------------------------------------------------------------------------------

pub struct TriggerMessageBuilder {
    trigger_message: MessageTrigger,
    connector_id: Option<u32>,

    message_type_name: MessageTypeName,
    trigger_message_request: Option<messages::trigger_message::TriggerMessageRequest>,
}

impl TriggerMessageBuilder {
    pub fn new(trigger_message: MessageTrigger, connector_id: Option<u32>) -> Self {
        Self {
            trigger_message,
            connector_id,
            message_type_name: MessageTypeName::TriggerMessage,
            trigger_message_request: None,
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl MessageBuilder<messages::trigger_message::TriggerMessageRequest> for TriggerMessageBuilder {
    fn get_message_type_name(&self) -> MessageTypeName {
        self.message_type_name.clone()
    }

    fn get_message_request(
        &self,
    ) -> Result<messages::trigger_message::TriggerMessageRequest, CustomError> {
        self.trigger_message_request
            .clone()
            .ok_or(CustomError::Common(
                ".build() has not been called!".to_owned(),
            ))
    }

    fn build(mut self) -> impl MessageBuilder<rust_ocpp::v1_6::messages::trigger_message::TriggerMessageRequest> {
        self.trigger_message_request = Some(messages::trigger_message::TriggerMessageRequest {
            requested_message: self.trigger_message.clone(),
            connector_id: self.connector_id,
        });

        self
    }
}
