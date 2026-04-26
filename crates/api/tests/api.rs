use api::{app, ApiState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

#[tokio::test]
async fn health_endpoint_ok() {
    let app = app(ApiState::default());
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn status_endpoint_unbound() {
    let app = app(ApiState::default());
    let resp = app
        .oneshot(Request::builder().uri("/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
