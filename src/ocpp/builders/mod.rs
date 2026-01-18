use crate::ocpp::ocpp_types::{CustomError, MessageTypeName, serialze_ocpp_request};

pub(crate) mod change_configuration_builder;
pub(crate) mod clear_charging_profile_builder;
pub(crate) mod set_charging_profile_builder;
pub(crate) mod trigger_message_builder;

use uuid::Uuid;

//-------------------------------------------------------------------------------------------------

pub trait MessageBuilder<U: serde::Serialize> {
    fn build(&mut self) -> &mut Self;
    fn get_message_type_name(&self) -> MessageTypeName;
    fn get_message_request(&self) -> Result<U, CustomError>;

    fn serialize(&self) -> Result<(String, String), CustomError>
    where
        Self: Sized,
    {
        let uuid = Uuid::new_v4().to_string();
        Ok((
            uuid.clone(),
            serialze_ocpp_request(
                self.get_message_type_name(),
                uuid,
                self.get_message_request()?,
            )?,
        ))
    }
}
