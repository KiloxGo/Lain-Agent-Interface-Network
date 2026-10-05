use std::sync::Arc;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use lain_kernel::{api, config::Config, kernel::Kernel};
use tempfile::TempDir;
use tower::ServiceExt;

const TOKEN: &str = "web-test-owner-token-0123456789abcdef";

#[tokio::test]
async fn public_static_files_never_weaken_api_authentication() {
    let root = TempDir::new().unwrap();
    let web = root.path().join("web");
    std::fs::create_dir(&web).unwrap();
    std::fs::write(web.join("index.html"), "<html>public shell</html>").unwrap();
    std::fs::write(root.path().join("private.txt"), "do-not-serve").unwrap();
    let kernel = Arc::new(Kernel::open(&root.path().join("db.sqlite3")).unwrap());
    kernel.mark_ready().unwrap();
    let app = api::web_router(kernel, TOKEN.parse().unwrap(), None, web);
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert!(
        response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'")
    );
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap(),
        "<html>public shell</html>"
    );
    for path in [
        "/health",
        "/ready",
        "/v1/work-sessions",
        "/v1/events",
        "/v1/no-such-api",
    ] {
        for method in ["GET", "POST", "PUT"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{method} {path}"
            );
        }
    }
    for path in ["/private.txt", "/%2e%2e/private.txt", "/missing.js"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_ne!(response.status(), StatusCode::OK);
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/work-sessions")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), "[]");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/no-such-api")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn web_assets_are_opt_in_and_invalid_directories_fail_closed() {
    let root = TempDir::new().unwrap();
    let lookup = |key: &str| match key {
        "LAIN_API_TOKEN" => Some(TOKEN.into()),
        "LAIN_WEB_DIR" => Some(root.path().into()),
        _ => None,
    };
    assert!(Config::from_lookup(lookup).is_err());
    std::fs::write(root.path().join("index.html"), "built shell").unwrap();
    assert_eq!(
        Config::from_lookup(lookup).unwrap().web_dir.unwrap(),
        root.path().canonicalize().unwrap()
    );
    assert!(
        Config::from_lookup(|key| (key == "LAIN_API_TOKEN").then(|| TOKEN.into()))
            .unwrap()
            .web_dir
            .is_none()
    );
}
