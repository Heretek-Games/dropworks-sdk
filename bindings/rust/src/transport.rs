//! Pluggable HTTP transport for the Dropworks client.
//!
//! The core crate has no runtime dependency on a specific HTTP stack. Games
//! implement [`Transport`] (for example over an engine's async HTTP API) or
//! enable the `http` feature for the bundled [`ReqwestTransport`].

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

/// Boxed future returned by [`Transport::execute`].
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub body: Option<String>,
    pub headers: Vec<(String, String)>,
}

impl HttpRequest {
    /// JSON `POST` request, matching the Dropworks REST contract.
    pub fn post(url: String, body: String) -> Self {
        Self {
            method: "POST",
            url,
            body: Some(body),
            headers: vec![(
                "content-type".to_string(),
                "application/json".to_string(),
            )],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct TransportError(pub String);

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "dropworks transport error: {}", self.0)
    }
}

impl Error for TransportError {}

pub trait Transport: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: HttpRequest,
    ) -> BoxFuture<'a, Result<HttpResponse, TransportError>>;
}

#[cfg(feature = "http")]
mod reqwest_transport {
    use super::*;

    /// Default transport backed by `reqwest` (feature `http`).
    #[derive(Debug, Clone)]
    pub struct ReqwestTransport {
        client: reqwest::Client,
    }

    impl ReqwestTransport {
        pub fn new() -> Result<Self, TransportError> {
            let client = reqwest::Client::builder()
                .build()
                .map_err(|error| TransportError(error.to_string()))?;
            Ok(Self { client })
        }
    }

    impl Transport for ReqwestTransport {
        fn execute<'a>(
            &'a self,
            request: HttpRequest,
        ) -> BoxFuture<'a, Result<HttpResponse, TransportError>> {
            Box::pin(async move {
                let mut builder = match request.method {
                    "GET" => self.client.get(&request.url),
                    "POST" => self.client.post(&request.url),
                    other => {
                        return Err(TransportError(format!(
                            "unsupported HTTP method '{other}'"
                        )))
                    }
                };
                for (name, value) in &request.headers {
                    builder = builder.header(name, value);
                }
                if let Some(body) = request.body {
                    builder = builder.body(body);
                }
                let response =
                    builder.send().await.map_err(|error| {
                        TransportError(error.to_string())
                    })?;
                let status = response.status().as_u16();
                let body = response.text().await.map_err(|error| {
                    TransportError(error.to_string())
                })?;
                Ok(HttpResponse { status, body })
            })
        }
    }
}

#[cfg(feature = "http")]
pub use reqwest_transport::ReqwestTransport;
