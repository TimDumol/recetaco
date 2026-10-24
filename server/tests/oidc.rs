//! All keys and providers in this file are test fixtures, never deployment credentials.
use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use recetaco::{
    AppState, app,
    auth::{Oidc, OidcConfig},
    db::Store,
};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

struct Provider {
    issuer: String,
    handle: tokio::task::JoinHandle<()>,
}
impl Drop for Provider {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
async fn provider(bad_issuer: bool) -> Provider {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}", listener.local_addr().unwrap());
    let declared = if bad_issuer {
        "https://wrong.example".to_string()
    } else {
        issuer.clone()
    };
    let jwks = format!("{issuer}/keys");
    let app = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(move || async move { Json(json!({"issuer":declared,"jwks_uri":jwks})) }),
        )
        .route(
            "/keys",
            get(|| async {
                Json(serde_json::from_str::<Value>(include_str!("fixtures/jwks.json")).unwrap())
            }),
        );
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Provider { issuer, handle }
}
fn config(issuer: &str) -> OidcConfig {
    OidcConfig {
        issuer: issuer.into(),
        client_id: "web-client".into(),
        android_client_id: "android-client".into(),
        audience: "https://recetaco.example".into(),
        allowed_subjects: vec![],
    }
}
fn token(issuer: &str, patch: Value, typ: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut claims = json!({"iss":issuer,"aud":"https://recetaco.example","sub":"pocket-id-user","iat":now,"exp":now+600});
    for (key, value) in patch.as_object().unwrap() {
        claims[key] = value.clone();
    }
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some(typ.into());
    header.kid = Some("test-key".into());
    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(include_bytes!("fixtures/test-only-rsa.pem")).unwrap(),
    )
    .unwrap()
}
async fn send(
    app: &Router,
    path: &str,
    credential: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let mut request = Request::builder().uri(path);
    if let Some(credential) = credential {
        request = request.header("authorization", format!("Bearer {credential}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let challenge = response
        .headers()
        .get("www-authenticate")
        .map(|s| s.to_str().unwrap().to_string());
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        challenge,
    )
}
#[tokio::test]
async fn valid_access_tokens_work_but_id_tokens_and_wrong_claims_fail() {
    let p = provider(false).await;
    let oidc = Oidc::discover(config(&p.issuer)).await.unwrap();
    let mut state = AppState::new(
        Store::open(":memory:").unwrap(),
        Some("legacy-must-not-bypass".into()),
    );
    state.oidc = Some(oidc);
    let app = app(state, "/nonexistent");
    assert_eq!(
        send(
            &app,
            "/api/recipes",
            Some(&token(&p.issuer, json!({}), "at+jwt"))
        )
        .await
        .0,
        StatusCode::OK
    );
    for (patch, typ) in [
        (json!({"iss":"https://different.example"}), "at+jwt"),
        (json!({"aud":"another-api"}), "at+jwt"),
        (json!({"exp":1}), "at+jwt"),
        (json!({"nbf":9_999_999_999_u64}), "at+jwt"),
        (json!({"sub":""}), "at+jwt"),
        (json!({"aud":"web-client"}), "JWT"),
        (json!({}), "JWT"),
        (json!({"exp":null}), "at+jwt"),
    ] {
        assert_eq!(
            send(&app, "/api/recipes", Some(&token(&p.issuer, patch, typ)))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        send(&app, "/api/recipes", Some("legacy-must-not-bypass"))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, _, challenge) = send(&app, "/api/recipes", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        challenge
            .unwrap()
            .contains("https://recetaco.example/.well-known/oauth-protected-resource")
    );
    let valid = token(&p.issuer, json!({}), "at+jwt");
    let mut bytes = valid.into_bytes();
    let index = bytes.len() - 20;
    bytes[index] = if bytes[index] == b'A' { b'B' } else { b'A' };
    assert_eq!(
        send(
            &app,
            "/api/recipes",
            Some(std::str::from_utf8(&bytes).unwrap())
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn public_configuration_and_mcp_resource_metadata_do_not_need_credentials() {
    let p = provider(false).await;
    let mut state = AppState::new(Store::open(":memory:").unwrap(), None);
    state.oidc = Some(Oidc::discover(config(&p.issuer)).await.unwrap());
    let app = app(state, "/nonexistent");
    let (s, c, _) = send(&app, "/auth/config", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(c["mode"], "oidc");
    assert_eq!(c["oidc"]["android_client_id"], "android-client");
    let (s, m, _) = send(&app, "/.well-known/oauth-protected-resource", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(m["resource"], "https://recetaco.example");
    assert_eq!(m["authorization_servers"], json!([p.issuer]));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]
async fn discovery_rejects_mismatched_issuer_and_insecure_configuration() {
    let p = provider(true).await;
    assert!(Oidc::discover(config(&p.issuer)).await.is_err());
    assert!(
        Oidc::discover(config("http://pocket-id.example"))
            .await
            .is_err()
    );
    let mut c = config("https://pocket-id.example");
    c.audience = "https://recetaco.example/api".into();
    assert!(Oidc::discover(c).await.is_err());
}

#[tokio::test]
async fn both_household_members_have_equal_write_access_and_others_are_denied() {
    let p = provider(false).await;
    let mut c = config(&p.issuer);
    c.allowed_subjects = vec!["alice".into(), "bob".into()];
    let mut state = AppState::new(Store::open(":memory:").unwrap(), None);
    state.oidc = Some(Oidc::discover(c).await.unwrap());
    let app = app(state, "/nonexistent");
    let alice = token(&p.issuer, json!({"sub":"alice"}), "at+jwt");
    let bob = token(&p.issuer, json!({"sub":"bob"}), "at+jwt");
    let other = token(&p.issuer, json!({"sub":"other"}), "at+jwt");
    let data = json!({"title":"Shared recipe","servings":2,"ingredients":[{"name":"Rice","quantity":100,"unit":"g"}]});
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/recipes")
                .header("authorization", format!("Bearer {alice}"))
                .header("content-type", "application/json")
                .body(Body::from(data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let r: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/recipes/{}", r["id"]))
                .header("authorization", format!("Bearer {bob}"))
                .header("content-type", "application/json")
                .body(Body::from(data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        send(&app, "/api/recipes", Some(&other)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(&app, "/api/recipes/1/photo", Some(&other)).await.0,
        StatusCode::FORBIDDEN
    );
    let (_, c, _) = send(&app, "/auth/config", None).await;
    assert!(c["oidc"].get("allowed_subjects").is_none());
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/recipes/{}", r["id"]))
                .header("authorization", format!("Bearer {bob}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
