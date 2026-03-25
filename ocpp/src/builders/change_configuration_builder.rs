use crate::ocpp_types::{CustomError, MessageTypeName};

use super::MessageBuilder;

use rust_ocpp::v1_6::messages;

//-------------------------------------------------------------------------------------------------

pub struct ChangeConfigurationBuilder {
    key: String,
    value: String,

    message_type_name: MessageTypeName,
    change_configuration_request:
        Option<messages::change_configuration::ChangeConfigurationRequest>,
}

impl ChangeConfigurationBuilder {
    pub fn new(key: String, value: String) -> Self {
        Self {
            key,
            value,
            message_type_name: MessageTypeName::ChangeConfiguration,
            change_configuration_request: None,
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl MessageBuilder<messages::change_configuration::ChangeConfigurationRequest>
    for ChangeConfigurationBuilder
{
    fn get_message_type_name(&self) -> MessageTypeName {
        self.message_type_name.clone()
    }

    fn get_message_request(
        &self,
    ) -> Result<messages::change_configuration::ChangeConfigurationRequest, CustomError> {
        self.change_configuration_request
            .clone()
            .ok_or(CustomError::Common(
                ".build() has not been called!".to_owned(),
            ))
    }

    fn build(
        mut self,
    ) -> impl MessageBuilder<messages::change_configuration::ChangeConfigurationRequest> {
        self.change_configuration_request =
            Some(messages::change_configuration::ChangeConfigurationRequest {
                key: self.key.clone(),
                value: self.value.clone(),
            });

        self
    }
}
