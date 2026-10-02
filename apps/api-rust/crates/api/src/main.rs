use anyhow::Context;
use astra_api::{HealthState, auth::AuthState, router};
use axum::http::{HeaderValue, Method};
use std::{env, sync::Arc};
use tower_http::{
    cors::{AllowHeaders, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let redis_url = env::var("REDIS_URL").context("REDIS_URL is required")?;
    let dependencies = Arc::new(infrastructure::HealthDependencies::new(
        &database_url,
        &redis_url,
    )?);
    let origin: HeaderValue = env::var("WEB_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:3000".into())
        .parse()?;
    let production = env::var("NODE_ENV").is_ok_and(|value| value == "production");
    let setting = |name: &str| env::var(name).ok().filter(|value| !value.is_empty());
    let supabase_url = setting("SUPABASE_URL");
    let service_key = setting("SUPABASE_SERVICE_ROLE_KEY");
    anyhow::ensure!(
        !production || (supabase_url.is_some() && service_key.is_some()),
        "Production requires SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY"
    );
    let verifier = match (supabase_url, service_key) {
        (Some(url), Some(key)) => Some(Arc::new(infrastructure::auth::SupabaseAuth::new(
            &url,
            setting("SUPABASE_JWT_SECRET"),
            setting("SUPABASE_JWKS_URL"),
            Some(key),
        )?)),
        _ => None,
    };
    let pool = infrastructure::postgres_pool(&database_url)?;
    let wallets = application::wallet::WalletService::new(Arc::new(
        infrastructure::wallet::SqlxWalletRepository::new(pool.clone()),
    ));
    let users = application::UserService::new(Arc::new(
        infrastructure::user::SqlxUserRepository::new(pool.clone()),
    ));
    let auth = AuthState {
        users,
        verifier,
        production,
        dev_user_id: setting("DEV_USER_ID"),
    };
    let (markets, _) = infrastructure::market::crypto_market_services(
        infrastructure::redis_pool(&redis_url)?,
        setting("COINGECKO_API_KEY"),
    )?;
    let news = infrastructure::news::news_service(
        infrastructure::redis_pool(&redis_url)?,
        setting("NEWSDATA_API_KEY"),
    )?;
    let accounts_router = astra_api::accounts::router(astra_api::accounts::AccountState {
        auth: auth.clone(),
        accounts: application::accounts::AccountService::new(Arc::new(
            infrastructure::accounts::SqlxAccountRepository::new(pool),
        )),
    });
    let news_router = astra_api::news::router(auth.clone(), news);
    let app = router(HealthState { dependencies }, auth, wallets)
        .merge(accounts_router)
        .merge(news_router)
        .merge(astra_api::market::router(markets))
        .layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_credentials(true)
                .allow_headers(AllowHeaders::mirror_request())
                .allow_methods([
                    Method::GET,
                    Method::HEAD,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ]),
        )
        .layer(TraceLayer::new_for_http());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3334".into())
        .parse::<u16>()?;
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await?;
    tracing::info!(%host, port, "Astra Rust API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}
async fn shutdown() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("install SIGTERM handler");
    tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
}
