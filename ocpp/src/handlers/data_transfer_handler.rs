use crate::ocpp_types::{CustomError, MessageTypeName};

use rust_ocpp::v1_6::messages::data_transfer;

use log::info;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_data_transfer_request(
    data_transfer_request: &data_transfer::DataTransferRequest,
) -> Result<data_transfer::DataTransferResponse, CustomError> {
    info!(
        "Received {} with content: {:?}",
        MessageTypeName::DataTransfer,
        data_transfer_request
    );

    Ok(data_transfer::DataTransferResponse {
        status: rust_ocpp::v1_6::types::DataTransferStatus::Rejected,
        data: None,
    })
}
