//! Pocket ID is the authorization server. Recetaco only validates API access tokens.
use crate::AppState;
use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone, Serialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub android_client_id: String,
    /// Resource indicator / audience of API access tokens, never an ID-token audience.
    pub audience: String,
    #[serde(skip_serializing)]
    pub allowed_subjects: Vec<String>,
}
#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    jwks_uri: String,
}
struct Keys {
    set: JwkSet,
    fetched: Instant,
    attempted: Instant,
}
pub struct Oidc {
    pub config: OidcConfig,
    http: Client,
    jwks_uri: String,
    keys: Mutex<Keys>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Principal {
    pub sub: String,
}
#[derive(Deserialize)]
struct Claims {
    sub: String,
}
#[derive(Debug)]
pub enum AuthError {
    Invalid,
    Unavailable,
    Forbidden,
}
fn valid_endpoint(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| "Invalid OIDC URL")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(local && url.scheme() == "http") {
        return Err(
            "OIDC URLs require HTTPS (HTTP allowed only on loopback for development)".into(),
        );
    }
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("OIDC URLs must not contain credentials, queries, or fragments".into());
    }
    Ok(url)
}
impl Oidc {
    pub async fn discover(config: OidcConfig) -> Result<Arc<Self>, String> {
        valid_endpoint(&config.issuer)?;
        let audience = valid_endpoint(&config.audience)?;
        if audience.path() != "/" {
            return Err(
                "RECETACO_OIDC_AUDIENCE must be Recetaco's public origin, without a path".into(),
            );
        }
        if config.client_id.trim().is_empty() || config.android_client_id.trim().is_empty() {
            return Err("OIDC client IDs cannot be empty".into());
        }
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Could not initialize OIDC HTTP client")?;
        let response = http
            .get(format!(
                "{}/.well-known/openid-configuration",
                config.issuer.trim_end_matches('/')
            ))
            .send()
            .await
            .map_err(|_| "Pocket ID discovery could not be reached")?
            .error_for_status()
            .map_err(|_| "Pocket ID discovery failed")?;
        if response.content_length().is_some_and(|n| n > 256_000) {
            return Err("OIDC discovery response too large".into());
        }
        let metadata: Discovery = response
            .json()
            .await
            .map_err(|_| "Invalid Pocket ID discovery document")?;
        if metadata.issuer != config.issuer {
            return Err(
                "Pocket ID discovery issuer does not match RECETACO_OIDC_ISSUER exactly".into(),
            );
        }
        valid_endpoint(&metadata.jwks_uri)?;
        let set = Self::fetch_keys(&http, &metadata.jwks_uri).await?;
        Ok(Arc::new(Self {
            config,
            http,
            jwks_uri: metadata.jwks_uri,
            keys: Mutex::new(Keys {
                set,
                fetched: Instant::now(),
                attempted: Instant::now(),
            }),
        }))
    }
    async fn fetch_keys(http: &Client, url: &str) -> Result<JwkSet, String> {
        let response = http
            .get(url)
            .send()
            .await
            .map_err(|_| "Pocket ID keys could not be reached")?
            .error_for_status()
            .map_err(|_| "Pocket ID key request failed")?;
        if response.content_length().is_some_and(|n| n > 256_000) {
            return Err("OIDC key response too large".into());
        }
        let set: JwkSet = response
            .json()
            .await
            .map_err(|_| "Invalid Pocket ID key set")?;
        if set.keys.is_empty() || set.keys.len() > 32 {
            return Err("Invalid Pocket ID key count".into());
        }
        Ok(set)
    }
    pub async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        if token.len() > 16384 {
            return Err(AuthError::Invalid);
        }
        let header = decode_header(token).map_err(|_| AuthError::Invalid)?;
        // Pocket ID 2.12 uses JWT; newer releases use the RFC 9068 at+jwt type.
        // The API resource audience below separates both from client-audienced ID tokens.
        if !header
            .typ
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("at+jwt") || t.eq_ignore_ascii_case("JWT"))
            || !matches!(
                header.alg,
                Algorithm::RS256
                    | Algorithm::RS384
                    | Algorithm::RS512
                    | Algorithm::ES256
                    | Algorithm::ES384
                    | Algorithm::EdDSA
            )
        {
            return Err(AuthError::Invalid);
        }
        let kid = header
            .kid
            .as_deref()
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .ok_or(AuthError::Invalid)?;
        let key = {
            let mut keys = self.keys.lock().await;
            let expired = keys.fetched.elapsed() > Duration::from_secs(300);
            let unknown = keys.set.find(kid).is_none();
            if expired && keys.attempted.elapsed() <= Duration::from_secs(30) {
                return Err(AuthError::Unavailable);
            }
            if expired || (unknown && keys.attempted.elapsed() > Duration::from_secs(30)) {
                keys.attempted = Instant::now();
                keys.set = Self::fetch_keys(&self.http, &self.jwks_uri)
                    .await
                    .map_err(|_| AuthError::Unavailable)?;
                keys.fetched = Instant::now();
            }
            let jwk = keys.set.find(kid).ok_or(AuthError::Invalid)?;
            DecodingKey::from_jwk(jwk).map_err(|_| AuthError::Invalid)?
        };
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.validate_nbf = true;
        validation.leeway = 30;
        let claims = decode::<Claims>(token, &key, &validation)
            .map_err(|_| AuthError::Invalid)?
            .claims;
        if claims.sub.trim().is_empty() {
            return Err(AuthError::Invalid);
        }
        if !self.config.allowed_subjects.is_empty()
            && !self.config.allowed_subjects.contains(&claims.sub)
        {
            return Err(AuthError::Forbidden);
        }
        Ok(Principal { sub: claims.sub })
    }
    pub fn challenge(&self) -> String {
        format!(
            "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
            self.config.audience.trim_end_matches('/')
        )
    }
}
pub async fn config(State(s): State<AppState>) -> Response {
    let value = if let Some(oidc) = s.oidc {
        json!({"mode":"oidc","oidc":oidc.config})
    } else {
        json!({"mode":if s.token.is_some() {"token"} else {"none"}})
    };
    ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response()
}
pub async fn resource_metadata(State(s): State<AppState>) -> Response {
    match s.oidc {
        Some(oidc)=>Json(json!({"resource":oidc.config.audience,"authorization_servers":[oidc.config.issuer],"scopes_supported":["openid","profile","offline_access"],"bearer_methods_supported":["header"]})).into_response(),
        None=>StatusCode::NOT_FOUND.into_response(),
    }
}
