use super::archives::safe;
use super::*;
use axum::http::HeaderMap;
use lain_wire::{DeviceId, devices::*};

pub(super) fn owner_routes() -> Router<ApiState> {
    Router::new()
        .merge(super::device_commands::owner_routes())
        .route("/v1/devices", get(list).post(register))
        .route("/v1/devices/{id}", get(inspect))
        .route("/v1/devices/{id}/revoke", post(revoke))
}

pub(super) fn agent_routes(state: ApiState) -> Router {
    Router::new()
        .merge(super::device_commands::agent_routes())
        .route("/v1/device-agent/{id}", get(own))
        .route("/v1/device-agent/{id}/claim", post(claim))
        .route("/v1/device-agent/{id}/reports", post(report))
        .with_state(state)
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(middleware::from_fn(agent_response))
}

pub(super) fn proof(headers: &HeaderMap) -> Result<String, ApiError> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values
        .next()
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(device_credential_fingerprint)
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "unauthorized"))?;
    if values.next().is_some() {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    Ok(value)
}
pub(super) fn agent_error(error: anyhow::Error) -> ApiError {
    if error.is::<crate::persistence::DeviceAuthenticationDenied>() {
        ApiError(StatusCode::UNAUTHORIZED, "unauthorized")
    } else {
        control_error(error)
    }
}
async fn agent_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if matches!(
        response.status(),
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
    ) && response
        .headers()
        .get(header::CONTENT_TYPE)
        .is_none_or(|h| h != "application/json")
    {
        response = ApiError(response.status(), "invalid_input_format").into_response();
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    if response.status() == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, "Bearer".parse().unwrap());
    }
    response
}
async fn list(State(s): State<ApiState>) -> Result<Json<Vec<DeviceSnapshot>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, Kernel::devices)
            .await
            .map_err(control_error)?,
    )
}
async fn register(
    State(s): State<ApiState>,
    Json(input): Json<RegisterDevice>,
) -> Result<Json<DeviceSnapshot>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_register(input))
            .await
            .map_err(control_error)?,
    )
}
async fn inspect(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
) -> Result<Json<DeviceSnapshot>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_snapshot(id))
            .await
            .map_err(control_error)?,
    )
}
async fn revoke(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
    Json(input): Json<RevokeDevice>,
) -> Result<Json<DeviceSnapshot>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_revoke(id, input))
            .await
            .map_err(control_error)?,
    )
}
async fn own(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
    headers: HeaderMap,
) -> Result<Json<DeviceSnapshot>, ApiError> {
    let fingerprint = proof(&headers)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_own_snapshot(id, &fingerprint))
            .await
            .map_err(agent_error)?,
    )
}
async fn claim(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
    headers: HeaderMap,
    Json(input): Json<ClaimDevice>,
) -> Result<Json<DeviceSnapshot>, ApiError> {
    let fingerprint = proof(&headers)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_claim(id, &fingerprint, input))
            .await
            .map_err(agent_error)?,
    )
}
async fn report(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
    headers: HeaderMap,
    Json(input): Json<ReportDevice>,
) -> Result<Json<DeviceReport>, ApiError> {
    let fingerprint = proof(&headers)?;
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_report(id, &fingerprint, input))
            .await
            .map_err(agent_error)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request as HttpRequest;
    use lain_wire::{DeviceWorkspaceId, RequestId};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    const OWNER: &str = "fixture-owner-api-credential-123456";
    fn registration(n: u8) -> (RegisterDevice, String) {
        let token = format!("lain-device-{}", format!("{n:02x}").repeat(32));
        (
            RegisterDevice {
                id: DeviceId::new(),
                request_id: RequestId::new(),
                label: "fixture-device".into(),
                platform: DevicePlatform::Linux,
                credential_fingerprint: device_credential_fingerprint(&token).unwrap(),
                workspaces: vec![DeviceWorkspace {
                    id: DeviceWorkspaceId::new(),
                    path: "/fixture/workspace".into(),
                }],
                capabilities: vec![],
                expires_at_ms: crate::domain::now_ms().unwrap() + 600000,
            },
            token,
        )
    }
    fn request(
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> HttpRequest<Body> {
        let mut r = HttpRequest::builder()
            .method(method)
            .uri(path)
            .header("Content-Type", "application/json");
        if let Some(token) = token {
            r = r.header("Authorization", format!("Bearer {token}"));
        }
        r.body(body.map_or(Body::empty(), |v| {
            Body::from(serde_json::to_vec(&v).unwrap())
        }))
        .unwrap()
    }
    async fn call(app: &Router, request: HttpRequest<Body>) -> (StatusCode, Value) {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    fn fixture() -> (tempfile::TempDir, Arc<Kernel>, Router) {
        let dir = tempfile::tempdir().unwrap();
        let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        k.mark_ready().unwrap();
        let app = super::super::router(k.clone(), OWNER.parse().unwrap());
        (dir, k, app)
    }
    #[tokio::test]
    async fn device_api_authentication_is_separate_from_admin_and_other_devices() {
        let (_dir, k, app) = fixture();
        let (r, token) = registration(1);
        assert_eq!(
            call(&app, request("POST", "/v1/devices", None, Some(json!(r))))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
        assert!(k.devices().unwrap().is_empty());
        assert_eq!(
            call(
                &app,
                request("POST", "/v1/devices", Some(OWNER), Some(json!(r)))
            )
            .await
            .0,
            StatusCode::OK
        );
        let path = format!("/v1/device-agent/{}", r.id);
        assert_eq!(
            call(&app, request("GET", &path, Some(&token), None))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
        for key in [OWNER, r.credential_fingerprint.as_str()] {
            assert_eq!(
                call(
                    &app,
                    request(
                        "POST",
                        &format!("{path}/claim"),
                        Some(key),
                        Some(json!({"request_id":RequestId::new(),"registration_fingerprint":registration_fingerprint(&r)}))
                    )
                )
                .await
                .0,
                StatusCode::UNAUTHORIZED
            );
        }
        let claimed = call(
            &app,
            request(
                "POST",
                &format!("{path}/claim"),
                Some(&token),
                Some(json!({"request_id":RequestId::new(),"registration_fingerprint":registration_fingerprint(&r)})),
            ),
        )
        .await;
        assert_eq!(claimed.0, StatusCode::OK);
        assert_eq!(claimed.1["device"]["state"], "active");
        assert!(!claimed.1.to_string().contains(&token));
        for route in [
            "/v1/devices",
            "/v1/conversations",
            "/v1/inference/budget",
            "/v1/runtime-profiles",
            "/health",
        ] {
            assert_eq!(
                call(&app, request("GET", route, Some(&token), None))
                    .await
                    .0,
                StatusCode::UNAUTHORIZED
            );
        }
        assert_eq!(
            call(
                &app,
                request(
                    "GET",
                    &format!("/v1/device-agent/{}", DeviceId::new()),
                    Some(&token),
                    None
                )
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
        let mut duplicate = request("GET", &path, Some(&token), None);
        duplicate.headers_mut().append(
            header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        assert_eq!(call(&app, duplicate).await.0, StatusCode::UNAUTHORIZED);
    }
    #[tokio::test]
    async fn sensitive_registration_and_malformed_reports_never_persist_or_echo_values() {
        let (_dir, k, app) = fixture();
        let (mut r, token) = registration(2);
        r.label = OWNER.into();
        assert_eq!(
            call(
                &app,
                request("POST", "/v1/devices", Some(OWNER), Some(json!(r)))
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        assert!(k.devices().unwrap().is_empty());
        r.label = "fixture".into();
        k.device_register(r.clone()).unwrap();
        k.device_claim(
            r.id,
            &r.credential_fingerprint,
            ClaimDevice {
                request_id: RequestId::new(),
                registration_fingerprint: registration_fingerprint(&r),
            },
        )
        .unwrap();
        let path = format!("/v1/device-agent/{}/reports", r.id);
        let (status, value) = call(
            &app,
            request("POST", &path, Some(&token), Some(json!({"sequence":token}))),
        )
        .await;
        assert!(matches!(
            status,
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ));
        assert!(!value.to_string().contains(&token));
        assert!(k.device_snapshot(r.id).unwrap().latest_report.is_none());
    }
    #[tokio::test]
    async fn revoke_before_delayed_report_body_finishes_denies_the_write() {
        let (_dir, k, app) = fixture();
        let (r, token) = registration(3);
        k.device_register(r.clone()).unwrap();
        k.device_claim(
            r.id,
            &r.credential_fingerprint,
            ClaimDevice {
                request_id: RequestId::new(),
                registration_fingerprint: registration_fingerprint(&r),
            },
        )
        .unwrap();
        let entered = Arc::new(tokio::sync::Notify::new());
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let input = ReportDevice {
            request_id: RequestId::new(),
            sequence: 1,
            reported_at_ms: 100,
            available_workspaces: vec![r.workspaces[0].id],
            capabilities: vec![],
        };
        let body = serde_json::to_vec(&input).unwrap();
        let seen = entered.clone();
        let wait = gate.clone();
        let stream = futures_util::stream::once(async move {
            seen.notify_one();
            let _permit = wait.acquire().await.unwrap();
            Ok::<_, std::io::Error>(body)
        });
        let mut req = request(
            "POST",
            &format!("/v1/device-agent/{}/reports", r.id),
            Some(&token),
            None,
        );
        *req.body_mut() = Body::from_stream(stream);
        let app2 = app.clone();
        let task = tokio::spawn(async move { call(&app2, req).await });
        tokio::time::timeout(Duration::from_secs(5), entered.notified())
            .await
            .unwrap();
        let revoked = call(
            &app,
            request(
                "POST",
                &format!("/v1/devices/{}/revoke", r.id),
                Some(OWNER),
                Some(json!({"expected_revision":2})),
            ),
        )
        .await;
        assert_eq!(revoked.0, StatusCode::OK);
        gate.add_permits(1);
        assert_eq!(task.await.unwrap().0, StatusCode::UNAUTHORIZED);
        assert!(k.device_snapshot(r.id).unwrap().latest_report.is_none());
    }
}
