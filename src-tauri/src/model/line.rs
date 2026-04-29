use eadai::message::{LineDirection, ParserMeta, ParserStatus};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiLineDirection {
    Rx,
    Tx,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiParserMeta {
    pub parser_name: Option<String>,
    pub status: UiParserStatus,
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiParserStatus {
    Unparsed,
    Parsed,
    Malformed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiLinePayload {
    pub direction: UiLineDirection,
    pub text: String,
    pub raw_length: usize,
    pub raw: Vec<u8>,
}

impl From<LineDirection> for UiLineDirection {
    fn from(value: LineDirection) -> Self {
        match value {
            LineDirection::Rx => Self::Rx,
            LineDirection::Tx => Self::Tx,
        }
    }
}

impl From<ParserStatus> for UiParserStatus {
    fn from(value: ParserStatus) -> Self {
        match value {
            ParserStatus::Unparsed => Self::Unparsed,
            ParserStatus::Parsed => Self::Parsed,
            ParserStatus::Malformed => Self::Malformed,
        }
    }
}

impl From<ParserMeta> for UiParserMeta {
    fn from(value: ParserMeta) -> Self {
        Self {
            parser_name: value.parser_name,
            status: value.status.into(),
            fields: value
                .fields
                .into_iter()
                .map(|(key, value)| (normalize_parser_key(&key), value))
                .collect(),
        }
    }
}

fn normalize_parser_key(key: &str) -> String {
    match key {
        "channel_id" => "channelId".to_string(),
        "numeric_value" => "numericValue".to_string(),
        "field_count" => "fieldCount".to_string(),
        other => other.to_string(),
    }
}
