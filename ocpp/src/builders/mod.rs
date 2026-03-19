use crate::ocpp_types::{CustomError, MessageTypeName, serialze_ocpp_request};

pub mod change_configuration_builder;
pub mod trigger_message_builder;

pub mod charging_profile_builder;
pub mod clear_charging_profile_builder;
pub mod remote_start_transaction_builder;
pub mod remote_stop_transaction_builder;
pub mod set_charging_profile_builder;

use uuid::Uuid;

//-------------------------------------------------------------------------------------------------

pub trait MessageBuilder<U: serde::Serialize> {
    fn build(self) -> impl MessageBuilder<U>;
    fn get_message_type_name(&self) -> MessageTypeName;
    fn get_message_request(&self) -> Result<U, CustomError>;

    fn serialize(self) -> Result<(String, String), CustomError>
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
