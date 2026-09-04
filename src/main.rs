use axum::{
    extract::{DefaultBodyLimit, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::net::SocketAddr;
use subtle::ConstantTimeEq;

const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_INPUT_BYTES: usize = 32 * 1024;

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
}

#[derive(Deserialize)]
struct HashRequest {
    algorithm: String,
    data: String,
}

#[derive(Deserialize)]
struct HmacRequest {
    algorithm: String,
    key: String,
    data: String,
}

#[derive(Deserialize)]
struct VerifyRequest {
    algorithm: String,
    key: String,
    data: String,
    mac: String,
}

#[derive(Serialize)]
struct HashResponse {
    algorithm: String,
    digest: String,
}

#[derive(Serialize)]
struct HmacResponse {
    algorithm: String,
    mac: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    algorithm: String,
    valid: bool,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

struct ApiError(StatusCode, &'static str);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ErrorBody {
                error: self.1.to_string(),
            }),
        )
            .into_response()
    }
}

#[derive(Clone, Copy)]
enum Algorithm {
    Sha256,
    Sha512,
}

impl Algorithm {
    fn parse(value: &str) -> Result<Self, ApiError> {
        match value {
            "sha256" => Ok(Self::Sha256),
            "sha512" => Ok(Self::Sha512),
            _ => Err(ApiError(
                StatusCode::BAD_REQUEST,
                "algorithm must be sha256 or sha512",
            )),
        }
    }
}

fn validate_input(value: &str) -> Result<(), ApiError> {
    if value.len() > MAX_INPUT_BYTES {
        return Err(ApiError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "input exceeds the 32 KiB limit",
        ));
    }

    Ok(())
}

fn hash_bytes(algorithm: Algorithm, data: &[u8]) -> Vec<u8> {
    match algorithm {
        Algorithm::Sha256 => Sha256::digest(data).to_vec(),
        Algorithm::Sha512 => Sha512::digest(data).to_vec(),
    }
}

fn hmac_bytes(algorithm: Algorithm, key: &[u8], data: &[u8]) -> Result<Vec<u8>, ApiError> {
    match algorithm {
        Algorithm::Sha256 => Hmac::<Sha256>::new_from_slice(key)
            .map(|mut mac| {
                mac.update(data);
                mac.finalize().into_bytes().to_vec()
            })
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "unable to create HMAC")),
        Algorithm::Sha512 => Hmac::<Sha512>::new_from_slice(key)
            .map(|mut mac| {
                mac.update(data);
                mac.finalize().into_bytes().to_vec()
            })
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "unable to create HMAC")),
    }
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "crypto-lab",
    })
}

async fn hash(Json(request): Json<HashRequest>) -> Result<Json<HashResponse>, ApiError> {
    let algorithm = Algorithm::parse(&request.algorithm)?;
    validate_input(&request.data)?;

    Ok(Json(HashResponse {
        algorithm: request.algorithm,
        digest: hex::encode(hash_bytes(algorithm, request.data.as_bytes())),
    }))
}

async fn hmac(Json(request): Json<HmacRequest>) -> Result<Json<HmacResponse>, ApiError> {
    let algorithm = Algorithm::parse(&request.algorithm)?;
    validate_input(&request.key)?;
    validate_input(&request.data)?;

    Ok(Json(HmacResponse {
        algorithm: request.algorithm,
        mac: hex::encode(hmac_bytes(
            algorithm,
            request.key.as_bytes(),
            request.data.as_bytes(),
        )?),
    }))
}

async fn verify(Json(request): Json<VerifyRequest>) -> Result<Json<VerifyResponse>, ApiError> {
    let algorithm = Algorithm::parse(&request.algorithm)?;
    validate_input(&request.key)?;
    validate_input(&request.data)?;
    validate_input(&request.mac)?;

    let expected = hex::decode(&request.mac)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "mac must be hexadecimal"))?;
    let actual = hmac_bytes(algorithm, request.key.as_bytes(), request.data.as_bytes())?;

    Ok(Json(VerifyResponse {
        algorithm: request.algorithm,
        valid: expected.as_slice().ct_eq(actual.as_slice()).into(),
    }))
}

fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/hash", post(hash))
        .route("/v1/hmac", post(hmac))
        .route("/v1/hmac/verify", post(verify))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8089));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind crypto-lab");

    eprintln!("crypto-lab listening on {addr}");
    axum::serve(listener, app())
        .await
        .expect("crypto-lab server failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, body::Body, http::Request};
    use serde_json::Value;
    use tower::ServiceExt;

    async fn json_response(request: Request<Body>) -> (StatusCode, Value) {
        let response = app().oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), MAX_BODY_BYTES).await.unwrap();

        (status, serde_json::from_slice(&body).unwrap())
    }

    #[test]
    fn hashes_with_sha256() {
        let digest = hash_bytes(Algorithm::Sha256, b"hello");
        assert_eq!(
            hex::encode(digest),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn verifies_hmac_and_rejects_changed_data() {
        let mac = hmac_bytes(Algorithm::Sha256, b"secret", b"message").unwrap();
        let changed = hmac_bytes(Algorithm::Sha256, b"secret", b"changed").unwrap();

        assert!(bool::from(mac.as_slice().ct_eq(mac.as_slice())));
        assert!(!bool::from(mac.as_slice().ct_eq(changed.as_slice())));
    }

    #[test]
    fn rejects_oversized_input() {
        let input = "x".repeat(MAX_INPUT_BYTES + 1);
        assert!(validate_input(&input).is_err());
    }

    #[tokio::test]
    async fn returns_health_response() {
        let (status, body) = json_response(
            Request::get("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["service"], "crypto-lab");
    }

    #[tokio::test]
    async fn rejects_unsupported_algorithm() {
        let (status, _) = json_response(
            Request::post("/v1/hash")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"algorithm":"md5","data":"hello"}"#))
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_non_json_requests() {
        let response = app()
            .oneshot(
                Request::post("/v1/hash")
                    .header("content-type", "text/plain")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn returns_sha512_digest() {
        let (status, body) = json_response(
            Request::post("/v1/hash")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"algorithm":"sha512","data":"hello"}"#))
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["digest"],
            "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043"
        );
    }

    #[tokio::test]
    async fn returns_sha512_hmac() {
        let (status, body) = json_response(
            Request::post("/v1/hmac")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"algorithm":"sha512","key":"secret","data":"message"}"#,
                ))
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["mac"],
            "1bba587c730eedba31f53abb0b6ca589e09de4e894ee455e6140807399759adaafa069eec7c01647bb173dcb17f55d22af49a18071b748c5c2edd7f7a829c632"
        );
    }

    #[tokio::test]
    async fn distinguishes_bad_mac_syntax_from_wrong_mac() {
        let (malformed_status, _) = json_response(
            Request::post("/v1/hmac/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"algorithm":"sha256","key":"secret","data":"message","mac":"not-hex"}"#,
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(malformed_status, StatusCode::BAD_REQUEST);

        let (wrong_status, wrong_body) = json_response(
            Request::post("/v1/hmac/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"algorithm":"sha256","key":"secret","data":"message","mac":"0000000000000000000000000000000000000000000000000000000000000000"}"#,
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(wrong_status, StatusCode::OK);
        assert_eq!(wrong_body["valid"], false);
    }

    #[tokio::test]
    async fn verifies_correct_mac_over_http() {
        let (status, body) = json_response(
            Request::post("/v1/hmac/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"algorithm":"sha256","key":"secret","data":"message","mac":"8b5f48702995c1598c573db1e21866a9b825d4a794d169d7060a03605796360b"}"#,
                ))
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["algorithm"], "sha256");
        assert_eq!(body["valid"], true);
    }

    #[tokio::test]
    async fn rejects_oversized_http_input() {
        let data = "x".repeat(MAX_INPUT_BYTES + 1);
        let body = serde_json::json!({ "algorithm": "sha256", "data": data });
        let response = app()
            .oneshot(
                Request::post("/v1/hash")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn returns_not_found_for_unknown_route() {
        let response = app()
            .oneshot(Request::get("/v1/unknown").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
