use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Data {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub marketprice: f64,
    pub unit: String
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MarketData {
    pub object: String,
    pub data: Vec<Data>,
    pub url: String
}
