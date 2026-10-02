use astra_api::{HealthState, health_router};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn health_aliases_preserve_fastify_degraded_contract() {
    let dependencies = Arc::new(
        infrastructure::HealthDependencies::new(
            "postgres://test:test@127.0.0.1:1/test",
            "redis://127.0.0.1:1",
        )
        .unwrap(),
    );
    let app = health_router(HealthState { dependencies });
    let probe = |path: &'static str| {
        let app = app.clone();
        async move {
            let response = app
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let value: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                    .unwrap();
            assert_eq!(value.as_object().unwrap().len(), 4);
            assert_eq!(value["status"], "degraded");
            assert_eq!(value["service"], "astra-wealth-api");
            assert_eq!(
                value["checks"],
                serde_json::json!({"database":"error","redis":"error"})
            );
            let timestamp = value["timestamp"].as_str().unwrap();
            assert!(timestamp.ends_with('Z'));
            assert_eq!(timestamp.len(), 24);
            chrono::DateTime::parse_from_rfc3339(timestamp).unwrap();
        }
    };
    tokio::join!(probe("/health"), probe("/v1/health"), probe("/v1/health/"));
    for path in ["/v1/me", "/users", "/__test_auth"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[test]
fn production_requires_supabase_configuration() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_astra-api"))
        .current_dir(std::env::temp_dir())
        .env_clear()
        .env("NODE_ENV", "production")
        .env("DATABASE_URL", "postgres://test:test@127.0.0.1:1/test")
        .env("REDIS_URL", "redis://127.0.0.1:1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Production requires SUPABASE_URL"));
}
