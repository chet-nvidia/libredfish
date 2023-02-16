use std::{collections::HashMap, time::Duration};

use reqwest::{
    blocking::Client, blocking::ClientBuilder, header::HeaderValue, header::ACCEPT,
    header::CONTENT_TYPE, Method, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use tracing::debug;

pub use crate::RedfishError;

pub const REDFISH_ENDPOINT: &str = "redfish/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct NetworkConfig {
    /// Hostname or IP address of BMC
    pub endpoint: String,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub timeout: Duration,
    pub accept_invalid_certs: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            endpoint: "".to_string(),
            port: None,
            user: None,
            password: None,
            timeout: DEFAULT_TIMEOUT,
            // BMCs often have a self-signed cert, so usually this has to be true
            accept_invalid_certs: true,
        }
    }
}

pub struct Network {
    config: NetworkConfig,
    pub http_client: Client,
}

impl Network {
    pub fn new(config: NetworkConfig) -> Self {
        let builder = ClientBuilder::new();
        let c = builder
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .timeout(config.timeout)
            .build()
            .unwrap();
        Self {
            config,
            http_client: c,
        }
    }

    pub fn get<T>(&self, api: &str) -> Result<(StatusCode, T), RedfishError>
    where
        T: DeserializeOwned + ::std::fmt::Debug,
    {
        let (status_code, resp_opt) = self.req::<T, String>(Method::GET, api, None, None)?;
        match resp_opt {
            Some(response_body) => Ok((status_code, response_body)),
            None => Err(RedfishError::NoContent),
        }
    }

    pub fn post(&self, api: &str, data: HashMap<&str, String>) -> Result<StatusCode, RedfishError> {
        let (status_code, _resp_body): (_, Option<HashMap<String, serde_json::Value>>) =
            self.req(Method::POST, api, Some(data), None)?;
        Ok(status_code)
    }

    pub fn patch<T>(&self, api: &str, data: T) -> Result<StatusCode, RedfishError>
    where
        T: Serialize + ::std::fmt::Debug,
    {
        let (status_code, _resp_body): (_, Option<HashMap<String, serde_json::Value>>) =
            self.req(Method::PATCH, api, Some(data), None)?;
        Ok(status_code)
    }

    // All the HTTP requests happen from here.
    pub fn req<T, B>(
        &self,
        method: Method,
        api: &str,
        body: Option<B>,
        override_timeout: Option<Duration>,
    ) -> Result<(StatusCode, Option<T>), RedfishError>
    where
        T: DeserializeOwned + ::std::fmt::Debug,
        B: Serialize + ::std::fmt::Debug,
    {
        let url = match self.config.port {
            Some(p) => format!(
                "https://{}:{}/{}/{}",
                self.config.endpoint, p, REDFISH_ENDPOINT, api
            ),
            None => format!(
                "https://{}/{}/{}",
                self.config.endpoint, REDFISH_ENDPOINT, api
            ),
        };
        let body_enc = match body {
            Some(b) => {
                let url = url.clone();
                let body_enc =
                    serde_json::to_string(&b).map_err(|e| RedfishError::JsonSerializeError {
                        url,
                        object_debug: format!("{b:?}"),
                        source: e,
                    })?;
                Some(body_enc)
            }
            None => None,
        };
        debug!(
            "TX {} {} {}",
            method,
            url,
            body_enc.as_deref().unwrap_or_default()
        );

        let mut req_b = match method {
            Method::GET => self.http_client.get(&url),
            Method::POST => self.http_client.post(&url),
            Method::PATCH => self.http_client.patch(&url),
            _ => unreachable!("Only GET, POST and PATCH http methods are used."),
        };
        req_b = req_b
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(user) = &self.config.user {
            req_b = req_b.basic_auth(user, self.config.password.as_ref());
        }
        if let Some(t) = override_timeout {
            req_b = req_b.timeout(t);
        }
        if let Some(b) = body_enc {
            req_b = req_b.body(b);
        }
        let response = req_b.send().map_err(|e| RedfishError::NetworkError {
            url: url.clone(),
            source: e,
        })?;
        let status_code = response.status();
        if status_code == StatusCode::CONFLICT {
            // 409 No Content is how Dell responds if we try to turn off a system that's already off, etc.
            // Note that Lenovo accepts these unnecessary operations and returns '204 No Content'.
            return Err(RedfishError::UnnecessaryOperation);
        }
        // read the body even if not status 2XX, because BMCs give useful error messages as JSON
        let response_body = response.text().map_err(|e| RedfishError::NetworkError {
            url: url.clone(),
            source: e,
        })?;
        let mut res = None;
        if !response_body.is_empty() {
            debug!("RX {status_code} {response_body}");
            match serde_json::from_str(&response_body) {
                Ok(v) => res.insert(v),
                Err(e) => {
                    return Err(RedfishError::JsonDeserializeError {
                        url,
                        body: response_body,
                        source: e,
                    });
                }
            };
        } else {
            debug!("RX {status_code}");
        }

        if !status_code.is_success() {
            return Err(RedfishError::HTTPErrorCode { url, status_code });
        }
        Ok((status_code, res))
    }
}
