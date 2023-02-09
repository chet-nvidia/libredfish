#[derive(thiserror::Error, Debug)]
pub enum RedfishError {
    #[error("Network error talking to BMC at {url}. {source}")]
    NetworkError { url: String, source: reqwest::Error },

    #[error("Non-2XX HTTP status at {url}. {source}")]
    HTTPError { url: String, source: reqwest::Error },

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
}
