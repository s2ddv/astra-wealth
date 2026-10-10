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
    let allow_dev_auth = astra_api::auth::development_auth_enabled(
        env::var("NODE_ENV").ok().as_deref(),
        env::var("ALLOW_DEV_AUTH").ok().as_deref(),
    );
    let setting = |name: &str| {
        env::var(name)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let supabase_url = setting("SUPABASE_URL");
    let service_key = setting("SUPABASE_SERVICE_ROLE_KEY");
    anyhow::ensure!(
        (supabase_url.is_some() && service_key.is_some())
            || (allow_dev_auth && supabase_url.is_none() && service_key.is_none()),
        "Authentication requires SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY; local development requires NODE_ENV=development and ALLOW_DEV_AUTH=true"
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
        allow_dev_auth,
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
            infrastructure::accounts::SqlxAccountRepository::new(pool.clone()),
        )),
    });
    let watchlists_router = astra_api::watchlist::router(astra_api::watchlist::WatchlistState {
        auth: auth.clone(),
        watchlists: application::watchlist::WatchlistService::new(Arc::new(
            infrastructure::watchlist::SqlxWatchlistRepository::new(pool.clone()),
        )),
    });
    let portfolio_cache = Arc::new(infrastructure::asset_icon::RedisAssetCache::new(
        infrastructure::redis_pool(&redis_url)?,
    ));
    let fx = application::fx::FxService::new(
        Arc::new(infrastructure::fx::BcbPtaxProvider::new()?),
        portfolio_cache.clone(),
    );
    let portfolio = application::portfolio::PortfolioService::new(
        Arc::new(infrastructure::portfolio::SqlxPortfolioRepository::new(
            pool.clone(),
        )),
        fx.clone(),
        Arc::new(infrastructure::tracking::CoinGeckoPriceProvider::new(
            infrastructure::asset_icon::asset_http_client()?,
            portfolio_cache,
            setting("COINGECKO_API_KEY"),
        )),
    );
    let portfolio_router = astra_api::portfolio::router(astra_api::portfolio::PortfolioState {
        auth: auth.clone(),
        portfolio,
        fx,
    });
    let tracking_router = astra_api::tracking::router(
        auth.clone(),
        infrastructure::tracking::tracking_service(
            pool,
            infrastructure::redis_pool(&redis_url)?,
            infrastructure::tracking::TrackingConfig::from_env(),
        )?,
    );
    let news_router = astra_api::news::router(auth.clone(), news);
    let app = router(HealthState { dependencies }, auth, wallets)
        .merge(accounts_router)
        .merge(portfolio_router)
        .merge(watchlists_router)
        .merge(tracking_router)
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
