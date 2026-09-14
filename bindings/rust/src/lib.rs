//! # dropworks
//!
//! Rust binding scaffold for the Dropworks native game SDK — the
//! Steamworks-replacement surface served by a Drop server.
//!
//! This mirrors the TypeScript reference client in `src/index.ts`:
//!
//! - [`DropworksClient::sign_in`] → `POST /api/v1/dropworks/session`
//! - [`DropworksClient::unlock_achievement`] → `POST /api/v1/dropworks/achievement`
//!
//! The C ABI equivalent lives in `include/dropworks.h`. Presence
//! (`WS /api/v1/dropworks/presence`) is not implemented yet.
//!
//! ## Status
//!
//! This crate is an early scaffold: the REST calls and session handling are
//! implemented against a pluggable [`Transport`], but there is no WebSocket
//! presence client, retry policy, or C-ABI export layer yet.

mod transport;

#[cfg(feature = "c-abi")]
pub mod cabi;

pub use transport::{BoxFuture, HttpRequest, HttpResponse, Transport, TransportError};

#[cfg(feature = "http")]
pub use transport::ReqwestTransport;

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Version of the Dropworks REST contract this crate targets.
pub const DROPWORKS_API_VERSION: u32 = 1;

/// Base path for every Dropworks REST endpoint.
pub const API_BASE_PATH: &str = "/api/v1/dropworks";

/// A signed-in Dropworks session, matching the TypeScript `DropworksSession`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropworksSession {
    pub app_id: String,
    pub user_id: String,
    pub auth_token: String,
}

#[derive(Debug, Deserialize)]
struct SessionResponse {
    #[serde(rename = "userId")]
    user_id: String,
}

#[derive(Debug)]
pub enum DropworksError {
    Transport(TransportError),
    InvalidArgument(String),
    InvalidResponse(String),
    NotSignedIn,
    Http { status: u16, body: String },
}

impl fmt::Display for DropworksError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => write!(formatter, "{error}"),
            Self::InvalidArgument(message) => {
                write!(formatter, "dropworks argument was invalid: {message}")
            }
            Self::InvalidResponse(message) => {
                write!(formatter, "dropworks response was invalid: {message}")
            }
            Self::NotSignedIn => write!(formatter, "dropworks client is not signed in"),
            Self::Http { status, body } => {
                write!(formatter, "dropworks request failed with HTTP {status}: {body}")
            }
        }
    }
}

impl Error for DropworksError {}

impl From<TransportError> for DropworksError {
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}

/// Dropworks client over a caller-supplied [`Transport`].
pub struct DropworksClient<T: Transport> {
    transport: T,
    base_url: String,
    session: Option<DropworksSession>,
}

impl<T: Transport> DropworksClient<T> {
    pub fn new(transport: T, base_url: impl Into<String>) -> Self {
        let mut base_url = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self {
            transport,
            base_url,
            session: None,
        }
    }

    pub fn current_session(&self) -> Option<&DropworksSession> {
        self.session.as_ref()
    }

    /// Signs in and stores the resulting session.
    pub async fn sign_in(
        &mut self,
        app_id: &str,
        auth_token: &str,
    ) -> Result<DropworksSession, DropworksError> {
        if app_id.is_empty() || auth_token.is_empty() {
            return Err(DropworksError::InvalidArgument(
                "app id and auth token are required".to_string(),
            ));
        }
        let body = serde_json::json!({ "appId": app_id, "authToken": auth_token });
        let request = HttpRequest::post(
            format!("{}{API_BASE_PATH}/session", self.base_url),
            serde_json::to_string(&body)
                .map_err(|error| DropworksError::InvalidResponse(error.to_string()))?,
        );
        let response = self.transport.execute(request).await?;
        if !(200..300).contains(&response.status) {
            return Err(DropworksError::Http {
                status: response.status,
                body: response.body,
            });
        }
        let payload: SessionResponse = serde_json::from_str(&response.body)
            .map_err(|error| DropworksError::InvalidResponse(error.to_string()))?;
        let session = DropworksSession {
            app_id: app_id.to_string(),
            user_id: payload.user_id,
            auth_token: auth_token.to_string(),
        };
        self.session = Some(session.clone());
        Ok(session)
    }

    /// Unlocks an achievement for the signed-in user.
    pub async fn unlock_achievement(
        &self,
        achievement_id: &str,
    ) -> Result<bool, DropworksError> {
        let session = self.session.as_ref().ok_or(DropworksError::NotSignedIn)?;
        let body = serde_json::json!({
            "appId": session.app_id,
            "userId": session.user_id,
            "achievementId": achievement_id,
        });
        let mut request = HttpRequest::post(
            format!("{}{API_BASE_PATH}/achievement", self.base_url),
            serde_json::to_string(&body)
                .map_err(|error| DropworksError::InvalidResponse(error.to_string()))?,
        );
        // The server authenticates the unlock with the session's API token.
        request.headers.push((
            "authorization".to_string(),
            format!("Bearer {}", session.auth_token),
        ));
        let response = self.transport.execute(request).await?;
        Ok((200..300).contains(&response.status))
    }

    /// Submits a score to a global leaderboard for the signed-in user.
    pub async fn submit_score(
        &self,
        leaderboard_key: &str,
        score: f64,
    ) -> Result<bool, DropworksError> {
        if leaderboard_key.is_empty() {
            return Err(DropworksError::InvalidArgument(
                "leaderboard key is required".to_string(),
            ));
        }
        if !score.is_finite() {
            return Err(DropworksError::InvalidArgument(
                "score must be finite".to_string(),
            ));
        }
        let session = self.session.as_ref().ok_or(DropworksError::NotSignedIn)?;
        let body = serde_json::json!({
            "appId": session.app_id,
            "userId": session.user_id,
            "key": leaderboard_key,
            "score": score,
        });
        let mut request = HttpRequest::post(
            format!("{}{API_BASE_PATH}/leaderboard", self.base_url),
            serde_json::to_string(&body)
                .map_err(|error| DropworksError::InvalidResponse(error.to_string()))?,
        );
        request.headers.push((
            "authorization".to_string(),
            format!("Bearer {}", session.auth_token),
        ));
        let response = self.transport.execute(request).await?;
        Ok((200..300).contains(&response.status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::future::Future;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Wake, Waker};

    struct NoopWaker;

    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    /// Minimal executor for futures that never actually suspend.
    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWaker));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    struct MockTransport {
        responses: Mutex<VecDeque<(u16, String)>>,
        requests: Mutex<Vec<HttpRequest>>,
    }

    impl MockTransport {
        fn new(responses: Vec<(u16, String)>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn requests(&self) -> Vec<HttpRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Transport for MockTransport {
        fn execute<'a>(
            &'a self,
            request: HttpRequest,
        ) -> BoxFuture<'a, Result<HttpResponse, TransportError>> {
            Box::pin(async move {
                self.requests.lock().unwrap().push(request);
                let (status, body) = self
                    .responses
                    .lock()
                    .unwrap()
                    .pop_front()
                    .ok_or_else(|| TransportError("no queued response".to_string()))?;
                Ok(HttpResponse { status, body })
            })
        }
    }

    #[test]
    fn sign_in_stores_the_session_and_posts_the_contract_body() {
        let transport = MockTransport::new(vec![(
            200,
            r#"{"userId":"user-1"}"#.to_string(),
        )]);
        let mut client = DropworksClient::new(transport, "https://drop.example.com/");

        let session = block_on(client.sign_in("app-1", "token-1")).unwrap();
        assert_eq!(session.app_id, "app-1");
        assert_eq!(session.user_id, "user-1");
        assert_eq!(
            client.current_session().map(|s| s.user_id.as_str()),
            Some("user-1")
        );

        let requests = client.transport.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].url,
            "https://drop.example.com/api/v1/dropworks/session"
        );
        assert!(requests[0].body.as_deref().unwrap().contains(r#""appId":"app-1""#));
    }

    #[test]
    fn unlock_achievement_requires_sign_in_and_reports_server_status() {
        let transport = MockTransport::new(vec![
            (200, r#"{"userId":"user-1"}"#.to_string()),
            (200, "{}".to_string()),
        ]);
        let mut client = DropworksClient::new(transport, "https://drop.example.com");

        let error = block_on(client.unlock_achievement("ach-1")).unwrap_err();
        assert!(matches!(error, DropworksError::NotSignedIn));

        block_on(client.sign_in("app", "token")).unwrap();
        let unlocked = block_on(client.unlock_achievement("ach-1")).unwrap();
        assert!(unlocked);

        let requests = client.transport.requests();
        let unlock_body = requests.last().unwrap().body.as_deref().unwrap();
        assert!(unlock_body.contains(r#""achievementId":"ach-1""#));
        assert!(unlock_body.contains(r#""userId":"user-1""#));
    }

    #[test]
    fn sign_in_maps_http_failures_and_invalid_payloads() {
        let transport = MockTransport::new(vec![
            (401, "unauthorized".to_string()),
            (200, r#"{"missing":"userId"}"#.to_string()),
        ]);
        let mut client = DropworksClient::new(transport, "https://drop.example.com");

        let error = block_on(client.sign_in("app", "token")).unwrap_err();
        assert!(matches!(error, DropworksError::Http { status: 401, .. }));

        let error = block_on(client.sign_in("app", "token")).unwrap_err();
        assert!(matches!(error, DropworksError::InvalidResponse(_)));
    }

    #[test]
    fn submit_score_requires_sign_in_and_posts_the_contract_body() {
        let transport = MockTransport::new(vec![
            (200, r#"{"userId":"user-1"}"#.to_string()),
            (200, "{}".to_string()),
        ]);
        let mut client = DropworksClient::new(transport, "https://drop.example.com");

        let error = block_on(client.submit_score("high", 1.0)).unwrap_err();
        assert!(matches!(error, DropworksError::NotSignedIn));

        block_on(client.sign_in("app", "token")).unwrap();
        assert!(block_on(client.submit_score("high", 42.0)).unwrap());

        let requests = client.transport.requests();
        let body = requests.last().unwrap().body.as_deref().unwrap();
        assert!(body.contains(r#""key":"high""#));
        assert!(body.contains(r#""score":42.0"#));
    }
}
