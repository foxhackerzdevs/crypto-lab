mod math;

use axum::{
    extract::{DefaultBodyLimit, Json, Query},
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

const ENDPOINTS: [&str; 25] = [
    "GET /health",
    "GET /v1/info",
    "POST /v1/hash",
    "POST /v1/hmac",
    "POST /v1/hmac/verify",
    "GET /v1/math/legendre",
    "POST /v1/math/legendre",
    "GET /v1/math/sqrtmod",
    "POST /v1/math/sqrtmod",
    "GET /v1/math/factor",
    "POST /v1/math/factor",
    "GET /v1/math/totient",
    "POST /v1/math/totient",
    "GET /v1/math/order",
    "POST /v1/math/order",
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
    endpoints: [&'static str; 25],
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
        .route("/v1/math/crt", get(math_crt_get).post(math_crt_post))
        .route("/v1/math/cf", get(math_cf_get).post(math_cf_post))
        .route("/v1/math/ecadd", get(math_ecadd_get).post(math_ecadd_post))
        .route("/v1/math/ecmul", get(math_ecmul_get).post(math_ecmul_post))
        .route(
            "/v1/math/challenge",
            get(math_challenge_get).post(math_challenge_post),
        )
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
        assert_eq!(body["endpoints"].as_array().unwrap().len(), 25);
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
    }
}
