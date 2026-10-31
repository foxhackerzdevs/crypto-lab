mod fermat;
mod math;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        DefaultBodyLimit, Json, Query,
    },
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

const ENDPOINTS: &[&str] = &[
    "GET /health",
    "GET /v1/info",
    "POST /v1/hash",
    "POST /v1/hmac",
    "POST /v1/hmac/verify",
    "GET /v1/math/legendre",
    "POST /v1/math/legendre",
    "GET /v1/math/jacobi",
    "POST /v1/math/jacobi",
    "GET /v1/math/solovay",
    "POST /v1/math/solovay",
    "GET /v1/math/sqrtmod",
    "POST /v1/math/sqrtmod",
    "GET /v1/math/factor",
    "POST /v1/math/factor",
    "GET /v1/math/totient",
    "POST /v1/math/totient",
    "GET /v1/math/order",
    "POST /v1/math/order",
    "GET /v1/math/dlog",
    "POST /v1/math/dlog",
    "GET /v1/math/mobius",
    "POST /v1/math/mobius",
    "GET /v1/math/sigma",
    "POST /v1/math/sigma",
    "GET /v1/math/tau",
    "POST /v1/math/tau",
    "GET /v1/math/arith",
    "POST /v1/math/arith",
    "GET /v1/math/crt",
    "POST /v1/math/crt",
    "GET /v1/math/cf",
    "POST /v1/math/cf",
    "GET /v1/math/ecadd",
    "POST /v1/math/ecadd",
    "GET /v1/math/ecmul",
    "POST /v1/math/ecmul",
    "GET /v1/math/challenge",
    "POST /v1/math/challenge",
    "GET /v1/math/rsa/keygen",
    "POST /v1/math/rsa/keygen",
    "GET /v1/math/rsa/encrypt",
    "POST /v1/math/rsa/encrypt",
    "GET /v1/math/rsa/decrypt",
    "POST /v1/math/rsa/decrypt",
    "GET /v1/math/rsa/crack",
    "POST /v1/math/rsa/crack",
    "GET /v1/math/dh/keygen",
    "POST /v1/math/dh/keygen",
    "GET /v1/math/dh/derive",
    "POST /v1/math/dh/derive",
    "GET /v1/math/dh/crack",
    "POST /v1/math/dh/crack",
    "GET /v1/math/fermat",
    "POST /v1/math/fermat",
    "GET /v1/math/chamber/ws",
    "GET /v1/math/mersenne",
    "POST /v1/math/mersenne",
    "GET /v1/math/perfect",
    "POST /v1/math/perfect",
    "GET /v1/math/pm1",
    "POST /v1/math/pm1",
    "GET /v1/math/ecm",
    "POST /v1/math/ecm",
    "GET /v1/math/dixon",
    "POST /v1/math/dixon",
];

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
}

#[derive(Serialize)]
struct Info {
    service: &'static str,
    api: &'static str,
    version: &'static str,
    endpoints: &'static [&'static str],
    build_profile: &'static str,
    environment: &'static str,
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

#[derive(Debug, PartialEq, Eq)]
pub struct ApiError(pub StatusCode, pub &'static str);

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

// ------------------- Math Request & Response Types -------------------

#[derive(Deserialize, Clone)]
struct LegendreRequest {
    a: i64,
    p: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct LegendreResponse {
    a: i64,
    p: i64,
    value: i32,
}

#[derive(Deserialize, Clone)]
struct JacobiRequest {
    a: i64,
    n: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct JacobiResponse {
    a: i64,
    n: i64,
    value: i32,
}

#[derive(Deserialize, Clone)]
struct SolovayRequest {
    n: i64,
    rounds: Option<u32>,
    seed: Option<u64>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct SolovayResponse {
    n: i64,
    rounds: u32,
    is_prime: bool,
}

#[derive(Deserialize, Clone)]
struct SqrtModRequest {
    a: i64,
    p: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct SqrtModResponse {
    a: i64,
    p: i64,
    roots: Vec<i64>,
}

#[derive(Deserialize, Clone)]
struct FactorRequest {
    n: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct FactorResponse {
    n: i64,
    factors: Vec<math::PrimeFactor>,
    prime_factors: Vec<i64>,
    expression: String,
    is_prime: bool,
}

#[derive(Deserialize, Clone)]
struct TotientRequest {
    n: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct TotientResponse {
    n: i64,
    phi: i64,
    lambda: i64,
    is_cyclic: bool,
}

#[derive(Deserialize, Clone)]
struct OrderRequest {
    a: i64,
    m: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct OrderResponse {
    a: i64,
    m: i64,
    order: i64,
    is_primitive_root: bool,
}

#[derive(Deserialize, Clone)]
struct DlogRequest {
    g: i64,
    h: i64,
    p: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct DlogResponse {
    g: i64,
    h: i64,
    p: i64,
    x: i64,
    order: i64,
}

#[derive(Deserialize, Clone)]
struct MobiusRequest {
    n: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct MobiusResponse {
    n: i64,
    value: i64,
}

#[derive(Deserialize, Clone)]
struct SigmaRequest {
    n: i64,
    k: Option<u32>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct SigmaResponse {
    n: i64,
    k: u32,
    value: i64,
}

#[derive(Deserialize, Clone)]
struct TauRequest {
    n: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct TauResponse {
    n: i64,
    value: i64,
}

#[derive(Deserialize, Clone)]
struct ArithRequest {
    n: i64,
    k: Option<u32>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct ArithResponse {
    n: i64,
    k: u32,
    mobius: i64,
    sigma: i64,
    tau: i64,
}

#[derive(Deserialize, Clone)]
struct CrtRequest {
    remainders: Option<Vec<i64>>,
    moduli: Option<Vec<i64>>,
    r: Option<String>,
    m: Option<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct CrtResponse {
    remainders: Vec<i64>,
    moduli: Vec<i64>,
    x: i64,
    modulus: i64,
}

#[derive(Deserialize, Clone)]
struct CfRequest {
    num: i64,
    den: i64,
    terms: Option<usize>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct CfResponse {
    num: i64,
    den: i64,
    cf: Vec<i64>,
    convergents: Vec<[i64; 2]>,
}

#[derive(Deserialize, Clone)]
struct EcAddRequest {
    curve: Option<String>,
    p1: Option<[i64; 2]>,
    p2: Option<[i64; 2]>,
    x1: Option<i64>,
    y1: Option<i64>,
    x2: Option<i64>,
    y2: Option<i64>,
    a: Option<i64>,
    b: Option<i64>,
    p: Option<i64>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct EcAddResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    curve: Option<String>,
    p1: [i64; 2],
    p2: [i64; 2],
    result: Option<[i64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_on_curve: Option<bool>,
}

#[derive(Deserialize, Clone)]
struct EcMulRequest {
    curve: Option<String>,
    k: i64,
    point: Option<[i64; 2]>,
    x: Option<i64>,
    y: Option<i64>,
    a: Option<i64>,
    b: Option<i64>,
    p: Option<i64>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct EcMulResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    curve: Option<String>,
    k: i64,
    point: [i64; 2],
    result: Option<[i64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_on_curve: Option<bool>,
}

#[derive(Deserialize, Clone, Default)]
struct ChallengeRequest {
    topic: Option<String>,
    count: Option<usize>,
    seed: Option<u64>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct ChallengeResponse {
    topic: String,
    seed: u64,
    count: usize,
    problems: Vec<math::ChallengeProblem>,
}

#[derive(Deserialize, Clone, Default)]
struct RsaKeygenRequest {
    bits: Option<u32>,
    e: Option<i64>,
    seed: Option<u64>,
}

#[derive(Deserialize, Clone)]
struct RsaEncryptRequest {
    m: i64,
    e: i64,
    n: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct RsaEncryptResponse {
    m: i64,
    e: i64,
    n: i64,
    c: i64,
}

#[derive(Deserialize, Clone)]
struct RsaDecryptRequest {
    c: i64,
    d: i64,
    n: i64,
    p: Option<i64>,
    q: Option<i64>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct RsaDecryptResponse {
    c: i64,
    d: i64,
    n: i64,
    m: i64,
    crt_used: bool,
}

#[derive(Deserialize, Clone)]
struct RsaCrackRequest {
    n: i64,
    e: i64,
    c: Option<i64>,
}

#[derive(Deserialize, Clone)]
struct DhKeygenRequest {
    p: i64,
    g: i64,
    private_key: Option<i64>,
    seed: Option<u64>,
}

#[derive(Deserialize, Clone)]
struct DhDeriveRequest {
    private_key: i64,
    peer_public_key: i64,
    p: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct DhDeriveResponse {
    private_key: i64,
    peer_public_key: i64,
    p: i64,
    shared_secret: i64,
}

#[derive(Deserialize, Clone)]
struct DhCrackRequest {
    g: i64,
    public_key: i64,
    p: i64,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct DhCrackResponse {
    g: i64,
    public_key: i64,
    p: i64,
    private_key: i64,
}

#[derive(Deserialize, Clone, Default)]
struct MersenneRequest {
    p: Option<u32>,
    limit: Option<u32>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
enum MersenneResponse {
    Single(math::MersenneInfo),
    List { limit: u32, primes: Vec<u32> },
}

#[derive(Deserialize, Clone, Default)]
struct PerfectRequest {
    n: Option<i64>,
    p: Option<u32>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct PerfectResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_perfect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sigma: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    p: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    perfect_number: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
struct Pm1Request {
    n: Option<i64>,
    b1: Option<u64>,
    b2: Option<u64>,
    base: Option<i64>,
    a: Option<i64>,
}

#[derive(Deserialize, Clone, Default)]
struct EcmRequest {
    n: Option<i64>,
    b1: Option<u64>,
    b2: Option<u64>,
    curves: Option<usize>,
    seed: Option<u64>,
}

#[derive(Deserialize, Clone, Default)]
struct DixonRequest {
    n: Option<i64>,
    b: Option<u64>,
    max_steps: Option<usize>,
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

async fn info() -> Json<Info> {
    Json(Info {
        service: "crypto-lab",
        api: "v1",
        version: env!("CARGO_PKG_VERSION"),
        endpoints: ENDPOINTS,
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        environment: option_env!("LAB_API_ENV").unwrap_or("unknown"),
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

// ------------------- Math Handlers -------------------

fn handle_legendre(req: LegendreRequest) -> Result<Json<LegendreResponse>, ApiError> {
    let value = math::legendre_symbol(req.a, req.p)?;
    Ok(Json(LegendreResponse {
        a: req.a,
        p: req.p,
        value,
    }))
}

async fn math_legendre_get(
    Query(req): Query<LegendreRequest>,
) -> Result<Json<LegendreResponse>, ApiError> {
    handle_legendre(req)
}

async fn math_legendre_post(
    Json(req): Json<LegendreRequest>,
) -> Result<Json<LegendreResponse>, ApiError> {
    handle_legendre(req)
}

fn handle_jacobi(req: JacobiRequest) -> Result<Json<JacobiResponse>, ApiError> {
    let value = math::jacobi_symbol(req.a, req.n)?;
    Ok(Json(JacobiResponse {
        a: req.a,
        n: req.n,
        value,
    }))
}

async fn math_jacobi_get(
    Query(req): Query<JacobiRequest>,
) -> Result<Json<JacobiResponse>, ApiError> {
    handle_jacobi(req)
}

async fn math_jacobi_post(
    Json(req): Json<JacobiRequest>,
) -> Result<Json<JacobiResponse>, ApiError> {
    handle_jacobi(req)
}

fn handle_solovay(req: SolovayRequest) -> Result<Json<SolovayResponse>, ApiError> {
    let rounds = req.rounds.unwrap_or(20);
    let seed = req.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(42)
    });
    let mut rng = math::SimpleRng::new(seed);
    let is_prime = math::solovay_strassen(req.n, rounds, &mut rng)?;
    Ok(Json(SolovayResponse {
        n: req.n,
        rounds,
        is_prime,
    }))
}

async fn math_solovay_get(
    Query(req): Query<SolovayRequest>,
) -> Result<Json<SolovayResponse>, ApiError> {
    handle_solovay(req)
}

async fn math_solovay_post(
    Json(req): Json<SolovayRequest>,
) -> Result<Json<SolovayResponse>, ApiError> {
    handle_solovay(req)
}

fn handle_sqrtmod(req: SqrtModRequest) -> Result<Json<SqrtModResponse>, ApiError> {
    let roots = math::sqrt_mod(req.a, req.p)?;
    Ok(Json(SqrtModResponse {
        a: req.a,
        p: req.p,
        roots,
    }))
}

async fn math_sqrtmod_get(
    Query(req): Query<SqrtModRequest>,
) -> Result<Json<SqrtModResponse>, ApiError> {
    handle_sqrtmod(req)
}

async fn math_sqrtmod_post(
    Json(req): Json<SqrtModRequest>,
) -> Result<Json<SqrtModResponse>, ApiError> {
    handle_sqrtmod(req)
}

fn handle_factor(req: FactorRequest) -> Result<Json<FactorResponse>, ApiError> {
    let factors = math::factorize(req.n)?;
    let is_prime = math::is_prime(req.n);
    let expression = math::format_factorization(&factors);
    let prime_factors = factors.iter().map(|f| f.prime).collect();
    Ok(Json(FactorResponse {
        n: req.n,
        factors,
        prime_factors,
        expression,
        is_prime,
    }))
}

async fn math_factor_get(
    Query(req): Query<FactorRequest>,
) -> Result<Json<FactorResponse>, ApiError> {
    handle_factor(req)
}

async fn math_factor_post(
    Json(req): Json<FactorRequest>,
) -> Result<Json<FactorResponse>, ApiError> {
    handle_factor(req)
}

fn handle_totient(req: TotientRequest) -> Result<Json<TotientResponse>, ApiError> {
    let phi = math::euler_phi(req.n)?;
    let lambda = math::carmichael_lambda(req.n)?;
    let is_cyclic = phi == lambda;
    Ok(Json(TotientResponse {
        n: req.n,
        phi,
        lambda,
        is_cyclic,
    }))
}

async fn math_totient_get(
    Query(req): Query<TotientRequest>,
) -> Result<Json<TotientResponse>, ApiError> {
    handle_totient(req)
}

async fn math_totient_post(
    Json(req): Json<TotientRequest>,
) -> Result<Json<TotientResponse>, ApiError> {
    handle_totient(req)
}

fn handle_order(req: OrderRequest) -> Result<Json<OrderResponse>, ApiError> {
    let order = math::multiplicative_order(req.a, req.m)?;
    let is_primitive_root = math::is_primitive_root(req.a, req.m)?;
    Ok(Json(OrderResponse {
        a: req.a,
        m: req.m,
        order,
        is_primitive_root,
    }))
}

async fn math_order_get(Query(req): Query<OrderRequest>) -> Result<Json<OrderResponse>, ApiError> {
    handle_order(req)
}

async fn math_order_post(Json(req): Json<OrderRequest>) -> Result<Json<OrderResponse>, ApiError> {
    handle_order(req)
}

fn handle_dlog(req: DlogRequest) -> Result<Json<DlogResponse>, ApiError> {
    let x = math::discrete_log(req.g, req.h, req.p)?;
    let order = math::multiplicative_order(req.g, req.p)?;
    Ok(Json(DlogResponse {
        g: req.g,
        h: req.h,
        p: req.p,
        x,
        order,
    }))
}

async fn math_dlog_get(Query(req): Query<DlogRequest>) -> Result<Json<DlogResponse>, ApiError> {
    handle_dlog(req)
}

async fn math_dlog_post(Json(req): Json<DlogRequest>) -> Result<Json<DlogResponse>, ApiError> {
    handle_dlog(req)
}

fn handle_mobius(req: MobiusRequest) -> Result<Json<MobiusResponse>, ApiError> {
    let value = math::mobius(req.n)?;
    Ok(Json(MobiusResponse { n: req.n, value }))
}

async fn math_mobius_get(
    Query(req): Query<MobiusRequest>,
) -> Result<Json<MobiusResponse>, ApiError> {
    handle_mobius(req)
}

async fn math_mobius_post(
    Json(req): Json<MobiusRequest>,
) -> Result<Json<MobiusResponse>, ApiError> {
    handle_mobius(req)
}

fn handle_sigma(req: SigmaRequest) -> Result<Json<SigmaResponse>, ApiError> {
    let k = req.k.unwrap_or(1);
    let value = math::sigma(req.n, k)?;
    Ok(Json(SigmaResponse { n: req.n, k, value }))
}

async fn math_sigma_get(Query(req): Query<SigmaRequest>) -> Result<Json<SigmaResponse>, ApiError> {
    handle_sigma(req)
}

async fn math_sigma_post(Json(req): Json<SigmaRequest>) -> Result<Json<SigmaResponse>, ApiError> {
    handle_sigma(req)
}

fn handle_tau(req: TauRequest) -> Result<Json<TauResponse>, ApiError> {
    let value = math::tau(req.n)?;
    Ok(Json(TauResponse { n: req.n, value }))
}

async fn math_tau_get(Query(req): Query<TauRequest>) -> Result<Json<TauResponse>, ApiError> {
    handle_tau(req)
}

async fn math_tau_post(Json(req): Json<TauRequest>) -> Result<Json<TauResponse>, ApiError> {
    handle_tau(req)
}

fn handle_arith(req: ArithRequest) -> Result<Json<ArithResponse>, ApiError> {
    let k = req.k.unwrap_or(1);
    let (mobius, sigma, tau) = math::arithmetic_suite(req.n, k)?;
    Ok(Json(ArithResponse {
        n: req.n,
        k,
        mobius,
        sigma,
        tau,
    }))
}

async fn math_arith_get(Query(req): Query<ArithRequest>) -> Result<Json<ArithResponse>, ApiError> {
    handle_arith(req)
}

async fn math_arith_post(Json(req): Json<ArithRequest>) -> Result<Json<ArithResponse>, ApiError> {
    handle_arith(req)
}

fn parse_crt_args(req: &CrtRequest) -> Result<(Vec<i64>, Vec<i64>), ApiError> {
    if let (Some(r), Some(m)) = (&req.remainders, &req.moduli) {
        return Ok((r.clone(), m.clone()));
    }
    if let (Some(r_str), Some(m_str)) = (&req.r, &req.m) {
        let r_vals = r_str
            .split(',')
            .map(|s| s.trim().parse::<i64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                ApiError(
                    StatusCode::BAD_REQUEST,
                    "r must be comma-separated integers",
                )
            })?;
        let m_vals = m_str
            .split(',')
            .map(|s| s.trim().parse::<i64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                ApiError(
                    StatusCode::BAD_REQUEST,
                    "m must be comma-separated integers",
                )
            })?;
        return Ok((r_vals, m_vals));
    }
    Err(ApiError(
        StatusCode::BAD_REQUEST,
        "provide remainders and moduli (or r and m)",
    ))
}

fn handle_crt(req: CrtRequest) -> Result<Json<CrtResponse>, ApiError> {
    let (remainders, moduli) = parse_crt_args(&req)?;
    let (x, modulus) = math::crt_solve(&remainders, &moduli)?;
    Ok(Json(CrtResponse {
        remainders,
        moduli,
        x,
        modulus,
    }))
}

async fn math_crt_get(Query(req): Query<CrtRequest>) -> Result<Json<CrtResponse>, ApiError> {
    handle_crt(req)
}

async fn math_crt_post(Json(req): Json<CrtRequest>) -> Result<Json<CrtResponse>, ApiError> {
    handle_crt(req)
}

fn handle_cf(req: CfRequest) -> Result<Json<CfResponse>, ApiError> {
    let (cf, convergents) = math::continued_fraction(req.num, req.den, req.terms.unwrap_or(100))?;
    Ok(Json(CfResponse {
        num: req.num,
        den: req.den,
        cf,
        convergents,
    }))
}

async fn math_cf_get(Query(req): Query<CfRequest>) -> Result<Json<CfResponse>, ApiError> {
    handle_cf(req)
}

async fn math_cf_post(Json(req): Json<CfRequest>) -> Result<Json<CfResponse>, ApiError> {
    handle_cf(req)
}

fn handle_ecadd(req: EcAddRequest) -> Result<Json<EcAddResponse>, ApiError> {
    let preset = req
        .curve
        .as_deref()
        .map(|name| {
            math::get_curve_preset(name)
                .ok_or(ApiError(StatusCode::BAD_REQUEST, "unknown named curve"))
        })
        .transpose()?;

    let a = req.a.or_else(|| preset.map(|c| c.a)).unwrap_or(-3);
    let b = req.b.or_else(|| preset.map(|c| c.b));
    let p = req
        .p
        .or_else(|| preset.map(|c| c.p))
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "modulus p is required"))?;

    let p1 = match (req.p1, req.x1, req.y1) {
        (Some([x, y]), _, _) => math::Point { x, y },
        (_, Some(x), Some(y)) => math::Point { x, y },
        _ => {
            if let Some(c) = preset {
                math::Point { x: c.gx, y: c.gy }
            } else {
                return Err(ApiError(
                    StatusCode::BAD_REQUEST,
                    "point 1 (p1 or x1, y1) is required",
                ));
            }
        }
    };

    let p2 = match (req.p2, req.x2, req.y2) {
        (Some([x, y]), _, _) => math::Point { x, y },
        (_, Some(x), Some(y)) => math::Point { x, y },
        _ => p1,
    };

    let result_point = math::ec_point_add(Some(p1), Some(p2), a, p)?;
    let result_arr = result_point.map(|pt| [pt.x, pt.y]);
    let result_on_curve = match (result_point, b) {
        (Some(pt), Some(b_val)) => Some(math::is_on_curve(&pt, a, b_val, p)),
        _ => None,
    };

    Ok(Json(EcAddResponse {
        curve: req.curve,
        p1: [p1.x, p1.y],
        p2: [p2.x, p2.y],
        result: result_arr,
        result_on_curve,
    }))
}

async fn math_ecadd_get(Query(req): Query<EcAddRequest>) -> Result<Json<EcAddResponse>, ApiError> {
    handle_ecadd(req)
}

async fn math_ecadd_post(Json(req): Json<EcAddRequest>) -> Result<Json<EcAddResponse>, ApiError> {
    handle_ecadd(req)
}

fn handle_ecmul(req: EcMulRequest) -> Result<Json<EcMulResponse>, ApiError> {
    let preset = req
        .curve
        .as_deref()
        .map(|name| {
            math::get_curve_preset(name)
                .ok_or(ApiError(StatusCode::BAD_REQUEST, "unknown named curve"))
        })
        .transpose()?;

    let a = req.a.or_else(|| preset.map(|c| c.a)).unwrap_or(-3);
    let b = req.b.or_else(|| preset.map(|c| c.b));
    let p = req
        .p
        .or_else(|| preset.map(|c| c.p))
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "modulus p is required"))?;

    let pt = match (req.point, req.x, req.y) {
        (Some([x, y]), _, _) => math::Point { x, y },
        (_, Some(x), Some(y)) => math::Point { x, y },
        _ => {
            if let Some(c) = preset {
                math::Point { x: c.gx, y: c.gy }
            } else {
                return Err(ApiError(
                    StatusCode::BAD_REQUEST,
                    "point (point or x, y) is required",
                ));
            }
        }
    };

    let result_point = math::ec_mul(req.k, pt, a, p)?;
    let result_arr = result_point.map(|p_res| [p_res.x, p_res.y]);
    let result_on_curve = match (result_point, b) {
        (Some(res_pt), Some(b_val)) => Some(math::is_on_curve(&res_pt, a, b_val, p)),
        _ => None,
    };

    Ok(Json(EcMulResponse {
        curve: req.curve,
        k: req.k,
        point: [pt.x, pt.y],
        result: result_arr,
        result_on_curve,
    }))
}

async fn math_ecmul_get(Query(req): Query<EcMulRequest>) -> Result<Json<EcMulResponse>, ApiError> {
    handle_ecmul(req)
}

async fn math_ecmul_post(Json(req): Json<EcMulRequest>) -> Result<Json<EcMulResponse>, ApiError> {
    handle_ecmul(req)
}

fn handle_challenge(req: ChallengeRequest) -> Result<Json<ChallengeResponse>, ApiError> {
    let topic = req.topic.as_deref().unwrap_or("mixed");
    let count = req.count.unwrap_or(1).clamp(1, 20);
    let seed = req.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(42)
    });

    let mut rng = math::SimpleRng::new(seed);
    let mut problems = Vec::with_capacity(count);
    for _ in 0..count {
        problems.push(math::generate_problem(topic, &mut rng)?);
    }

    Ok(Json(ChallengeResponse {
        topic: topic.to_string(),
        seed,
        count,
        problems,
    }))
}

async fn math_challenge_get(
    Query(req): Query<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, ApiError> {
    handle_challenge(req)
}

async fn math_challenge_post(
    Json(req): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, ApiError> {
    handle_challenge(req)
}

fn handle_rsa_keygen(req: RsaKeygenRequest) -> Result<Json<math::RsaKeypair>, ApiError> {
    let bits = req.bits.unwrap_or(32);
    let e = req.e.unwrap_or(65537);
    let seed = req.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(42)
    });
    let mut rng = math::SimpleRng::new(seed);
    let keys = math::rsa_keygen(bits, e, &mut rng)?;
    Ok(Json(keys))
}

async fn math_rsa_keygen_get(
    Query(req): Query<RsaKeygenRequest>,
) -> Result<Json<math::RsaKeypair>, ApiError> {
    handle_rsa_keygen(req)
}

async fn math_rsa_keygen_post(
    Json(req): Json<RsaKeygenRequest>,
) -> Result<Json<math::RsaKeypair>, ApiError> {
    handle_rsa_keygen(req)
}

fn handle_rsa_encrypt(req: RsaEncryptRequest) -> Result<Json<RsaEncryptResponse>, ApiError> {
    let c = math::rsa_encrypt(req.m, req.e, req.n)?;
    Ok(Json(RsaEncryptResponse {
        m: req.m,
        e: req.e,
        n: req.n,
        c,
    }))
}

async fn math_rsa_encrypt_get(
    Query(req): Query<RsaEncryptRequest>,
) -> Result<Json<RsaEncryptResponse>, ApiError> {
    handle_rsa_encrypt(req)
}

async fn math_rsa_encrypt_post(
    Json(req): Json<RsaEncryptRequest>,
) -> Result<Json<RsaEncryptResponse>, ApiError> {
    handle_rsa_encrypt(req)
}

fn handle_rsa_decrypt(req: RsaDecryptRequest) -> Result<Json<RsaDecryptResponse>, ApiError> {
    let crt_used = req.p.is_some() && req.q.is_some();
    let m = math::rsa_decrypt(req.c, req.d, req.n, req.p, req.q)?;
    Ok(Json(RsaDecryptResponse {
        c: req.c,
        d: req.d,
        n: req.n,
        m,
        crt_used,
    }))
}

async fn math_rsa_decrypt_get(
    Query(req): Query<RsaDecryptRequest>,
) -> Result<Json<RsaDecryptResponse>, ApiError> {
    handle_rsa_decrypt(req)
}

async fn math_rsa_decrypt_post(
    Json(req): Json<RsaDecryptRequest>,
) -> Result<Json<RsaDecryptResponse>, ApiError> {
    handle_rsa_decrypt(req)
}

fn handle_rsa_crack(req: RsaCrackRequest) -> Result<Json<math::RsaCrackResult>, ApiError> {
    let res = math::rsa_crack(req.n, req.e, req.c)?;
    Ok(Json(res))
}

async fn math_rsa_crack_get(
    Query(req): Query<RsaCrackRequest>,
) -> Result<Json<math::RsaCrackResult>, ApiError> {
    handle_rsa_crack(req)
}

async fn math_rsa_crack_post(
    Json(req): Json<RsaCrackRequest>,
) -> Result<Json<math::RsaCrackResult>, ApiError> {
    handle_rsa_crack(req)
}

fn handle_dh_keygen(req: DhKeygenRequest) -> Result<Json<math::DhKeypair>, ApiError> {
    let seed = req.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(42)
    });
    let mut rng = math::SimpleRng::new(seed);
    let keys = math::dh_keygen(req.p, req.g, req.private_key, &mut rng)?;
    Ok(Json(keys))
}

async fn math_dh_keygen_get(
    Query(req): Query<DhKeygenRequest>,
) -> Result<Json<math::DhKeypair>, ApiError> {
    handle_dh_keygen(req)
}

async fn math_dh_keygen_post(
    Json(req): Json<DhKeygenRequest>,
) -> Result<Json<math::DhKeypair>, ApiError> {
    handle_dh_keygen(req)
}

fn handle_dh_derive(req: DhDeriveRequest) -> Result<Json<DhDeriveResponse>, ApiError> {
    let shared_secret = math::dh_derive(req.private_key, req.peer_public_key, req.p)?;
    Ok(Json(DhDeriveResponse {
        private_key: req.private_key,
        peer_public_key: req.peer_public_key,
        p: req.p,
        shared_secret,
    }))
}

async fn math_dh_derive_get(
    Query(req): Query<DhDeriveRequest>,
) -> Result<Json<DhDeriveResponse>, ApiError> {
    handle_dh_derive(req)
}

async fn math_dh_derive_post(
    Json(req): Json<DhDeriveRequest>,
) -> Result<Json<DhDeriveResponse>, ApiError> {
    handle_dh_derive(req)
}

fn handle_dh_crack(req: DhCrackRequest) -> Result<Json<DhCrackResponse>, ApiError> {
    let res = math::dh_crack(req.g, req.public_key, req.p)?;
    Ok(Json(DhCrackResponse {
        g: res.g,
        public_key: res.public_key,
        p: res.p,
        private_key: res.private_key,
    }))
}

async fn math_dh_crack_get(
    Query(req): Query<DhCrackRequest>,
) -> Result<Json<DhCrackResponse>, ApiError> {
    handle_dh_crack(req)
}

async fn math_dh_crack_post(
    Json(req): Json<DhCrackRequest>,
) -> Result<Json<DhCrackResponse>, ApiError> {
    handle_dh_crack(req)
}

fn deserialize_u128_flexible<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        U128(u128),
        Str(String),
    }

    match NumOrStr::deserialize(deserializer)? {
        NumOrStr::U128(v) => Ok(v),
        NumOrStr::Str(s) => s.trim().parse::<u128>().map_err(serde::de::Error::custom),
    }
}

#[derive(Deserialize)]
struct FermatRequest {
    #[serde(deserialize_with = "deserialize_u128_flexible")]
    n: u128,
    #[serde(default)]
    max_steps: Option<u64>,
    #[serde(default)]
    record_steps: Option<bool>,
}

#[allow(clippy::manual_is_multiple_of)]
fn handle_fermat(req: FermatRequest) -> Result<Json<fermat::FermatSummary>, ApiError> {
    if req.n < 3 || req.n % 2 == 0 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "n must be an odd integer >= 3",
        ));
    }
    let max_steps = req.max_steps.unwrap_or(1000).min(50_000);
    let record_steps = req.record_steps.unwrap_or(true);
    let summary = fermat::fermat_factor(req.n, max_steps, record_steps)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(summary))
}

async fn math_fermat_get(
    Query(req): Query<FermatRequest>,
) -> Result<Json<fermat::FermatSummary>, ApiError> {
    handle_fermat(req)
}

async fn math_fermat_post(
    Json(req): Json<FermatRequest>,
) -> Result<Json<fermat::FermatSummary>, ApiError> {
    handle_fermat(req)
}

fn handle_mersenne(req: MersenneRequest) -> Result<Json<MersenneResponse>, ApiError> {
    if let Some(p) = req.p {
        let info = math::mersenne_info(p)?;
        Ok(Json(MersenneResponse::Single(info)))
    } else {
        let limit = req.limit.unwrap_or(127).min(5000);
        let primes = math::mersenne_primes(limit);
        Ok(Json(MersenneResponse::List { limit, primes }))
    }
}

async fn math_mersenne_get(
    Query(req): Query<MersenneRequest>,
) -> Result<Json<MersenneResponse>, ApiError> {
    handle_mersenne(req)
}

async fn math_mersenne_post(
    Json(req): Json<MersenneRequest>,
) -> Result<Json<MersenneResponse>, ApiError> {
    handle_mersenne(req)
}

fn handle_perfect(req: PerfectRequest) -> Result<Json<PerfectResponse>, ApiError> {
    if req.n.is_none() && req.p.is_none() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Either n (to test if n is perfect) or p (to generate perfect number for prime exponent p) must be provided",
        ));
    }

    let mut resp = PerfectResponse {
        n: req.n,
        is_perfect: None,
        sigma: None,
        p: req.p,
        perfect_number: None,
    };

    if let Some(n) = req.n {
        let is_p = math::is_perfect(n);
        let sig = math::sigma(n, 1).ok();
        resp.is_perfect = Some(is_p);
        resp.sigma = sig;
    }

    if let Some(p) = req.p {
        let perf = math::perfect_number(p)?;
        resp.perfect_number = Some(perf);
    }

    Ok(Json(resp))
}

async fn math_perfect_get(
    Query(req): Query<PerfectRequest>,
) -> Result<Json<PerfectResponse>, ApiError> {
    handle_perfect(req)
}

async fn math_perfect_post(
    Json(req): Json<PerfectRequest>,
) -> Result<Json<PerfectResponse>, ApiError> {
    handle_perfect(req)
}

fn handle_pm1(req: Pm1Request) -> Result<Json<math::PollardPm1Result>, ApiError> {
    let n = req.n.ok_or(ApiError(
        StatusCode::BAD_REQUEST,
        "n is required (integer to factor >= 2)",
    ))?;
    let base = req.base.or(req.a);
    let res = math::pollard_p_minus_1(n, req.b1, req.b2, base)?;
    Ok(Json(res))
}

async fn math_pm1_get(
    Query(req): Query<Pm1Request>,
) -> Result<Json<math::PollardPm1Result>, ApiError> {
    handle_pm1(req)
}

async fn math_pm1_post(
    Json(req): Json<Pm1Request>,
) -> Result<Json<math::PollardPm1Result>, ApiError> {
    handle_pm1(req)
}

fn handle_ecm(req: EcmRequest) -> Result<Json<math::EcmResult>, ApiError> {
    let n = req.n.ok_or(ApiError(
        StatusCode::BAD_REQUEST,
        "n is required (integer to factor >= 2)",
    ))?;
    let res = math::lenstra_ecm(n, req.b1, req.b2, req.curves, req.seed)?;
    Ok(Json(res))
}

async fn math_ecm_get(Query(req): Query<EcmRequest>) -> Result<Json<math::EcmResult>, ApiError> {
    handle_ecm(req)
}

async fn math_ecm_post(Json(req): Json<EcmRequest>) -> Result<Json<math::EcmResult>, ApiError> {
    handle_ecm(req)
}

fn handle_dixon(req: DixonRequest) -> Result<Json<math::DixonResult>, ApiError> {
    let n = req.n.ok_or(ApiError(
        StatusCode::BAD_REQUEST,
        "n is required (integer to factor >= 2)",
    ))?;
    let res = math::dixon_factor(n, req.b, req.max_steps)?;
    Ok(Json(res))
}

async fn math_dixon_get(
    Query(req): Query<DixonRequest>,
) -> Result<Json<math::DixonResult>, ApiError> {
    handle_dixon(req)
}

async fn math_dixon_post(
    Json(req): Json<DixonRequest>,
) -> Result<Json<math::DixonResult>, ApiError> {
    handle_dixon(req)
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum WsClientAction {
    Start {
        #[serde(deserialize_with = "deserialize_u128_flexible")]
        n: u128,
        #[serde(default)]
        delay_ms: Option<u64>,
        #[serde(default)]
        max_steps: Option<u64>,
    },
    Pause,
    Resume,
    Step,
    Speed {
        delay_ms: u64,
    },
    Stop,
}

async fn handle_client_msg(
    text: &str,
    stepper: &mut Option<fermat::FermatStepper>,
    is_paused: &mut bool,
    delay_ms: &mut u64,
    max_steps: &mut u64,
    start_time: &mut std::time::Instant,
    socket: &mut WebSocket,
) {
    let action: Result<WsClientAction, _> = serde_json::from_str(text);
    match action {
        Ok(WsClientAction::Start {
            n,
            delay_ms: d,
            max_steps: m,
        }) => match fermat::FermatStepper::new(n) {
            Ok(st) => {
                let initial_x = st.x;
                *delay_ms = d.unwrap_or(50);
                *max_steps = m.unwrap_or(1000).min(100_000);
                *stepper = Some(st);
                *is_paused = false;
                *start_time = std::time::Instant::now();
                let msg = serde_json::json!({
                    "type": "start",
                    "n": n,
                    "initial_x": initial_x
                });
                let _ = socket.send(Message::Text(msg.to_string())).await;
            }
            Err(e) => {
                let msg = serde_json::json!({
                    "type": "error",
                    "message": e
                });
                let _ = socket.send(Message::Text(msg.to_string())).await;
            }
        },
        Ok(WsClientAction::Pause) => {
            *is_paused = true;
        }
        Ok(WsClientAction::Resume) => {
            *is_paused = false;
        }
        Ok(WsClientAction::Speed { delay_ms: d }) => {
            *delay_ms = d;
        }
        Ok(WsClientAction::Stop) => {
            *stepper = None;
            let msg = serde_json::json!({
                "type": "done",
                "reason": "stopped",
                "step": 0,
                "elapsed_us": 0
            });
            let _ = socket.send(Message::Text(msg.to_string())).await;
        }
        Ok(WsClientAction::Step) => {
            *is_paused = true;
            if stepper.is_some() {
                execute_single_step(stepper, start_time, socket).await;
            }
        }
        Err(e) => {
            let msg = serde_json::json!({
                "type": "error",
                "message": format!("Invalid message format: {e}")
            });
            let _ = socket.send(Message::Text(msg.to_string())).await;
        }
    }
}

async fn execute_single_step(
    stepper_opt: &mut Option<fermat::FermatStepper>,
    start_time: &std::time::Instant,
    socket: &mut WebSocket,
) {
    let outcome = match stepper_opt.as_mut() {
        Some(st) => st.next_step(),
        None => return,
    };

    match outcome {
        fermat::StepOutcome::Step(rec) => {
            let msg = serde_json::json!({
                "type": "step",
                "step": rec.step,
                "x": rec.x,
                "r": rec.r,
                "sieve_passed": rec.sieve_passed,
                "rejected_by": rec.rejected_by,
                "is_square": rec.is_square
            });
            let _ = socket.send(Message::Text(msg.to_string())).await;
        }
        fermat::StepOutcome::Found {
            step,
            x,
            y,
            p,
            q,
            record,
        } => {
            let elapsed_us = start_time.elapsed().as_micros();
            let cand_msg = serde_json::json!({
                "type": "candidate",
                "step": record.step,
                "x": record.x,
                "r": record.r,
                "sieve_passed": record.sieve_passed,
                "is_square": record.is_square,
                "y": record.y
            });
            let _ = socket.send(Message::Text(cand_msg.to_string())).await;
            let found_msg = serde_json::json!({
                "type": "found",
                "step": step,
                "x": x,
                "y": y,
                "p": p,
                "q": q,
                "elapsed_us": elapsed_us
            });
            let _ = socket.send(Message::Text(found_msg.to_string())).await;
            let done_msg = serde_json::json!({
                "type": "done",
                "reason": "found",
                "step": step,
                "p": p,
                "q": q,
                "elapsed_us": elapsed_us
            });
            let _ = socket.send(Message::Text(done_msg.to_string())).await;
            *stepper_opt = None;
        }
        fermat::StepOutcome::Trivial {
            step, p, q, record, ..
        } => {
            let elapsed_us = start_time.elapsed().as_micros();
            let cand_msg = serde_json::json!({
                "type": "candidate",
                "step": record.step,
                "x": record.x,
                "r": record.r,
                "sieve_passed": record.sieve_passed,
                "is_square": record.is_square,
                "y": record.y
            });
            let _ = socket.send(Message::Text(cand_msg.to_string())).await;
            let done_msg = serde_json::json!({
                "type": "done",
                "reason": "trivial",
                "step": step,
                "p": p,
                "q": q,
                "elapsed_us": elapsed_us
            });
            let _ = socket.send(Message::Text(done_msg.to_string())).await;
            *stepper_opt = None;
        }
        fermat::StepOutcome::DoneMaxX { step, .. } => {
            let elapsed_us = start_time.elapsed().as_micros();
            let done_msg = serde_json::json!({
                "type": "done",
                "reason": "exhausted",
                "step": step,
                "elapsed_us": elapsed_us
            });
            let _ = socket.send(Message::Text(done_msg.to_string())).await;
            *stepper_opt = None;
        }
        fermat::StepOutcome::Overflow => {
            let elapsed_us = start_time.elapsed().as_micros();
            let step = stepper_opt.as_ref().map(|s| s.step).unwrap_or(0);
            let done_msg = serde_json::json!({
                "type": "done",
                "reason": "overflow",
                "step": step,
                "elapsed_us": elapsed_us
            });
            let _ = socket.send(Message::Text(done_msg.to_string())).await;
            *stepper_opt = None;
        }
        fermat::StepOutcome::AlreadyFinished => {
            *stepper_opt = None;
        }
    }
}

async fn chamber_ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_chamber_socket)
}

async fn handle_chamber_socket(mut socket: WebSocket) {
    let mut stepper: Option<fermat::FermatStepper> = None;
    let mut is_paused = false;
    let mut delay_ms: u64 = 50;
    let mut max_steps: u64 = 1000;
    let mut start_time = std::time::Instant::now();

    loop {
        if stepper.is_none() || is_paused {
            match socket.recv().await {
                Some(Ok(Message::Text(text))) => {
                    handle_client_msg(
                        &text,
                        &mut stepper,
                        &mut is_paused,
                        &mut delay_ms,
                        &mut max_steps,
                        &mut start_time,
                        &mut socket,
                    )
                    .await;
                }
                Some(Ok(Message::Close(_))) | None => break,
                _ => {}
            }
        } else {
            let sleep_duration = tokio::time::Duration::from_millis(delay_ms.max(1));
            tokio::select! {
                incoming = socket.recv() => {
                    match incoming {
                        Some(Ok(Message::Text(text))) => {
                            handle_client_msg(
                                &text,
                                &mut stepper,
                                &mut is_paused,
                                &mut delay_ms,
                                &mut max_steps,
                                &mut start_time,
                                &mut socket,
                            ).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(sleep_duration) => {
                    let should_stop = match stepper.as_ref() {
                        Some(st) => st.step >= max_steps,
                        None => true,
                    };

                    if should_stop {
                        let step = stepper.as_ref().map(|s| s.step).unwrap_or(0);
                        let elapsed_us = start_time.elapsed().as_micros();
                        let msg = serde_json::json!({
                            "type": "done",
                            "reason": "max_steps",
                            "step": step,
                            "elapsed_us": elapsed_us
                        });
                        let _ = socket.send(Message::Text(msg.to_string())).await;
                        stepper = None;
                    } else {
                        execute_single_step(&mut stepper, &start_time, &mut socket).await;
                    }
                }
            }
        }
    }
}

fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/info", get(info))
        .route("/v1/hash", post(hash))
        .route("/v1/hmac", post(hmac))
        .route("/v1/hmac/verify", post(verify))
        .route(
            "/v1/math/legendre",
            get(math_legendre_get).post(math_legendre_post),
        )
        .route(
            "/v1/math/jacobi",
            get(math_jacobi_get).post(math_jacobi_post),
        )
        .route(
            "/v1/math/solovay",
            get(math_solovay_get).post(math_solovay_post),
        )
        .route(
            "/v1/math/sqrtmod",
            get(math_sqrtmod_get).post(math_sqrtmod_post),
        )
        .route(
            "/v1/math/factor",
            get(math_factor_get).post(math_factor_post),
        )
        .route(
            "/v1/math/totient",
            get(math_totient_get).post(math_totient_post),
        )
        .route("/v1/math/order", get(math_order_get).post(math_order_post))
        .route("/v1/math/dlog", get(math_dlog_get).post(math_dlog_post))
        .route(
            "/v1/math/mobius",
            get(math_mobius_get).post(math_mobius_post),
        )
        .route("/v1/math/sigma", get(math_sigma_get).post(math_sigma_post))
        .route("/v1/math/tau", get(math_tau_get).post(math_tau_post))
        .route("/v1/math/arith", get(math_arith_get).post(math_arith_post))
        .route("/v1/math/crt", get(math_crt_get).post(math_crt_post))
        .route("/v1/math/cf", get(math_cf_get).post(math_cf_post))
        .route("/v1/math/ecadd", get(math_ecadd_get).post(math_ecadd_post))
        .route("/v1/math/ecmul", get(math_ecmul_get).post(math_ecmul_post))
        .route(
            "/v1/math/challenge",
            get(math_challenge_get).post(math_challenge_post),
        )
        .route(
            "/v1/math/rsa/keygen",
            get(math_rsa_keygen_get).post(math_rsa_keygen_post),
        )
        .route(
            "/v1/math/rsa/encrypt",
            get(math_rsa_encrypt_get).post(math_rsa_encrypt_post),
        )
        .route(
            "/v1/math/rsa/decrypt",
            get(math_rsa_decrypt_get).post(math_rsa_decrypt_post),
        )
        .route(
            "/v1/math/rsa/crack",
            get(math_rsa_crack_get).post(math_rsa_crack_post),
        )
        .route(
            "/v1/math/dh/keygen",
            get(math_dh_keygen_get).post(math_dh_keygen_post),
        )
        .route(
            "/v1/math/dh/derive",
            get(math_dh_derive_get).post(math_dh_derive_post),
        )
        .route(
            "/v1/math/dh/crack",
            get(math_dh_crack_get).post(math_dh_crack_post),
        )
        .route(
            "/v1/math/fermat",
            get(math_fermat_get).post(math_fermat_post),
        )
        .route("/v1/math/chamber/ws", get(chamber_ws_handler))
        .route(
            "/v1/math/mersenne",
            get(math_mersenne_get).post(math_mersenne_post),
        )
        .route(
            "/v1/math/perfect",
            get(math_perfect_get).post(math_perfect_post),
        )
        .route("/v1/math/pm1", get(math_pm1_get).post(math_pm1_post))
        .route("/v1/math/ecm", get(math_ecm_get).post(math_ecm_post))
        .route("/v1/math/dixon", get(math_dixon_get).post(math_dixon_post))
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
        let body = to_bytes(response.into_body(), MAX_BODY_BYTES)
            .await
            .unwrap();

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
        let (status, body) =
            json_response(Request::get("/health").body(Body::empty()).unwrap()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["service"], "crypto-lab");
    }

    #[tokio::test]
    async fn returns_public_info_response() {
        let (status, body) =
            json_response(Request::get("/v1/info").body(Body::empty()).unwrap()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["service"], "crypto-lab");
        assert_eq!(body["api"], "v1");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(
            body["build_profile"],
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
        );
        assert_eq!(
            body["environment"],
            option_env!("LAB_API_ENV").unwrap_or("unknown")
        );
        assert_eq!(body["endpoints"].as_array().unwrap().len(), 66);
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

    #[tokio::test]
    async fn tests_legendre_get_and_post() {
        // GET
        let (status, body) = json_response(
            Request::get("/v1/math/legendre?a=2&p=7")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["a"], 2);
        assert_eq!(body["p"], 7);
        assert_eq!(body["value"], 1);

        // POST
        let (status, body) = json_response(
            Request::post("/v1/math/legendre")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"a":5,"p":17}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], -1);
    }

    #[tokio::test]
    async fn tests_sqrtmod_get_and_post() {
        // GET
        let (status, body) = json_response(
            Request::get("/v1/math/sqrtmod?a=10&p=13")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["roots"], serde_json::json!([6, 7]));

        // POST non-residue rejection
        let (status, body) = json_response(
            Request::post("/v1/math/sqrtmod")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"a":5,"p":17}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "a has no square root modulo p");
    }

    #[tokio::test]
    async fn tests_factor_get_and_post() {
        // GET composite
        let (status, body) = json_response(
            Request::get("/v1/math/factor?n=360")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 360);
        assert_eq!(body["expression"], "2^3 * 3^2 * 5");
        assert_eq!(body["is_prime"], false);
        assert_eq!(body["prime_factors"], serde_json::json!([2, 3, 5]));
        assert_eq!(
            body["factors"],
            serde_json::json!([
                {"prime": 2, "power": 3},
                {"prime": 3, "power": 2},
                {"prime": 5, "power": 1}
            ])
        );

        // POST prime
        let (status, body) = json_response(
            Request::post("/v1/math/factor")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":104729}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 104729);
        assert_eq!(body["is_prime"], true);
        assert_eq!(body["expression"], "104729");

        // Invalid input
        let (status, body) = json_response(
            Request::get("/v1/math/factor?n=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "n must be a positive integer >= 1");
    }

    #[tokio::test]
    async fn tests_crt_get_and_post() {
        // GET with comma-separated query params
        let (status, body) = json_response(
            Request::get("/v1/math/crt?r=2,3,2&m=3,5,7")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["x"], 23);
        assert_eq!(body["modulus"], 105);

        // POST with JSON arrays
        let (status, body) = json_response(
            Request::post("/v1/math/crt")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"remainders":[2,3,2],"moduli":[3,5,7]}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["x"], 23);
        assert_eq!(body["modulus"], 105);
    }

    #[tokio::test]
    async fn tests_cf_get_and_post() {
        let (status, body) = json_response(
            Request::get("/v1/math/cf?num=355&den=113")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["cf"], serde_json::json!([3, 7, 16]));
        assert_eq!(
            body["convergents"],
            serde_json::json!([[3, 1], [22, 7], [355, 113]])
        );
    }

    #[tokio::test]
    async fn tests_ecadd_and_ecmul() {
        // ecadd on toy193 doubling generator G=(1, 59)
        let (status, body) = json_response(
            Request::get("/v1/math/ecadd?curve=toy193")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["result_on_curve"], true);

        // ecmul k=7 on toy193 -> (66, 24)
        let (status, body) = json_response(
            Request::post("/v1/math/ecmul")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"curve":"toy193","k":7}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["result"], serde_json::json!([66, 24]));
        assert_eq!(body["result_on_curve"], true);

        // ecmul k=193 on toy193 -> identity (null)
        let (status, body) = json_response(
            Request::post("/v1/math/ecmul")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"curve":"toy193","k":193}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["result"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn tests_challenge_endpoint() {
        let (status, body) = json_response(
            Request::get("/v1/math/challenge?topic=legendre&seed=42&count=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["topic"], "legendre");
        assert_eq!(body["seed"], 42);
        assert_eq!(body["count"], 2);
        assert_eq!(body["problems"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn tests_totient_and_order_get_and_post() {
        // GET totient
        let (status, body) = json_response(
            Request::get("/v1/math/totient?n=360")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 360);
        assert_eq!(body["phi"], 96);
        assert_eq!(body["lambda"], 12);
        assert_eq!(body["is_cyclic"], false);

        // POST totient prime
        let (status, body) = json_response(
            Request::post("/v1/math/totient")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":7}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 7);
        assert_eq!(body["phi"], 6);
        assert_eq!(body["lambda"], 6);
        assert_eq!(body["is_cyclic"], true);

        // GET order
        let (status, body) = json_response(
            Request::get("/v1/math/order?a=3&m=7")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["a"], 3);
        assert_eq!(body["m"], 7);
        assert_eq!(body["order"], 6);
        assert_eq!(body["is_primitive_root"], true);

        // POST order
        let (status, body) = json_response(
            Request::post("/v1/math/order")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"a":2,"m":7}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["a"], 2);
        assert_eq!(body["m"], 7);
        assert_eq!(body["order"], 3);
        assert_eq!(body["is_primitive_root"], false);

        // Challenge with totient topic
        let (status, body) = json_response(
            Request::get("/v1/math/challenge?topic=totient")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["topic"], "totient");
        assert!(body["problems"][0]["question"]
            .as_str()
            .unwrap()
            .contains("phi"));
    }

    #[tokio::test]
    async fn tests_dlog_and_arithmetic_endpoints() {
        // GET dlog
        let (status, body) = json_response(
            Request::get("/v1/math/dlog?g=3&h=6&p=7")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["g"], 3);
        assert_eq!(body["h"], 6);
        assert_eq!(body["p"], 7);
        assert_eq!(body["x"], 3);
        assert_eq!(body["order"], 6);

        // POST dlog
        let (status, body) = json_response(
            Request::post("/v1/math/dlog")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"g":2,"h":5,"p":13}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["g"], 2);
        assert_eq!(body["h"], 5);
        assert_eq!(body["p"], 13);
        assert_eq!(body["x"], 9);
        assert_eq!(body["order"], 12);

        // GET mobius
        let (status, body) = json_response(
            Request::get("/v1/math/mobius?n=30")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 30);
        assert_eq!(body["value"], -1);

        // POST mobius
        let (status, body) = json_response(
            Request::post("/v1/math/mobius")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":12}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 12);
        assert_eq!(body["value"], 0);

        // GET sigma (default k=1)
        let (status, body) = json_response(
            Request::get("/v1/math/sigma?n=12")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 12);
        assert_eq!(body["k"], 1);
        assert_eq!(body["value"], 28);

        // POST sigma with custom k
        let (status, body) = json_response(
            Request::post("/v1/math/sigma")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":6,"k":2}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 6);
        assert_eq!(body["k"], 2);
        assert_eq!(body["value"], 50);

        // GET tau
        let (status, body) = json_response(
            Request::get("/v1/math/tau?n=12")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 12);
        assert_eq!(body["value"], 6);

        // POST tau
        let (status, body) = json_response(
            Request::post("/v1/math/tau")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":36}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 36);
        assert_eq!(body["value"], 9);

        // GET arith (default k=1)
        let (status, body) = json_response(
            Request::get("/v1/math/arith?n=12")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 12);
        assert_eq!(body["k"], 1);
        assert_eq!(body["mobius"], 0);
        assert_eq!(body["sigma"], 28);
        assert_eq!(body["tau"], 6);

        // POST arith with custom k
        let (status, body) = json_response(
            Request::post("/v1/math/arith")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":6,"k":2}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 6);
        assert_eq!(body["k"], 2);
        assert_eq!(body["mobius"], 1);
        assert_eq!(body["sigma"], 50);
        assert_eq!(body["tau"], 4);

        // Challenge with dlog topic
        let (status, body) = json_response(
            Request::get("/v1/math/challenge?topic=dlog")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["topic"], "dlog");
        assert!(body["problems"][0]["question"]
            .as_str()
            .unwrap()
            .contains("such that"));

        // GET jacobi
        let (status, body) = json_response(
            Request::get("/v1/math/jacobi?a=10&n=21")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["a"], 10);
        assert_eq!(body["n"], 21);
        assert_eq!(body["value"], -1);

        // POST jacobi
        let (status, body) = json_response(
            Request::post("/v1/math/jacobi")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"a":2,"n":15}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["a"], 2);
        assert_eq!(body["n"], 15);
        assert_eq!(body["value"], 1);

        // GET solovay (prime)
        let (status, body) = json_response(
            Request::get("/v1/math/solovay?n=104729&rounds=20")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 104729);
        assert_eq!(body["rounds"], 20);
        assert_eq!(body["is_prime"], true);

        // POST solovay (Carmichael composite)
        let (status, body) = json_response(
            Request::post("/v1/math/solovay")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":561,"rounds":20,"seed":42}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 561);
        assert_eq!(body["is_prime"], false);

        // Challenge with jacobi topic
        let (status, body) = json_response(
            Request::get("/v1/math/challenge?topic=jacobi")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["topic"], "jacobi");
        assert!(body["problems"][0]["question"]
            .as_str()
            .unwrap()
            .contains("Jacobi"));
    }

    #[tokio::test]
    async fn tests_rsa_and_dh_endpoints() {
        // RSA Keygen GET
        let (status, body) = json_response(
            Request::get("/v1/math/rsa/keygen?bits=32&seed=42")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let n = body["n"].as_i64().unwrap();
        let e = body["e"].as_i64().unwrap();
        let d = body["d"].as_i64().unwrap();
        let p = body["p"].as_i64().unwrap();
        let q = body["q"].as_i64().unwrap();
        assert_eq!(p * q, n);
        assert_eq!(e, 65537);

        // RSA Encrypt GET & POST
        let (status, body_enc) = json_response(
            Request::get(format!("/v1/math/rsa/encrypt?m=42&e={e}&n={n}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let c = body_enc["c"].as_i64().unwrap();

        let (status, body_enc_post) = json_response(
            Request::post("/v1/math/rsa/encrypt")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"m":42,"e":{e},"n":{n}}}"#)))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_enc_post["c"], c);

        // RSA Decrypt GET (without CRT)
        let (status, body_dec) = json_response(
            Request::get(format!("/v1/math/rsa/decrypt?c={c}&d={d}&n={n}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_dec["m"], 42);
        assert_eq!(body_dec["crt_used"], false);

        // RSA Decrypt POST (with CRT)
        let (status, body_dec_crt) = json_response(
            Request::post("/v1/math/rsa/decrypt")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"c":{c},"d":{d},"n":{n},"p":{p},"q":{q}}}"#
                )))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_dec_crt["m"], 42);
        assert_eq!(body_dec_crt["crt_used"], true);

        // RSA Crack GET & POST
        let (status, body_crack) = json_response(
            Request::get(format!("/v1/math/rsa/crack?n={n}&e={e}&c={c}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_crack["p"], p);
        assert_eq!(body_crack["q"], q);
        assert_eq!(body_crack["d"], d);
        assert_eq!(body_crack["m"], 42);

        let (status, body_crack_post) = json_response(
            Request::post("/v1/math/rsa/crack")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"n":{n},"e":{e},"c":{c}}}"#)))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_crack_post["d"], d);
        assert_eq!(body_crack_post["m"], 42);

        // DH Keygen GET (Alice: p=23, g=5, a=6)
        let (status, body_alice) = json_response(
            Request::get("/v1/math/dh/keygen?p=23&g=5&private_key=6")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_alice["public_key"], 8);

        // DH Keygen POST (Bob: p=23, g=5, a=15)
        let (status, body_bob) = json_response(
            Request::post("/v1/math/dh/keygen")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"p":23,"g":5,"private_key":15}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_bob["public_key"], 19);

        // DH Derive GET (Alice derives from Bob's public key)
        let (status, body_s_alice) = json_response(
            Request::get("/v1/math/dh/derive?private_key=6&peer_public_key=19&p=23")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_s_alice["shared_secret"], 2);

        // DH Derive POST (Bob derives from Alice's public key)
        let (status, body_s_bob) = json_response(
            Request::post("/v1/math/dh/derive")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"private_key":15,"peer_public_key":8,"p":23}"#,
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_s_bob["shared_secret"], 2);

        // DH Crack GET & POST (recover Alice's private key 6 from public key 8)
        let (status, body_crack_dh) = json_response(
            Request::get("/v1/math/dh/crack?g=5&public_key=8&p=23")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_crack_dh["private_key"], 6);

        let (status, body_crack_dh_post) = json_response(
            Request::post("/v1/math/dh/crack")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"g":5,"public_key":19,"p":23}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_crack_dh_post["private_key"], 15);

        // Challenge RSA & DH topics
        let (status, body_chal_rsa) = json_response(
            Request::get("/v1/math/challenge?topic=rsa")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_chal_rsa["topic"], "rsa");

        let (status, body_chal_dh) = json_response(
            Request::get("/v1/math/challenge?topic=dh")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_chal_dh["topic"], "dh");
    }

    #[tokio::test]
    async fn tests_fermat_endpoints() {
        // Fermat GET with n=5959
        let (status, body) = json_response(
            Request::get("/v1/math/fermat?n=5959")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["found"], true);
        assert_eq!(body["steps_count"], 3);
        assert_eq!(body["x"], 80);
        assert_eq!(body["y"], 21);
        let p = body["p"].as_u64().unwrap();
        let q = body["q"].as_u64().unwrap();
        assert_eq!(p * q, 5959);

        // Fermat POST with string n="8051"
        let (status, body_post) = json_response(
            Request::post("/v1/math/fermat")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n":"8051","max_steps":100}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_post["found"], true);
        let p = body_post["p"].as_u64().unwrap();
        let q = body_post["q"].as_u64().unwrap();
        assert_eq!(p * q, 8051);

        // Fermat rejection of even number
        let (status, body_err) = json_response(
            Request::get("/v1/math/fermat?n=18")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body_err["error"].as_str().unwrap().contains("odd integer"));

        // Check /v1/info endpoints and version
        let (status, info_body) =
            json_response(Request::get("/v1/info").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(info_body["version"], "0.12.0");
        let endpoints = info_body["endpoints"].as_array().unwrap();
        assert_eq!(endpoints.len(), 66);
        assert!(endpoints.iter().any(|e| e == "GET /v1/math/fermat"));
        assert!(endpoints.iter().any(|e| e == "POST /v1/math/fermat"));
        assert!(endpoints.iter().any(|e| e == "GET /v1/math/chamber/ws"));
        assert!(endpoints.iter().any(|e| e == "GET /v1/math/mersenne"));
        assert!(endpoints.iter().any(|e| e == "POST /v1/math/mersenne"));
        assert!(endpoints.iter().any(|e| e == "GET /v1/math/perfect"));
        assert!(endpoints.iter().any(|e| e == "POST /v1/math/perfect"));
        assert!(endpoints.iter().any(|e| e == "GET /v1/math/pm1"));
        assert!(endpoints.iter().any(|e| e == "POST /v1/math/pm1"));
        assert!(endpoints.iter().any(|e| e == "GET /v1/math/ecm"));
        assert!(endpoints.iter().any(|e| e == "POST /v1/math/ecm"));
        assert!(endpoints.iter().any(|e| e == "GET /v1/math/dixon"));
        assert!(endpoints.iter().any(|e| e == "POST /v1/math/dixon"));
    }

    #[tokio::test]
    async fn tests_mersenne_and_perfect_endpoints() {
        // Mersenne GET p=7
        let (status, body) = json_response(
            Request::get("/v1/math/mersenne?p=7")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["p"], 7);
        assert_eq!(body["mp"], "127");
        assert_eq!(body["p_is_prime"], true);
        assert_eq!(body["is_prime"], true);
        assert_eq!(body["perfect_number"], "8128");

        // Mersenne POST p=11
        let (status, body_p11) = json_response(
            Request::post("/v1/math/mersenne")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"p": 11}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_p11["p"], 11);
        assert_eq!(body_p11["is_prime"], false);
        assert!(body_p11.get("perfect_number").is_none());

        // Mersenne primes list GET limit=31
        let (status, body_list) = json_response(
            Request::get("/v1/math/mersenne?limit=31")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_list["limit"], 31);
        let primes = body_list["primes"].as_array().unwrap();
        assert_eq!(primes, &vec![2, 3, 5, 7, 13, 17, 19, 31]);

        // Perfect GET n=28
        let (status, body_perf) = json_response(
            Request::get("/v1/math/perfect?n=28")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_perf["n"], 28);
        assert_eq!(body_perf["is_perfect"], true);
        assert_eq!(body_perf["sigma"], 56);

        // Perfect POST p=5
        let (status, body_perf_p) = json_response(
            Request::post("/v1/math/perfect")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"p": 5}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_perf_p["p"], 5);
        assert_eq!(body_perf_p["perfect_number"], "496");

        // Perfect both n and p
        let (status, body_both) = json_response(
            Request::get("/v1/math/perfect?n=6&p=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_both["n"], 6);
        assert_eq!(body_both["is_perfect"], true);
        assert_eq!(body_both["perfect_number"], "6");

        // Perfect error: neither n nor p provided
        let (status, body_err) = json_response(
            Request::get("/v1/math/perfect")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body_err["error"].as_str().unwrap().contains("Either n"));

        // Challenge Mersenne & Perfect topics
        let (status, body_chal_m) = json_response(
            Request::get("/v1/math/challenge?topic=mersenne&seed=123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_chal_m["topic"], "mersenne");

        let (status, body_chal_p) = json_response(
            Request::get("/v1/math/challenge?topic=perfect&seed=456")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body_chal_p["topic"], "perfect");
    }

    #[tokio::test]
    async fn tests_pm1_endpoints() {
        // GET with b1=20 on n = 1009 * 1381 = 1393429
        let (status, body) = json_response(
            Request::get("/v1/math/pm1?n=1393429&b1=20")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["factor"], 1009);
        assert_eq!(body["cofactor"], 1381);
        assert_eq!(body["stage"], 1);
        assert_eq!(body["p_minus_1"], 1008);

        // POST with b1=2, b2=600 (Stage 2) on n = 1019 * 1381 = 1407239
        let (status_post, body_post) = json_response(
            Request::post("/v1/math/pm1")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n": 1407239, "b1": 2, "b2": 600}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status_post, StatusCode::OK);
        assert_eq!(body_post["success"], true);
        assert_eq!(body_post["factor"], 1019);
        assert_eq!(body_post["stage"], 2);

        // Missing n -> BAD_REQUEST
        let (status_err, _) =
            json_response(Request::get("/v1/math/pm1").body(Body::empty()).unwrap()).await;
        assert_eq!(status_err, StatusCode::BAD_REQUEST);

        // Challenge pm1 topic
        let (status_chal, body_chal) = json_response(
            Request::get("/v1/math/challenge?topic=pm1&seed=789")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status_chal, StatusCode::OK);
        assert_eq!(body_chal["topic"], "pm1");
    }

    #[tokio::test]
    async fn tests_ecm_endpoints() {
        // GET with b1=50 on n = 1009 * 1381 = 1393429
        let (status, body) = json_response(
            Request::get("/v1/math/ecm?n=1393429&b1=50&curves=10&seed=42")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["factor"], 1009);
        assert_eq!(body["cofactor"], 1381);
        assert_eq!(body["stage"], 1);

        // POST on n = 701 * 1487 = 1042387 (Stage 2)
        let (status_post, body_post) = json_response(
            Request::post("/v1/math/ecm")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"n": 1042387, "b1": 50, "b2": 200, "curves": 10, "seed": 42}"#,
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(status_post, StatusCode::OK);
        assert_eq!(body_post["success"], true);
        assert_eq!(body_post["factor"], 701);
        assert_eq!(body_post["cofactor"], 1487);

        // Missing n -> BAD_REQUEST
        let (status_err, _) =
            json_response(Request::get("/v1/math/ecm").body(Body::empty()).unwrap()).await;
        assert_eq!(status_err, StatusCode::BAD_REQUEST);

        // Challenge ecm topic
        let (status_chal, body_chal) = json_response(
            Request::get("/v1/math/challenge?topic=ecm&seed=123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status_chal, StatusCode::OK);
        assert_eq!(body_chal["topic"], "ecm");
    }

    #[tokio::test]
    async fn tests_dixon_endpoints() {
        // GET with b=30 on n = 163 * 521 = 84923
        let (status, body) = json_response(
            Request::get("/v1/math/dixon?n=84923&b=30")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert!(body["factor"] == 163 || body["factor"] == 521);
        assert!(body["congruence"].is_string());

        // POST on n = 701 * 1487 = 1042387
        let (status_post, body_post) = json_response(
            Request::post("/v1/math/dixon")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"n": 1042387, "b": 50}"#))
                .unwrap(),
        )
        .await;
        assert_eq!(status_post, StatusCode::OK);
        assert_eq!(body_post["success"], true);
        assert!(body_post["factor"] == 701 || body_post["factor"] == 1487);

        // Missing n -> BAD_REQUEST
        let (status_err, _) =
            json_response(Request::get("/v1/math/dixon").body(Body::empty()).unwrap()).await;
        assert_eq!(status_err, StatusCode::BAD_REQUEST);

        // Challenge dixon topic
        let (status_chal, body_chal) = json_response(
            Request::get("/v1/math/challenge?topic=dixon&seed=123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status_chal, StatusCode::OK);
        assert_eq!(body_chal["topic"], "dixon");
    }
}
