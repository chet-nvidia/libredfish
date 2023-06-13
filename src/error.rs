use reqwest::StatusCode;

use crate::model::InvalidValueError;

#[derive(thiserror::Error, Debug)]
pub enum RedfishError {
    #[error("Network error talking to BMC at {url}. {source}")]
    NetworkError { url: String, source: reqwest::Error },

    #[error("Non-2XX HTTP status at {url}. {source}")]
    HTTPError { url: String, source: reqwest::Error },

    #[error(
        "HTTP {status_code} at {url}. Enable debug logs with `export RUST_LOG=debug` and re-run."
    )]
    HTTPErrorCode {
        url: String,
        status_code: StatusCode,
    },

    #[error("Could not deserialize response from {url}. Body: {body}. {source}")]
    JsonDeserializeError {
        url: String,
        body: String,
        source: serde_json::Error,
    },

    #[error("Could not serialize request body for {url}. Obj: {object_debug}. {source}")]
    JsonSerializeError {
        url: String,
        object_debug: String,
        source: serde_json::Error,
    },

    #[error("Remote returned empty body")]
    NoContent,

    #[error("No such boot option {0}")]
    MissingBootOption(String),

    #[error("UnnecessaryOperation such as trying to turn on a machine that is already on.")]
    UnnecessaryOperation,

    #[error("Missing key {key} in JSON at {url}")]
    MissingKey { key: String, url: String },

    #[error("Key {key} should be {expected_type} at {url}")]
    InvalidKeyType {
        key: String,
        expected_type: String,
        url: String,
    },

    #[error("Field {field} parse error at {url}: {err}")]
    InvalidValue {
        url: String,
        field: String,
        err: InvalidValueError,
    },

    #[error("BMC is locked down, operation cannot be applied. Disable lockdown and retry.")]
    Lockdown,
}
