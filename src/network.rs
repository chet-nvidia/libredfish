use std::{collections::HashMap, time::Duration};

use reqwest::{
    blocking::Client as HttpClient, blocking::ClientBuilder as HttpClientBuilder, header::HeaderValue, header::ACCEPT,
    header::CONTENT_TYPE, Method, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use tracing::debug;

pub use crate::RedfishError;

pub const REDFISH_ENDPOINT: &str = "redfish/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug)]
pub struct RedfishClientPoolBuilder {
    timeout: Duration,
    accept_invalid_certs: bool,
}

impl RedfishClientPoolBuilder {
    /// Prevents the Redfish Client from accepting self signed certificates
    /// and other invalid certificates.
    ///
    /// By default self signed certificates will be accepted, since BMCs usually
    /// use those.
    pub fn reject_invalid_certs(mut self) -> RedfishClientPoolBuilder {
        self.accept_invalid_certs = false;
        self
    }

    /// Overwrites the timeout that will be applied to every request
    pub fn timeout(mut self, timeout: Duration) -> RedfishClientPoolBuilder {
        self.timeout = timeout;
        self
    }

    /// Builds a Redfish Client Network Configuration
    pub fn build(&self) -> Result<RedfishClientPool, RedfishError> {
        let builder = HttpClientBuilder::new();
        let http_client = builder
            .danger_accept_invalid_certs(self.accept_invalid_certs)
            .timeout(self.timeout)
            .build()
            .unwrap();
        let pool = RedfishClientPool { http_client };

        Ok(pool)
    }
}

/// The endpoint that the redfish client connects to
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// Hostname or IP address of BMC
    pub host: String,
    /// BMC port. If absent the default HTTPS port 443 will be used
    pub port: Option<u16>,
    /// BMC username
    pub user: Option<String>,
    /// BMC password
    pub password: Option<String>,
}

impl Default for Endpoint {
    fn default() -> Self {
        Endpoint {
            host: "".to_string(),
            port: None,
            user: None,
            password: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RedfishClientPool {
    http_client: HttpClient,
}

impl RedfishClientPool {
    /// Returns Builder for configuring a Redfish HTTP connection pool
    pub fn builder() -> RedfishClientPoolBuilder {
        RedfishClientPoolBuilder {
            timeout: DEFAULT_TIMEOUT,
            // BMCs often have a self-signed cert, so usually this has to be true
            accept_invalid_certs: true,
        }
    }

    /// Creates a Redfish BMC client for a certain endpoint
    ///
    /// Creating the client will immediately start a HTTP request which determines
    /// the BMC type.
    pub fn create_client(
        &self,
        endpoint: Endpoint,
    ) -> Result<Box<dyn crate::Redfish>, RedfishError> {
        let client = RedfishHttpClient::new(self.http_client.clone(), endpoint);
        let s = crate::standard::RedfishStandard::new(client)?;
        match s.vendor.as_deref() {
            Some("Dell") => Ok(Box::new(crate::dell::Bmc::new(s)?)),
            Some("Lenovo") => Ok(Box::new(crate::lenovo::Bmc::new(s)?)),
            _ => Ok(Box::new(s)),
        }
    }
}

/// A HTTP client which targets a single libredfish endpoint
pub struct RedfishHttpClient {
    endpoint: Endpoint,
    http_client: HttpClient,
}

impl RedfishHttpClient {
    pub fn new(http_client: HttpClient, endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            http_client,
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
        let url = match self.endpoint.port {
            Some(p) => format!(
                "https://{}:{}/{}/{}",
                self.endpoint.host, p, REDFISH_ENDPOINT, api
            ),
            None => format!(
                "https://{}/{}/{}",
                self.endpoint.host, REDFISH_ENDPOINT, api
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
        if let Some(user) = &self.endpoint.user {
            req_b = req_b.basic_auth(user, self.endpoint.password.as_ref());
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
