use recetaco::{
    AppState, app,
    auth::{Oidc, OidcConfig},
    db::Store,
};
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind: std::net::SocketAddr = std::env::var("RECETACO_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    let token = std::env::var("RECETACO_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let oidc = if let Ok(issuer) = std::env::var("RECETACO_OIDC_ISSUER") {
        if token.is_some() {
            return Err("Use Pocket ID or RECETACO_TOKEN, not both".into());
        }
        let client_id =
            std::env::var("RECETACO_OIDC_CLIENT_ID").map_err(|_| "Set RECETACO_OIDC_CLIENT_ID")?;
        let android_client_id =
            std::env::var("RECETACO_OIDC_ANDROID_CLIENT_ID").unwrap_or_else(|_| client_id.clone());
        let audience = std::env::var("RECETACO_OIDC_AUDIENCE")
            .map_err(|_| "Set RECETACO_OIDC_AUDIENCE to Recetaco's public origin")?;
        Some(
            Oidc::discover(OidcConfig {
                issuer,
                client_id,
                android_client_id,
                audience,
                allowed_subjects: match std::env::var("RECETACO_OIDC_ALLOWED_SUBJECTS") {
                    Ok(value) => {
                        let subjects: Vec<String> = value
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect();
                        if subjects.is_empty() {
                            return Err(
                                "RECETACO_OIDC_ALLOWED_SUBJECTS must contain at least one subject ID"
                                    .into(),
                            );
                        }
                        subjects
                    }
                    Err(_) => Vec::new(),
                },
            })
            .await?,
        )
    } else {
        None
    };
    if !bind.ip().is_loopback() && oidc.is_none() && token.as_ref().is_none_or(|t| t.len() < 24) {
        return Err(
            "Configure RECETACO_OIDC_ISSUER or a development RECETACO_TOKEN before listening beyond loopback".into(),
        );
    }
    let db = std::env::var("RECETACO_DB").unwrap_or_else(|_| "recetaco.db".into());
    let web = std::env::var("RECETACO_WEB_DIR").unwrap_or_else(|_| "web/dist".into());
    let store = Store::open(&db).map_err(|e| format!("Could not open database: {e:?}"))?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    println!("Recetaco listening on http://{bind}; MCP endpoint /mcp");
    let mut state = AppState::new(store, token);
    state.oidc = oidc;
    axum::serve(listener, app(state, &web))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
