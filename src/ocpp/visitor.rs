use super::ocpp_types::{OcppRequestMessage, OcppResponseMessage};

pub trait Visitor<T> {
    fn visit_request_message(&mut self, request: OcppRequestMessage) -> T;
    fn visit_response_message(&mut self, request: OcppResponseMessage) -> T;
}
