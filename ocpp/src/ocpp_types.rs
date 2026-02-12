use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use serde::{Deserialize, Serialize};
use serde_repr::*;

use log::debug;

//-------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub enum CustomError {
    Serde(serde_json::Error),
    Common(String),
    Sql(rusqlite::Error),
}

impl From<serde_json::Error> for CustomError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serde(error)
    }
}

impl From<rusqlite::Error> for CustomError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl Display for CustomError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serde(e) => Display::fmt(e, f),
            Self::Common(e) => Display::fmt(e, f),
            Self::Sql(e) => Display::fmt(e, f),
        }
    }
}

impl std::error::Error for CustomError {}

//-------------------------------------------------------------------------------------------------

#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Clone)]
#[repr(u32)]
pub enum MessageType {
    Request = 2,
    Response = 3,
    Error = 4,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum MessageTypeName {
    // Initiated by ChargePoint
    Authorize,
    BootNotification,
    DataTransfer,
    DiagnosticsStatusNotification,
    FirmwareStatusNotification,
    Heartbeat,
    MeterValues,
    StartTransaction,
    StatusNotification,
    StopTransaction,
    LogStatusNotification,
    SecurityEventNotification,
    SignedFirmwareStatusNotification,

    // Initiated by CentralSystem
    RemoteStartTransaction,
    RemoteStopTransaction,
    TriggerMessage,
    SetChargingProfile,
    GetDiagnostics,
    ChangeConfiguration,
    ClearChargingProfile,
}

impl fmt::Display for MessageTypeName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Initiated by ChargePoint
            MessageTypeName::Authorize => write!(f, "Authorize"),
            MessageTypeName::BootNotification => write!(f, "BootNotification"),
            MessageTypeName::DataTransfer => write!(f, "DataTransfer"),
            MessageTypeName::DiagnosticsStatusNotification => {
                write!(f, "DiagnosticsStatusNotification")
            }
            MessageTypeName::FirmwareStatusNotification => write!(f, "FirmwareStatusNotification"),
            MessageTypeName::Heartbeat => write!(f, "Heartbeat"),
            MessageTypeName::MeterValues => write!(f, "MeterValues"),
            MessageTypeName::StartTransaction => write!(f, "StartTransaction"),
            MessageTypeName::StatusNotification => write!(f, "StatusNotification"),
            MessageTypeName::StopTransaction => write!(f, "StopTransaction"),
            MessageTypeName::LogStatusNotification => write!(f, "LogStatusNotification"),
            MessageTypeName::SecurityEventNotification => write!(f, "SecurityEventNotification"),
            MessageTypeName::SignedFirmwareStatusNotification => {
                write!(f, "SignedFirmwareStatusNotification")
            }

            // Initiated by CentralSystem
            MessageTypeName::RemoteStartTransaction => write!(f, "RemoteStartTransaction"),
            MessageTypeName::RemoteStopTransaction => write!(f, "RemoteStopTransaction"),
            MessageTypeName::TriggerMessage => write!(f, "TriggerMessage"),
            MessageTypeName::SetChargingProfile => write!(f, "SetChargingProfile"),
            MessageTypeName::GetDiagnostics => write!(f, "GetDiagnostics"),
            MessageTypeName::ChangeConfiguration => write!(f, "ChangeConfiguration"),
            MessageTypeName::ClearChargingProfile => write!(f, "ClearChargingProfile"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OcppRequestMessage {
    pub id: MessageType,
    pub uuid: String,
    pub message_type: MessageTypeName,
    pub json: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OcppResponseMessage {
    pub id: MessageType,
    pub uuid: String,
    pub json: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OcppErrorMessage {
    pub id: MessageType,
    pub uuid: String,
    pub error_code: String,
    pub error_description: String,
    pub json: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum OcppMessage {
    Request(OcppRequestMessage),
    Response(OcppResponseMessage),
    Error(OcppErrorMessage),
}

//-------------------------------------------------------------------------------------------------

pub fn parse_ocpp_message(msg: &str) -> Result<OcppMessage, serde_json::Error> {
    debug!("Trying to parse {}", msg);
    Ok(serde_json::from_str::<OcppMessage>(msg)?)
}

//-------------------------------------------------------------------------------------------------

pub fn serialze_ocpp_response(
    uuid: &String,
    response: &serde_json::Value,
) -> Result<String, CustomError> {
    let ocpp_response_message = OcppResponseMessage {
        id: MessageType::Response,
        uuid: uuid.clone(),
        json: response.clone(),
    };

    let response = format!(
        "[{},\"{}\",{}]",
        serde_json::to_string(&ocpp_response_message.id)?,
        ocpp_response_message.uuid,
        serde_json::to_string(&ocpp_response_message.json)?
    );

    Ok(response)
}

//-------------------------------------------------------------------------------------------------

pub fn serialze_ocpp_request<T>(
    message_type: MessageTypeName,
    uuid: String,
    request: T,
) -> Result<String, CustomError>
where
    T: Serialize,
{
    let ocpp_request_message = OcppRequestMessage {
        id: MessageType::Request,
        message_type,
        uuid,
        json: serde_json::to_value(&request)?,
    };

    let request = format!(
        "[{},\"{}\",\"{}\",{}]",
        serde_json::to_string(&ocpp_request_message.id)?,
        ocpp_request_message.uuid,
        ocpp_request_message.message_type,
        serde_json::to_string(&ocpp_request_message.json)?
    );

    Ok(request)
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn parse_ocpp_request_message() -> Result<(), CustomError> {
        /*
         * [
         *   2,
         *   "1367122111",
         *   "BootNotification",
         *   {
         *       "chargePointModel": "EVlink Pro AC",
         *       "chargePointVendor": "Schneider Electric",
         *       "chargePointSerialNumber": "SERIAL_NUMBER",
         *       "firmwareVersion": "1.6.1",
         *       "meterType": "Embedded MID meter",
         *       "meterSerialNumber": "SERIAL_NUMBER"
         *   }
         * ]
         */

        let ocpp_message = r#"
[2,"1367122111","BootNotification",
{
"chargePointModel": "EVlink Pro AC",
"chargePointVendor": "Schneider Electric",
"chargePointSerialNumber": "SERIAL_NUMBER",
"firmwareVersion": "1.6.1",
"meterType": "Embedded MID meter",
"meterSerialNumber": "SERIAL_NUMBER"
}]"#;

        let parsed_ocpp_message = parse_ocpp_message(ocpp_message)?;
        match parsed_ocpp_message {
            OcppMessage::Request(r) => {
                assert_eq!(r.id, MessageType::Request);
                assert_eq!(r.uuid, "1367122111");
                assert_eq!(r.message_type, MessageTypeName::BootNotification);
            }
            _ => assert!(false),
        }

        Ok(())
    }

    #[test]
    fn parse_ocpp_response_message() -> Result<(), CustomError> {
        /*
         * [
         *   3,
         *   "1367122111",
         *   {
         *       "status": "Accepted"
         *   }
         * ]
         */

        let ocpp_message = r#"
[3,"1367122111",
{
"status": "Accepted"
}]"#;

        let parsed_ocpp_message = parse_ocpp_message(ocpp_message)?;
        match parsed_ocpp_message {
            OcppMessage::Response(r) => {
                assert_eq!(r.id, MessageType::Response);
                assert_eq!(r.uuid, "1367122111");
            }
            _ => assert!(false),
        }

        Ok(())
    }

    #[test]
    fn parse_ocpp_error_message() -> Result<(), CustomError> {
        /*
         * [
         *   4,
         *   "1367122111",
         *   "PropertyConstraintViolation",
         *   ""
         *   {}
         * ]
         */

        let ocpp_message = r#"[4,"1367122111","PropertyConstraintViolation","",{}]"#;

        let parsed_ocpp_message = parse_ocpp_message(ocpp_message)?;
        match parsed_ocpp_message {
            OcppMessage::Error(r) => {
                assert_eq!(r.id, MessageType::Error);
                assert_eq!(r.uuid, "1367122111");
            }
            _ => assert!(false),
        }

        Ok(())
    }
}
