use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
pub struct MathError(pub StatusCode, pub &'static str);

impl From<MathError> for crate::ApiError {
    fn from(err: MathError) -> Self {
        crate::ApiError(err.0, err.1)
    }
}

// ------------------- Basic Modular Arithmetic -------------------

pub fn mod_pow(mut base: i128, mut exp: i128, modulus: i128) -> i128 {
    if modulus <= 1 {
        return 0;
    }
    let mut res = 1i128;
    base = base.rem_euclid(modulus);
    while exp > 0 {
        if exp % 2 == 1 {
            res = (res * base).rem_euclid(modulus);
        }
        base = (base * base).rem_euclid(modulus);
        exp /= 2;
    }
    res
}

pub fn egcd(a: i128, b: i128) -> (i128, i128, i128) {
    if b == 0 {
        if a < 0 {
            (-a, -1, 0)
        } else {
            (a, 1, 0)
        }
    } else {
        let (g, x1, y1) = egcd(b, a.rem_euclid(b));
        let x = y1;
        let y = x1 - (a / b) * y1;
        (g, x, y)
    }
}

pub fn mod_inv(a: i128, m: i128) -> Option<i128> {
    if m <= 1 {
        return None;
    }
    let (g, x, _) = egcd(a.rem_euclid(m), m);
    if g == 1 {
        Some(x.rem_euclid(m))
    } else {
        None
    }
}

// ------------------- Legendre Symbol -------------------

pub fn legendre_symbol(a: i64, p: i64) -> Result<i32, MathError> {
    if p < 3 || p % 2 == 0 {
        return Err(MathError(StatusCode::BAD_REQUEST, "p must be an odd prime"));
    }
    let p128 = p as i128;
    let a128 = (a as i128).rem_euclid(p128);
    let res = mod_pow(a128, (p128 - 1) / 2, p128);
    if res == 1 {
        Ok(1)
    } else if res == p128 - 1 {
        Ok(-1)
    } else {
        Ok(0)
    }
}

// ------------------- Jacobi Symbol -------------------

pub fn jacobi_symbol(a: i64, n: i64) -> Result<i32, MathError> {
    if n <= 0 || n % 2 == 0 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive odd integer",
        ));
    }
    let mut a128 = (a as i128).rem_euclid(n as i128);
    let mut n128 = n as i128;
    let mut result = 1i32;

    while a128 != 0 {
        while a128 % 2 == 0 {
            a128 /= 2;
            let n_mod_8 = n128 % 8;
            if n_mod_8 == 3 || n_mod_8 == 5 {
                result = -result;
            }
        }

        if a128 % 4 == 3 && n128 % 4 == 3 {
            result = -result;
        }

        let temp = a128;
        a128 = n128 % a128;
        n128 = temp;
    }

    if n128 == 1 {
        Ok(result)
    } else {
        Ok(0)
    }
}

// ------------------- Solovay-Strassen Primality Test -------------------

pub fn solovay_strassen(n: i64, rounds: u32, rng: &mut SimpleRng) -> Result<bool, MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if rounds < 1 {
        return Err(MathError(StatusCode::BAD_REQUEST, "rounds must be >= 1"));
    }
    if n < 2 {
        return Ok(false);
    }
    if n == 2 || n == 3 {
        return Ok(true);
    }
    if n % 2 == 0 {
        return Ok(false);
    }

    let n128 = n as i128;
    let rounds = rounds.min(100);
    for _ in 0..rounds {
        let a = if n > 4 { rng.gen_range(2, n - 2) } else { 2 };
        let x = jacobi_symbol(a, n)?;
        if x == 0 {
            return Ok(false);
        }
        let mod_exp = mod_pow(a as i128, (n128 - 1) / 2, n128);
        let x_mod = (x as i128).rem_euclid(n128);
        if mod_exp != x_mod {
            return Ok(false);
        }
    }
    Ok(true)
}

// ------------------- Tonelli-Shanks Modular Square Root -------------------

pub fn sqrt_mod(a: i64, p: i64) -> Result<Vec<i64>, MathError> {
    if p < 3 || p % 2 == 0 {
        return Err(MathError(StatusCode::BAD_REQUEST, "p must be an odd prime"));
    }
    let p128 = p as i128;
    let a128 = (a as i128).rem_euclid(p128);
    let sym = legendre_symbol(a, p)?;
    if sym == 0 {
        return Ok(vec![0]);
    }
    if sym == -1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "a has no square root modulo p",
        ));
    }

    let root = if p128 % 4 == 3 {
        mod_pow(a128, (p128 + 1) / 4, p128)
    } else {
        let mut q = p128 - 1;
        let mut s = 0;
        while q % 2 == 0 {
            q /= 2;
            s += 1;
        }

        let mut non_residue = 2i64;
        while legendre_symbol(non_residue, p)? != -1 {
            non_residue += 1;
        }

        let mut c = mod_pow(non_residue as i128, q, p128);
        let mut r = mod_pow(a128, (q + 1) / 2, p128);
        let mut t = mod_pow(a128, q, p128);
        let mut m = s;

        while t != 1 {
            let mut i = 1;
            let mut t_sq = (t * t).rem_euclid(p128);
            while t_sq != 1 {
                t_sq = (t_sq * t_sq).rem_euclid(p128);
                i += 1;
                if i == m {
                    return Err(MathError(
                        StatusCode::BAD_REQUEST,
                        "a has no square root modulo p",
                    ));
                }
            }
            let exp = 1i128 << (m - i - 1);
            let b = mod_pow(c, exp, p128);
            r = (r * b).rem_euclid(p128);
            t = (t * b * b).rem_euclid(p128);
            c = (b * b).rem_euclid(p128);
            m = i;
        }
        r
    };

    let other = (-root).rem_euclid(p128);
    let r1 = root as i64;
    let r2 = other as i64;
    if r1 == r2 {
        Ok(vec![r1])
    } else if r1 < r2 {
        Ok(vec![r1, r2])
    } else {
        Ok(vec![r2, r1])
    }
}

// ------------------- Chinese Remainder Theorem -------------------

pub fn crt_solve(remainders: &[i64], moduli: &[i64]) -> Result<(i64, i64), MathError> {
    if remainders.is_empty() || remainders.len() != moduli.len() {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "remainders and moduli must be non-empty and of equal length",
        ));
    }
    for &m in moduli {
        if m < 2 {
            return Err(MathError(
                StatusCode::BAD_REQUEST,
                "each modulus must be >= 2",
            ));
        }
    }

    let mut cur_r = (remainders[0] as i128).rem_euclid(moduli[0] as i128);
    let mut cur_m = moduli[0] as i128;

    for i in 1..remainders.len() {
        let next_r = (remainders[i] as i128).rem_euclid(moduli[i] as i128);
        let next_m = moduli[i] as i128;

        let (g, x, _) = egcd(cur_m, next_m);
        let diff = next_r - cur_r;
        if diff % g != 0 {
            return Err(MathError(
                StatusCode::BAD_REQUEST,
                "system of congruences is inconsistent",
            ));
        }

        let step_m = next_m / g;
        let factor = (diff / g).rem_euclid(step_m);
        let k = (factor * x).rem_euclid(step_m);

        let new_m = cur_m * step_m;
        cur_r = (cur_r + cur_m * k).rem_euclid(new_m);
        cur_m = new_m;
    }

    Ok((cur_r as i64, cur_m as i64))
}

// ------------------- Continued Fractions -------------------

pub fn continued_fraction(
    num: i64,
    den: i64,
    max_terms: usize,
) -> Result<(Vec<i64>, Vec<[i64; 2]>), MathError> {
    if den == 0 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "denominator cannot be zero",
        ));
    }
    let mut num128 = num as i128;
    let mut den128 = den as i128;
    if den128 < 0 {
        num128 = -num128;
        den128 = -den128;
    }

    let mut terms = Vec::new();
    let limit = if max_terms == 0 {
        100
    } else {
        max_terms.min(500)
    };

    while den128 != 0 && terms.len() < limit {
        let a = num128.div_euclid(den128);
        terms.push(a as i64);
        let rem = num128.rem_euclid(den128);
        num128 = den128;
        den128 = rem;
    }

    let mut convergents = Vec::new();
    let mut h_prev2 = 0i128;
    let mut h_prev1 = 1i128;
    let mut k_prev2 = 1i128;
    let mut k_prev1 = 0i128;

    for &a in &terms {
        let a128 = a as i128;
        let h = a128 * h_prev1 + h_prev2;
        let k = a128 * k_prev1 + k_prev2;
        convergents.push([h as i64, k as i64]);
        h_prev2 = h_prev1;
        h_prev1 = h;
        k_prev2 = k_prev1;
        k_prev1 = k;
    }

    Ok((terms, convergents))
}

// ------------------- Elliptic Curves -------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct CurvePreset {
    pub a: i64,
    pub b: i64,
    pub p: i64,
    pub gx: i64,
    pub gy: i64,
}

pub fn get_curve_preset(name: &str) -> Option<CurvePreset> {
    match name {
        "toy97" => Some(CurvePreset {
            a: 2,
            b: 3,
            p: 97,
            gx: 3,
            gy: 6,
        }),
        "toy193" => Some(CurvePreset {
            a: 1,
            b: 5,
            p: 193,
            gx: 1,
            gy: 59,
        }),
        _ => None,
    }
}

pub fn is_on_curve(pt: &Point, a: i64, b: i64, p: i64) -> bool {
    let p128 = p as i128;
    let x = (pt.x as i128).rem_euclid(p128);
    let y = (pt.y as i128).rem_euclid(p128);
    let lhs = (y * y).rem_euclid(p128);
    let rhs = (x * x * x + (a as i128) * x + (b as i128)).rem_euclid(p128);
    lhs == rhs
}

pub fn ec_point_add(
    p1: Option<Point>,
    p2: Option<Point>,
    a: i64,
    p: i64,
) -> Result<Option<Point>, MathError> {
    if p < 2 {
        return Err(MathError(StatusCode::BAD_REQUEST, "modulus p must be >= 2"));
    }
    let p128 = p as i128;
    let a128 = (a as i128).rem_euclid(p128);

    let (pt1, pt2) = match (p1, p2) {
        (None, other) => return Ok(other),
        (other, None) => return Ok(other),
        (Some(first), Some(second)) => (first, second),
    };

    let x1 = (pt1.x as i128).rem_euclid(p128);
    let y1 = (pt1.y as i128).rem_euclid(p128);
    let x2 = (pt2.x as i128).rem_euclid(p128);
    let y2 = (pt2.y as i128).rem_euclid(p128);

    if x1 == x2 && (y1 + y2).rem_euclid(p128) == 0 {
        return Ok(None);
    }

    let lambda = if x1 == x2 && y1 == y2 {
        if y1 == 0 {
            return Ok(None);
        }
        let num = (3 * x1 * x1 + a128).rem_euclid(p128);
        let den = (2 * y1).rem_euclid(p128);
        let inv =
            mod_inv(den, p128).ok_or(MathError(StatusCode::BAD_REQUEST, "point at infinity"))?;
        (num * inv).rem_euclid(p128)
    } else {
        let num = (y2 - y1).rem_euclid(p128);
        let den = (x2 - x1).rem_euclid(p128);
        let inv =
            mod_inv(den, p128).ok_or(MathError(StatusCode::BAD_REQUEST, "point at infinity"))?;
        (num * inv).rem_euclid(p128)
    };

    let x3 = (lambda * lambda - x1 - x2).rem_euclid(p128);
    let y3 = (lambda * (x1 - x3) - y1).rem_euclid(p128);

    Ok(Some(Point {
        x: x3 as i64,
        y: y3 as i64,
    }))
}

pub fn ec_mul(mut k: i64, pt: Point, a: i64, p: i64) -> Result<Option<Point>, MathError> {
    if k == 0 {
        return Ok(None);
    }
    if k < 0 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "scalar k must be non-negative",
        ));
    }
    let mut current = Some(pt);
    let mut result: Option<Point> = None;

    while k > 0 {
        if k & 1 == 1 {
            result = ec_point_add(result, current, a, p)?;
        }
        current = ec_point_add(current, current, a, p)?;
        k >>= 1;
    }

    Ok(result)
}

// ------------------- Integer Factorization & Primality -------------------

pub fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 || n == 5 || n == 7 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 || n % 5 == 0 {
        return false;
    }
    let bases = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    let mut d = (n - 1) as i128;
    let mut s = 0;
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }

    let n128 = n as i128;
    'outer: for &a in &bases {
        if a >= n {
            break;
        }
        let mut x = mod_pow(a as i128, d, n128);
        if x == 1 || x == n128 - 1 {
            continue;
        }
        for _ in 0..s - 1 {
            x = (x * x).rem_euclid(n128);
            if x == n128 - 1 {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

fn pollard_brent(n: i64, rng: &mut SimpleRng) -> i64 {
    if n % 2 == 0 {
        return 2;
    }
    if is_prime(n) {
        return n;
    }

    let n128 = n as i128;
    loop {
        let mut y = rng.gen_range(1, n - 1) as i128;
        let c = rng.gen_range(1, n - 1) as i128;
        let m = 128;
        let mut g = 1i128;
        let mut r = 1usize;
        let mut q = 1i128;
        let mut ys = y;
        let mut x = y;

        while g == 1 {
            x = y;
            for _ in 0..r {
                y = (y * y + c).rem_euclid(n128);
            }
            let mut k = 0usize;
            while k < r && g == 1 {
                ys = y;
                let limit = m.min(r - k);
                for _ in 0..limit {
                    y = (y * y + c).rem_euclid(n128);
                    q = (q * (x - y).abs()).rem_euclid(n128);
                }
                let (gcd_val, _, _) = egcd(q, n128);
                g = gcd_val;
                k += limit;
            }
            r *= 2;
        }

        if g == n128 {
            g = 1;
            while g == 1 {
                ys = (ys * ys + c).rem_euclid(n128);
                let (gcd_val, _, _) = egcd((x - ys).abs(), n128);
                g = gcd_val;
            }
            if g == n128 {
                continue;
            }
        }
        return g as i64;
    }
}

fn factor_recursive(n: i64, rng: &mut SimpleRng, factors: &mut Vec<i64>) {
    if n <= 1 {
        return;
    }
    if is_prime(n) {
        factors.push(n);
        return;
    }
    let d = pollard_brent(n, rng);
    factor_recursive(d, rng, factors);
    factor_recursive(n / d, rng, factors);
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimeFactor {
    pub prime: i64,
    pub power: u32,
}

pub fn factorize(mut n: i64) -> Result<Vec<PrimeFactor>, MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if n == 1 {
        return Ok(Vec::new());
    }

    let mut factors = Vec::new();
    let small_primes = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97,
    ];

    for &p in &small_primes {
        while n % p == 0 {
            factors.push(p);
            n /= p;
        }
        if n == 1 {
            break;
        }
    }

    if n > 1 {
        if is_prime(n) {
            factors.push(n);
        } else {
            let mut rng = SimpleRng::new((n as u64) ^ 0x5DEECE66D);
            factor_recursive(n, &mut rng, &mut factors);
        }
    }

    factors.sort_unstable();
    let mut grouped: Vec<PrimeFactor> = Vec::new();
    for f in factors {
        if let Some(last) = grouped.last_mut() {
            if last.prime == f {
                last.power += 1;
                continue;
            }
        }
        grouped.push(PrimeFactor { prime: f, power: 1 });
    }

    Ok(grouped)
}

pub fn format_factorization(factors: &[PrimeFactor]) -> String {
    if factors.is_empty() {
        return "1".to_string();
    }
    factors
        .iter()
        .map(|f| {
            if f.power == 1 {
                format!("{}", f.prime)
            } else {
                format!("{}^{}", f.prime, f.power)
            }
        })
        .collect::<Vec<_>>()
        .join(" * ")
}

// ------------------- Pollard's p - 1 Factoring -------------------

pub fn sieve_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return Vec::new();
    }
    let mut is_p = vec![true; limit + 1];
    is_p[0] = false;
    is_p[1] = false;
    let mut p = 2;
    while p * p <= limit {
        if is_p[p] {
            let mut i = p * p;
            while i <= limit {
                is_p[i] = false;
                i += p;
            }
        }
        p += 1;
    }
    is_p.into_iter()
        .enumerate()
        .filter_map(|(i, prime)| if prime { Some(i) } else { None })
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PollardPm1Result {
    pub n: i64,
    pub factor: Option<i64>,
    pub cofactor: Option<i64>,
    pub success: bool,
    pub stage: Option<u32>,
    pub b1: u64,
    pub b2: u64,
    pub base: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p_minus_1: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p_minus_1_factors: Option<Vec<PrimeFactor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p_minus_1_expression: Option<String>,
    pub steps: usize,
    pub message: String,
    pub elapsed_us: u128,
}

pub fn pollard_p_minus_1(
    n: i64,
    b1_opt: Option<u64>,
    b2_opt: Option<u64>,
    base_opt: Option<i64>,
) -> Result<PollardPm1Result, MathError> {
    let start = std::time::Instant::now();
    if n < 2 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be an integer >= 2",
        ));
    }

    let b1 = b1_opt.unwrap_or(1000).clamp(2, 1_000_000);
    let b2 = b2_opt.unwrap_or(b1 * 10).clamp(b1, 5_000_000);
    let base = base_opt.unwrap_or(2);

    if base <= 1 || base >= n {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Base a must satisfy 1 < a < n",
        ));
    }

    if n % 2 == 0 {
        let p = 2i64;
        let q = n / 2;
        let p_minus_1 = p - 1;
        let factors = factorize(p_minus_1).ok();
        let expr = factors.as_ref().map(|f| format_factorization(f));
        return Ok(PollardPm1Result {
            n,
            factor: Some(p),
            cofactor: Some(q),
            success: true,
            stage: Some(0),
            b1,
            b2,
            base,
            p_minus_1: Some(p_minus_1),
            p_minus_1_factors: factors,
            p_minus_1_expression: expr,
            steps: 0,
            message: "n is even; factor 2 found immediately".to_string(),
            elapsed_us: start.elapsed().as_micros(),
        });
    }

    if is_prime(n) {
        return Ok(PollardPm1Result {
            n,
            factor: None,
            cofactor: None,
            success: false,
            stage: None,
            b1,
            b2,
            base,
            p_minus_1: None,
            p_minus_1_factors: None,
            p_minus_1_expression: None,
            steps: 0,
            message: format!("n = {} is already prime", n),
            elapsed_us: start.elapsed().as_micros(),
        });
    }

    let n128 = n as i128;

    let g0 = egcd(base as i128, n128).0 as i64;
    if g0 > 1 && g0 < n {
        let p = g0.min(n / g0);
        let q = n / p;
        let p_minus_1 = p - 1;
        let factors = factorize(p_minus_1).ok();
        let expr = factors.as_ref().map(|f| format_factorization(f));
        return Ok(PollardPm1Result {
            n,
            factor: Some(p),
            cofactor: Some(q),
            success: true,
            stage: Some(0),
            b1,
            b2,
            base,
            p_minus_1: Some(p_minus_1),
            p_minus_1_factors: factors,
            p_minus_1_expression: expr,
            steps: 0,
            message: format!("Base {} shares non-trivial factor {} with n", base, p),
            elapsed_us: start.elapsed().as_micros(),
        });
    }

    let mut steps = 0usize;
    let primes = sieve_primes(b2 as usize);
    let mut x = base as i128;

    // Stage 1
    for &q in &primes {
        if q as u64 > b1 {
            break;
        }
        steps += 1;
        let mut pk = q as u64;
        let mut k = 1;
        while pk.saturating_mul(q as u64) <= b1 {
            pk *= q as u64;
            k += 1;
        }

        let prev_x = x;
        x = mod_pow(x, pk as i128, n128);
        let g = egcd((x - 1).rem_euclid(n128), n128).0 as i64;

        if g > 1 && g < n {
            let p = g.min(n / g);
            let q_factor = n / p;
            let p_minus_1 = p - 1;
            let factors = factorize(p_minus_1).ok();
            let expr = factors.as_ref().map(|f| format_factorization(f));
            return Ok(PollardPm1Result {
                n,
                factor: Some(p),
                cofactor: Some(q_factor),
                success: true,
                stage: Some(1),
                b1,
                b2,
                base,
                p_minus_1: Some(p_minus_1),
                p_minus_1_factors: factors,
                p_minus_1_expression: expr,
                steps,
                message: format!(
                    "Found non-trivial factor p = {} in Stage 1 at prime {}",
                    p, q
                ),
                elapsed_us: start.elapsed().as_micros(),
            });
        }

        if g == n {
            let mut curr = prev_x;
            for _ in 0..k {
                curr = mod_pow(curr, q as i128, n128);
                let g_sub = egcd((curr - 1).rem_euclid(n128), n128).0 as i64;
                if g_sub > 1 && g_sub < n {
                    let p = g_sub.min(n / g_sub);
                    let q_factor = n / p;
                    let p_minus_1 = p - 1;
                    let factors = factorize(p_minus_1).ok();
                    let expr = factors.as_ref().map(|f| format_factorization(f));
                    return Ok(PollardPm1Result {
                        n,
                        factor: Some(p),
                        cofactor: Some(q_factor),
                        success: true,
                        stage: Some(1),
                        b1,
                        b2,
                        base,
                        p_minus_1: Some(p_minus_1),
                        p_minus_1_factors: factors,
                        p_minus_1_expression: expr,
                        steps,
                        message: format!(
                            "Found non-trivial factor p = {} in Stage 1 backtrack at prime {}",
                            p, q
                        ),
                        elapsed_us: start.elapsed().as_micros(),
                    });
                }
                if g_sub == n {
                    break;
                }
            }
            return Ok(PollardPm1Result {
                n,
                factor: None,
                cofactor: None,
                success: false,
                stage: Some(1),
                b1,
                b2,
                base,
                p_minus_1: None,
                p_minus_1_factors: None,
                p_minus_1_expression: None,
                steps,
                message: format!("Stage 1 failed: all factors became congruent to 1 mod n at prime {}. Try a smaller B1 or different base.", q),
                elapsed_us: start.elapsed().as_micros(),
            });
        }
    }

    // Stage 2
    if b2 > b1 {
        let s2_primes: Vec<usize> = primes
            .into_iter()
            .filter(|&p| p as u64 > b1 && p as u64 <= b2)
            .collect();
        if !s2_primes.is_empty() {
            let a0 = x;
            let q1 = s2_primes[0];
            let mut cur_x = mod_pow(a0, q1 as i128, n128);
            steps += 1;
            let g = egcd((cur_x - 1).rem_euclid(n128), n128).0 as i64;
            if g > 1 && g < n {
                let p = g.min(n / g);
                let q_factor = n / p;
                let p_minus_1 = p - 1;
                let factors = factorize(p_minus_1).ok();
                let expr = factors.as_ref().map(|f| format_factorization(f));
                return Ok(PollardPm1Result {
                    n,
                    factor: Some(p),
                    cofactor: Some(q_factor),
                    success: true,
                    stage: Some(2),
                    b1,
                    b2,
                    base,
                    p_minus_1: Some(p_minus_1),
                    p_minus_1_factors: factors,
                    p_minus_1_expression: expr,
                    steps,
                    message: format!(
                        "Found non-trivial factor p = {} in Stage 2 at prime {}",
                        p, q1
                    ),
                    elapsed_us: start.elapsed().as_micros(),
                });
            }

            let mut a0_d: std::collections::HashMap<usize, i128> = std::collections::HashMap::new();

            for i in 0..s2_primes.len() - 1 {
                let d = s2_primes[i + 1] - s2_primes[i];
                let mult = *a0_d
                    .entry(d)
                    .or_insert_with(|| mod_pow(a0, d as i128, n128));
                cur_x = (cur_x * mult).rem_euclid(n128);
                steps += 1;
                let g = egcd((cur_x - 1).rem_euclid(n128), n128).0 as i64;
                if g > 1 && g < n {
                    let p = g.min(n / g);
                    let q_factor = n / p;
                    let p_minus_1 = p - 1;
                    let factors = factorize(p_minus_1).ok();
                    let expr = factors.as_ref().map(|f| format_factorization(f));
                    return Ok(PollardPm1Result {
                        n,
                        factor: Some(p),
                        cofactor: Some(q_factor),
                        success: true,
                        stage: Some(2),
                        b1,
                        b2,
                        base,
                        p_minus_1: Some(p_minus_1),
                        p_minus_1_factors: factors,
                        p_minus_1_expression: expr,
                        steps,
                        message: format!(
                            "Found non-trivial factor p = {} in Stage 2 at prime {}",
                            p,
                            s2_primes[i + 1]
                        ),
                        elapsed_us: start.elapsed().as_micros(),
                    });
                }
            }
        }
    }

    Ok(PollardPm1Result {
        n,
        factor: None,
        cofactor: None,
        success: false,
        stage: None,
        b1,
        b2,
        base,
        p_minus_1: None,
        p_minus_1_factors: None,
        p_minus_1_expression: None,
        steps,
        message: format!("No non-trivial factor found with B1 = {}, B2 = {}. Try increasing bounds or changing base.", b1, b2),
        elapsed_us: start.elapsed().as_micros(),
    })
}

pub fn euler_phi(n: i64) -> Result<i64, MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if n == 1 {
        return Ok(1);
    }
    let factors = factorize(n)?;
    let mut result = 1i64;
    for f in factors {
        let mut p_pow = 1i64;
        for _ in 0..(f.power - 1) {
            p_pow = p_pow
                .checked_mul(f.prime)
                .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
        }
        let term = p_pow
            .checked_mul(f.prime - 1)
            .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
        result = result
            .checked_mul(term)
            .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
    }
    Ok(result)
}

pub fn carmichael_lambda(n: i64) -> Result<i64, MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if n == 1 {
        return Ok(1);
    }
    let factors = factorize(n)?;
    let mut lam = 1i64;
    for f in factors {
        let pk_lam = if f.prime == 2 {
            if f.power == 1 {
                1i64
            } else if f.power == 2 {
                2i64
            } else {
                1i64.checked_shl(f.power - 2)
                    .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?
            }
        } else {
            let mut p_pow = 1i64;
            for _ in 0..(f.power - 1) {
                p_pow = p_pow
                    .checked_mul(f.prime)
                    .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
            }
            p_pow
                .checked_mul(f.prime - 1)
                .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?
        };
        let (g, _, _) = egcd(lam as i128, pk_lam as i128);
        let g = g as i64;
        lam = (lam / g)
            .checked_mul(pk_lam)
            .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
    }
    Ok(lam)
}

#[allow(dead_code)]
pub fn is_cyclic(n: i64) -> Result<bool, MathError> {
    let phi = euler_phi(n)?;
    let lam = carmichael_lambda(n)?;
    Ok(phi == lam)
}

pub fn multiplicative_order(a: i64, m: i64) -> Result<i64, MathError> {
    if m <= 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Modulus m must be greater than 1",
        ));
    }
    let a_mod = a.rem_euclid(m);
    let (g, _, _) = egcd(a_mod as i128, m as i128);
    if g != 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "a and m are not coprime",
        ));
    }
    if a_mod == 1 {
        return Ok(1);
    }
    let lam = carmichael_lambda(m)?;
    let mut order = lam;
    let factors = factorize(lam)?;
    for f in factors {
        let p = f.prime;
        while order % p == 0 && mod_pow(a_mod as i128, (order / p) as i128, m as i128) == 1 {
            order /= p;
        }
    }
    Ok(order)
}

pub fn is_primitive_root(a: i64, m: i64) -> Result<bool, MathError> {
    if m <= 1 {
        return Ok(false);
    }
    let a_mod = a.rem_euclid(m);
    let (g, _, _) = egcd(a_mod as i128, m as i128);
    if g != 1 {
        return Ok(false);
    }
    let order = multiplicative_order(a, m)?;
    let phi = euler_phi(m)?;
    Ok(order == phi)
}

// ------------------- Discrete Logarithm (BSGS) -------------------

pub fn discrete_log(g: i64, h: i64, p: i64) -> Result<i64, MathError> {
    if p <= 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Modulus p must be greater than 1",
        ));
    }
    let p128 = p as i128;
    let g128 = (g as i128).rem_euclid(p128);
    let h128 = (h as i128).rem_euclid(p128);

    if h128 == 1 {
        return Ok(0);
    }

    let (d, _, _) = egcd(g128, p128);
    if d != 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Base g and modulus p must be coprime",
        ));
    }

    let order = multiplicative_order(g, p)?;
    let mut m = (order as f64).sqrt() as i64;
    if m * m < order {
        m += 1;
    }

    let mut table = std::collections::HashMap::with_capacity(m as usize);
    let mut cur = 1i128;
    for j in 0..m {
        table.entry(cur as i64).or_insert(j);
        cur = (cur * g128).rem_euclid(p128);
    }

    let gm = mod_pow(g128, m as i128, p128);
    let factor =
        mod_inv(gm, p128).ok_or(MathError(StatusCode::BAD_REQUEST, "modular inverse failed"))?;

    let mut gamma = h128;
    let steps = (order + m - 1) / m;
    for i in 0..=steps {
        if let Some(&j) = table.get(&(gamma as i64)) {
            let ans = i * m + j;
            if ans < order {
                return Ok(ans);
            }
        }
        gamma = (gamma * factor).rem_euclid(p128);
    }

    Err(MathError(
        StatusCode::BAD_REQUEST,
        "No discrete logarithm exists",
    ))
}

// ------------------- Arithmetic Functions -------------------

fn mobius_from_factors(factors: &[PrimeFactor]) -> i64 {
    for f in factors {
        if f.power > 1 {
            return 0;
        }
    }
    if factors.len() % 2 == 1 {
        -1
    } else {
        1
    }
}

fn tau_from_factors(factors: &[PrimeFactor]) -> Result<i64, MathError> {
    let mut total = 1i64;
    for f in factors {
        total = total
            .checked_mul((f.power + 1) as i64)
            .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
    }
    Ok(total)
}

fn sigma_from_factors(factors: &[PrimeFactor], k: u32) -> Result<i64, MathError> {
    if k == 0 {
        return tau_from_factors(factors);
    }
    let mut total = 1i64;
    for f in factors {
        let mut term = 1i128;
        let mut cur = 1i128;
        let p_pow_k = (f.prime as i128)
            .checked_pow(k)
            .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
        for _ in 0..f.power {
            cur = cur
                .checked_mul(p_pow_k)
                .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
            term = term
                .checked_add(cur)
                .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
        }
        if term > i64::MAX as i128 {
            return Err(MathError(StatusCode::BAD_REQUEST, "integer overflow"));
        }
        total = total
            .checked_mul(term as i64)
            .ok_or(MathError(StatusCode::BAD_REQUEST, "integer overflow"))?;
    }
    Ok(total)
}

pub fn arithmetic_suite(n: i64, k: u32) -> Result<(i64, i64, i64), MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if n == 1 {
        return Ok((1, 1, 1));
    }
    let factors = factorize(n)?;
    let mu = mobius_from_factors(&factors);
    let sig = sigma_from_factors(&factors, k)?;
    let t = tau_from_factors(&factors)?;
    Ok((mu, sig, t))
}

pub fn mobius(n: i64) -> Result<i64, MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if n == 1 {
        return Ok(1);
    }
    let factors = factorize(n)?;
    Ok(mobius_from_factors(&factors))
}

pub fn tau(n: i64) -> Result<i64, MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if n == 1 {
        return Ok(1);
    }
    let factors = factorize(n)?;
    tau_from_factors(&factors)
}

pub fn sigma(n: i64, k: u32) -> Result<i64, MathError> {
    if n < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "n must be a positive integer >= 1",
        ));
    }
    if n == 1 {
        return Ok(1);
    }
    let factors = factorize(n)?;
    sigma_from_factors(&factors, k)
}

// ------------------- Random Number Generator -------------------

#[derive(Clone, Debug)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    pub fn gen_range(&mut self, low: i64, high: i64) -> i64 {
        if low >= high {
            return low;
        }
        let range = (high - low + 1) as u64;
        let val = self.next_u64() % range;
        low + val as i64
    }

    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        let idx = (self.next_u64() as usize) % slice.len();
        &slice[idx]
    }
}

// ------------------- Cryptosystems (RSA & Diffie-Hellman) -------------------

pub fn generate_prime(bits: u32, rng: &mut SimpleRng) -> Result<i64, MathError> {
    if bits < 2 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Prime bit length must be at least 2",
        ));
    }
    if bits > 62 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Prime bit length must be at most 62",
        ));
    }
    if bits == 2 {
        return Ok(if rng.gen_range(0, 1) == 0 { 2 } else { 3 });
    }
    let low = 1i64 << (bits - 1);
    let high = (1i64 << bits) - 1;
    for _ in 0..10_000 {
        let mut val = rng.gen_range(low, high);
        val |= (1i64 << (bits - 1)) | 1;
        if is_prime(val) {
            return Ok(val);
        }
    }
    Err(MathError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to find prime within iteration limit",
    ))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RsaKeypair {
    pub p: i64,
    pub q: i64,
    pub n: i64,
    pub e: i64,
    pub d: i64,
    pub phi: i64,
    pub lambda: i64,
    pub dp: i64,
    pub dq: i64,
    pub qinv: i64,
}

pub fn rsa_keygen(bits: u32, e: i64, rng: &mut SimpleRng) -> Result<RsaKeypair, MathError> {
    if bits < 8 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "RSA modulus bit length must be at least 8",
        ));
    }
    if bits > 62 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "RSA modulus bit length must be at most 62",
        ));
    }
    if e < 3 || e % 2 == 0 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Public exponent e must be an odd integer >= 3",
        ));
    }

    let p_bits = bits / 2;
    let q_bits = bits - p_bits;

    for _ in 0..5000 {
        let p_cand = generate_prime(p_bits, rng)?;
        let q_cand = generate_prime(q_bits, rng)?;
        if p_cand == q_cand {
            continue;
        }
        let (p, q) = if p_cand < q_cand {
            (q_cand, p_cand)
        } else {
            (p_cand, q_cand)
        };
        let n = p.checked_mul(q).ok_or(MathError(
            StatusCode::BAD_REQUEST,
            "modulus n integer overflow",
        ))?;
        let phi = (p - 1).checked_mul(q - 1).ok_or(MathError(
            StatusCode::BAD_REQUEST,
            "totient phi integer overflow",
        ))?;
        let (g, _, _) = egcd((p - 1) as i128, (q - 1) as i128);
        let lam = phi / (g as i64);

        let (ge, _, _) = egcd(e as i128, lam as i128);
        if ge == 1 {
            let d = mod_inv(e as i128, lam as i128).ok_or(MathError(
                StatusCode::BAD_REQUEST,
                "e has no inverse mod lambda",
            ))? as i64;
            let dp = (d as i128).rem_euclid((p - 1) as i128) as i64;
            let dq = (d as i128).rem_euclid((q - 1) as i128) as i64;
            let qinv = mod_inv(q as i128, p as i128)
                .ok_or(MathError(StatusCode::BAD_REQUEST, "q has no inverse mod p"))?
                as i64;
            return Ok(RsaKeypair {
                p,
                q,
                n,
                e,
                d,
                phi,
                lambda: lam,
                dp,
                dq,
                qinv,
            });
        }
    }

    Err(MathError(
        StatusCode::BAD_REQUEST,
        "Failed to generate RSA key with given parameters",
    ))
}

pub fn rsa_encrypt(m: i64, e: i64, n: i64) -> Result<i64, MathError> {
    if n <= 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Modulus n must be greater than 1",
        ));
    }
    if e < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent e must be >= 1",
        ));
    }
    if m < 0 || m >= n {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Message m must satisfy 0 <= m < n",
        ));
    }
    Ok(mod_pow(m as i128, e as i128, n as i128) as i64)
}

pub fn rsa_decrypt(
    c: i64,
    d: i64,
    n: i64,
    p: Option<i64>,
    q: Option<i64>,
) -> Result<i64, MathError> {
    if n <= 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Modulus n must be greater than 1",
        ));
    }
    if d < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent d must be >= 1",
        ));
    }
    if c < 0 || c >= n {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Ciphertext c must satisfy 0 <= c < n",
        ));
    }
    if p.is_some() != q.is_some() {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Both p and q must be provided for CRT acceleration, or neither",
        ));
    }

    if let (Some(p_val), Some(q_val)) = (p, q) {
        if p_val <= 1 || q_val <= 1 {
            return Err(MathError(
                StatusCode::BAD_REQUEST,
                "Factors p and q must be greater than 1",
            ));
        }
        if (p_val as i128) * (q_val as i128) != n as i128 {
            return Err(MathError(
                StatusCode::BAD_REQUEST,
                "Invalid factors: p * q != n",
            ));
        }
        let p128 = p_val as i128;
        let q128 = q_val as i128;
        let dp = (d as i128).rem_euclid(p128 - 1);
        let dq = (d as i128).rem_euclid(q128 - 1);
        let qinv = mod_inv(q128, p128).ok_or(MathError(
            StatusCode::BAD_REQUEST,
            "q has no modular inverse mod p",
        ))?;
        let m1 = mod_pow(c as i128, dp, p128);
        let m2 = mod_pow(c as i128, dq, q128);
        let h = (qinv * (m1 - m2)).rem_euclid(p128);
        let m = m2 + h * q128;
        return Ok(m as i64);
    }

    Ok(mod_pow(c as i128, d as i128, n as i128) as i64)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RsaCrackResult {
    pub n: i64,
    pub e: i64,
    pub p: i64,
    pub q: i64,
    pub phi: i64,
    pub lambda: i64,
    pub d: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m: Option<i64>,
}

pub fn rsa_crack(n: i64, e: i64, c: Option<i64>) -> Result<RsaCrackResult, MathError> {
    if n <= 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Modulus n must be greater than 1",
        ));
    }
    if e < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent e must be >= 1",
        ));
    }
    if let Some(c_val) = c {
        if c_val < 0 || c_val >= n {
            return Err(MathError(
                StatusCode::BAD_REQUEST,
                "Ciphertext c must satisfy 0 <= c < n",
            ));
        }
    }

    let factors = factorize(n)?;
    let mut primes = Vec::new();
    for f in &factors {
        for _ in 0..f.power {
            primes.push(f.prime);
        }
    }
    if primes.len() != 2 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Modulus n must be a product of exactly two primes",
        ));
    }
    let (mut p, mut q) = (primes[0], primes[1]);
    if p < q {
        std::mem::swap(&mut p, &mut q);
    }
    let phi = (p - 1) * (q - 1);
    let (g, _, _) = egcd((p - 1) as i128, (q - 1) as i128);
    let lam = phi / (g as i64);
    let (ge, _, _) = egcd(e as i128, lam as i128);
    if ge != 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Cannot invert e modulo lambda(n)",
        ));
    }
    let d = mod_inv(e as i128, lam as i128).ok_or(MathError(
        StatusCode::BAD_REQUEST,
        "Cannot invert e modulo lambda(n)",
    ))? as i64;
    let m = if let Some(c_val) = c {
        Some(rsa_decrypt(c_val, d, n, Some(p), Some(q))?)
    } else {
        None
    };
    Ok(RsaCrackResult {
        n,
        e,
        p,
        q,
        phi,
        lambda: lam,
        d,
        c,
        m,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DhKeypair {
    pub p: i64,
    pub g: i64,
    pub private_key: i64,
    pub public_key: i64,
}

pub fn dh_keygen(
    p: i64,
    g: i64,
    private_key: Option<i64>,
    rng: &mut SimpleRng,
) -> Result<DhKeypair, MathError> {
    if p <= 2 || !is_prime(p) {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Diffie-Hellman modulus p must be a prime > 2",
        ));
    }
    if g <= 1 || g >= p {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Generator g must satisfy 1 < g < p",
        ));
    }
    let a = if let Some(priv_k) = private_key {
        if priv_k < 2 || priv_k >= p - 1 {
            return Err(MathError(
                StatusCode::BAD_REQUEST,
                "private_key must satisfy 2 <= private_key <= p-2",
            ));
        }
        priv_k
    } else {
        rng.gen_range(2, p - 2)
    };
    let public_key = mod_pow(g as i128, a as i128, p as i128) as i64;
    Ok(DhKeypair {
        p,
        g,
        private_key: a,
        public_key,
    })
}

pub fn dh_derive(private_key: i64, peer_public_key: i64, p: i64) -> Result<i64, MathError> {
    if p <= 2 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Modulus p must be greater than 2",
        ));
    }
    if private_key < 1 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "private_key must be >= 1",
        ));
    }
    if peer_public_key < 1 || peer_public_key >= p {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "peer_public_key must satisfy 1 <= key < p",
        ));
    }
    Ok(mod_pow(peer_public_key as i128, private_key as i128, p as i128) as i64)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DhCrackResult {
    pub g: i64,
    pub public_key: i64,
    pub p: i64,
    pub private_key: i64,
}

pub fn dh_crack(g: i64, public_key: i64, p: i64) -> Result<DhCrackResult, MathError> {
    let private_key = discrete_log(g, public_key, p)?;
    Ok(DhCrackResult {
        g,
        public_key,
        p,
        private_key,
    })
}

// ------------------- Challenge / Quiz Generator -------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChallengeProblem {
    pub topic: String,
    pub question: String,
    pub answer: i64,
}

pub fn generate_problem(topic: &str, rng: &mut SimpleRng) -> Result<ChallengeProblem, MathError> {
    let selected_topic = if topic == "mixed" {
        let choices = [
            "legendre", "jacobi", "sqrtmod", "crt", "ecdlp", "factor", "totient", "dlog", "rsa",
            "dh", "mersenne", "perfect", "pm1",
        ];
        *rng.choose(&choices)
    } else {
        topic
    };

    match selected_topic {
        "legendre" => {
            let primes = [7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
            let p = *rng.choose(&primes);
            let a = rng.gen_range(2, p - 1);
            let ans = legendre_symbol(a, p)? as i64;
            Ok(ChallengeProblem {
                topic: "legendre".to_string(),
                question: f!("Legendre symbol ({a}/{p})"),
                answer: ans,
            })
        }
        "sqrtmod" => {
            let primes = [11, 13, 17, 19, 23, 29, 31, 37, 41, 43];
            let p = *rng.choose(&primes);
            let root = rng.gen_range(2, (p - 1) / 2);
            let a = (root * root) % p;
            Ok(ChallengeProblem {
                topic: "sqrtmod".to_string(),
                question: f!("Smaller positive root of x² ≡ {a} (mod {p})"),
                answer: root,
            })
        }
        "crt" => {
            let m1 = 3i64;
            let m2 = 5i64;
            let m3 = 7i64;
            let mod_product = m1 * m2 * m3;
            let x = rng.gen_range(1, mod_product - 1);
            let r1 = x % m1;
            let r2 = x % m2;
            let r3 = x % m3;
            Ok(ChallengeProblem {
                topic: "crt".to_string(),
                question: f!("Find x (0 ≤ x < {mod_product}): x ≡ {r1} (mod {m1}), x ≡ {r2} (mod {m2}), x ≡ {r3} (mod {m3})"),
                answer: x,
            })
        }
        "ecdlp" => {
            let preset = get_curve_preset("toy193").unwrap();
            let k = rng.gen_range(2, 20);
            let g = Point {
                x: preset.gx,
                y: preset.gy,
            };
            let q = ec_mul(k, g, preset.a, preset.p)?
                .expect("point on toy193 cannot be infinity for k in 2..20");
            Ok(ChallengeProblem {
                topic: "ecdlp".to_string(),
                question: f!("On y² = x³ + {a}x + {b} (mod {p}) with G=({gx}, {gy}): find k (2 ≤ k ≤ 20) such that k·G = ({qx}, {qy})",
                    a = preset.a, b = preset.b, p = preset.p, gx = preset.gx, gy = preset.gy, qx = q.x, qy = q.y),
                answer: k,
            })
        }
        "factor" => {
            let primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
            let count = *rng.choose(&[2, 2, 2, 3]);
            let mut chosen = Vec::with_capacity(count);
            for _ in 0..count {
                chosen.push(*rng.choose(&primes));
            }
            let n: i64 = chosen.iter().product();
            let min_p = *chosen.iter().min().unwrap();
            Ok(ChallengeProblem {
                topic: "factor".to_string(),
                question: f!("Find the smallest prime factor of {n}"),
                answer: min_p,
            })
        }
        "totient" => {
            let nums = [
                6, 8, 9, 10, 12, 14, 15, 16, 18, 20, 21, 22, 24, 25, 26, 27, 28, 30, 35, 36, 40,
                42, 45, 49, 50, 54, 60, 72, 84, 100,
            ];
            let n = *rng.choose(&nums);
            let ans = euler_phi(n)?;
            Ok(ChallengeProblem {
                topic: "totient".to_string(),
                question: f!("Compute Euler's totient phi({n})"),
                answer: ans,
            })
        }
        "dlog" => {
            let problems = [
                (2i64, 13i64),
                (3, 17),
                (2, 19),
                (2, 29),
                (3, 31),
                (2, 37),
                (3, 41),
                (3, 43),
                (2, 53),
                (2, 59),
            ];
            let &(g, p) = rng.choose(&problems);
            let order = multiplicative_order(g, p)?;
            let x = rng.gen_range(1, order - 1);
            let h = mod_pow(g as i128, x as i128, p as i128) as i64;
            Ok(ChallengeProblem {
                topic: "dlog".to_string(),
                question: f!("Find x (0 ≤ x < {order}) such that {g}^x ≡ {h} (mod {p})"),
                answer: x,
            })
        }
        "jacobi" => {
            let moduli = [
                9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31, 33, 35, 39, 45, 51, 55, 63, 65, 77,
                85, 91, 99,
            ];
            let n = *rng.choose(&moduli);
            let a = rng.gen_range(1, n - 1);
            let ans = jacobi_symbol(a, n)? as i64;
            Ok(ChallengeProblem {
                topic: "jacobi".to_string(),
                question: f!("Jacobi symbol ({a}/{n})"),
                answer: ans,
            })
        }
        "rsa" => {
            let primes = [3i64, 5, 7, 11, 13, 17, 19, 23];
            let mut p = *rng.choose(&primes);
            let mut q = *rng.choose(&primes);
            while p == q {
                q = *rng.choose(&primes);
            }
            if p < q {
                std::mem::swap(&mut p, &mut q);
            }
            let n = p * q;
            let phi = (p - 1) * (q - 1);
            let e_candidates = [3i64, 5, 7, 11, 13, 17];
            let valid_e: Vec<i64> = e_candidates
                .iter()
                .copied()
                .filter(|&cand| {
                    let (g, _, _) = egcd(cand as i128, phi as i128);
                    g == 1
                })
                .collect();
            let e = *rng.choose(&valid_e);
            let m_max = (n - 1).min(15);
            let m = rng.gen_range(2, m_max);
            let c = mod_pow(m as i128, e as i128, n as i128) as i64;
            Ok(ChallengeProblem {
                topic: "rsa".to_string(),
                question: f!("In RSA with e = {e} and n = {n}, find ciphertext c for message m = {m} (c ≡ m^{e} mod {n})"),
                answer: c,
            })
        }
        "dh" => {
            let dh_params = [
                (11i64, 2i64),
                (13, 2),
                (17, 3),
                (19, 2),
                (23, 5),
                (29, 2),
                (31, 3),
                (41, 6),
                (43, 3),
            ];
            let &(p, g) = rng.choose(&dh_params);
            let a = rng.gen_range(2, 6);
            if rng.gen_range(0, 1) == 0 {
                let pub_a = mod_pow(g as i128, a as i128, p as i128) as i64;
                Ok(ChallengeProblem {
                    topic: "dh".to_string(),
                    question: f!("In Diffie-Hellman with p = {p} and generator g = {g}, find public key A for private key a = {a} (A ≡ {g}^{a} mod {p})"),
                    answer: pub_a,
                })
            } else {
                let b = rng.gen_range(2, 6);
                let pub_b = mod_pow(g as i128, b as i128, p as i128) as i64;
                let s = mod_pow(pub_b as i128, a as i128, p as i128) as i64;
                Ok(ChallengeProblem {
                    topic: "dh".to_string(),
                    question: f!("In Diffie-Hellman with p = {p}, Alice's private key is a = {a} and Bob's public key is B = {pub_b}. Find shared secret S (S ≡ {pub_b}^{a} mod {p})"),
                    answer: s,
                })
            }
        }
        "mersenne" => {
            let candidates = [2, 3, 5, 7, 11, 13, 17, 19, 23, 31];
            let &p = rng.choose(&candidates);
            if rng.gen_range(0, 1) == 0 {
                let ans = if lucas_lehmer(p).unwrap_or(false) { 1 } else { 0 };
                Ok(ChallengeProblem {
                    topic: "mersenne".to_string(),
                    question: f!("Is Mersenne number M_{p} = 2^{p} - 1 prime? (1 for Yes, 0 for No)"),
                    answer: ans,
                })
            } else {
                let small_candidates = [2, 3, 5, 7, 13];
                let &sp = rng.choose(&small_candidates);
                let mp = (1i64 << sp) - 1;
                Ok(ChallengeProblem {
                    topic: "mersenne".to_string(),
                    question: f!("Compute the value of Mersenne number M_{sp} = 2^{sp} - 1"),
                    answer: mp,
                })
            }
        }
        "perfect" => {
            let candidates = [6, 12, 28, 40, 496, 500, 8128];
            let &n = rng.choose(&candidates);
            let ans = if is_perfect(n) { 1 } else { 0 };
            Ok(ChallengeProblem {
                topic: "perfect".to_string(),
                question: f!("Is {n} a perfect number (sum of proper divisors equals n)? (1 for Yes, 0 for No)"),
                answer: ans,
            })
        }
        "pm1" => {
            let smooth_primes = [
                29, 37, 41, 73, 97, 101, 109, 113, 137, 181, 193, 241, 257, 337, 433, 577, 601,
                641, 769, 1009,
            ];
            let other_primes = [
                103, 107, 127, 131, 139, 149, 151, 157, 163, 167, 173, 179,
            ];
            let p = *rng.choose(&smooth_primes);
            let q = *rng.choose(&other_primes);
            let n = p * q;
            let ans = p.min(q);
            Ok(ChallengeProblem {
                topic: "pm1".to_string(),
                question: f!("Smallest prime factor of composite n = {n} (hint: p - 1 is smooth)"),
                answer: ans,
            })
        }
        _ => Err(MathError(
            StatusCode::BAD_REQUEST,
            "topic must be legendre, jacobi, sqrtmod, crt, ecdlp, factor, totient, dlog, rsa, dh, mersenne, perfect, pm1, or mixed",
        )),
    }
}

// ------------------- Mersenne Numbers & Lucas-Lehmer Primality Test -------------------

use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BigUint {
    pub digits: Vec<u32>, // Little-endian base 2^32, trimmed
}

impl Ord for BigUint {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.digits.len() != other.digits.len() {
            return self.digits.len().cmp(&other.digits.len());
        }
        for (a, b) in self.digits.iter().rev().zip(other.digits.iter().rev()) {
            if a != b {
                return a.cmp(b);
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for BigUint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl BigUint {
    pub fn zero() -> Self {
        BigUint { digits: Vec::new() }
    }

    pub fn from_u64(mut val: u64) -> Self {
        let mut digits = Vec::new();
        while val > 0 {
            digits.push((val & 0xFFFF_FFFF) as u32);
            val >>= 32;
        }
        BigUint { digits }
    }

    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    pub fn trim(&mut self) {
        while let Some(&0) = self.digits.last() {
            self.digits.pop();
        }
    }

    pub fn mersenne(p: u32) -> Self {
        if p == 0 {
            return Self::zero();
        }
        let full_limbs = (p / 32) as usize;
        let rem_bits = p % 32;
        let mut digits = vec![0xFFFF_FFFFu32; full_limbs];
        if rem_bits > 0 {
            digits.push(((1u64 << rem_bits) - 1) as u32);
        }
        let mut res = BigUint { digits };
        res.trim();
        res
    }

    pub fn perfect_number(p: u32) -> Self {
        let m = Self::mersenne(p);
        m.shl((p - 1) as usize)
    }

    pub fn add(&self, other: &Self) -> Self {
        let max_len = self.digits.len().max(other.digits.len());
        let mut res = Vec::with_capacity(max_len + 1);
        let mut carry = 0u64;
        for i in 0..max_len {
            let a = self.digits.get(i).copied().unwrap_or(0) as u64;
            let b = other.digits.get(i).copied().unwrap_or(0) as u64;
            let sum = a + b + carry;
            res.push((sum & 0xFFFF_FFFF) as u32);
            carry = sum >> 32;
        }
        if carry > 0 {
            res.push(carry as u32);
        }
        let mut r = BigUint { digits: res };
        r.trim();
        r
    }

    pub fn sub(&self, other: &Self) -> Option<Self> {
        if self < other {
            return None;
        }
        let mut res = Vec::with_capacity(self.digits.len());
        let mut borrow = 0i64;
        for i in 0..self.digits.len() {
            let a = self.digits[i] as i64;
            let b = other.digits.get(i).copied().unwrap_or(0) as i64;
            let diff = a - b - borrow;
            if diff < 0 {
                res.push((diff + 0x1_0000_0000) as u32);
                borrow = 1;
            } else {
                res.push(diff as u32);
                borrow = 0;
            }
        }
        let mut r = BigUint { digits: res };
        r.trim();
        Some(r)
    }

    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let mut res = vec![0u32; self.digits.len() + other.digits.len()];
        for (i, &a) in self.digits.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in other.digits.iter().enumerate() {
                let cur = res[i + j] as u64 + (a as u64) * (b as u64) + carry;
                res[i + j] = (cur & 0xFFFF_FFFF) as u32;
                carry = cur >> 32;
            }
            if carry > 0 {
                res[i + other.digits.len()] += carry as u32;
            }
        }
        let mut r = BigUint { digits: res };
        r.trim();
        r
    }

    pub fn shl(&self, shift: usize) -> Self {
        if self.is_zero() || shift == 0 {
            return self.clone();
        }
        let limb_shift = shift / 32;
        let bit_shift = shift % 32;
        let mut res = vec![0u32; limb_shift];
        let mut carry = 0u64;
        for &d in &self.digits {
            let cur = ((d as u64) << bit_shift) | carry;
            res.push((cur & 0xFFFF_FFFF) as u32);
            carry = cur >> 32;
        }
        if carry > 0 {
            res.push(carry as u32);
        }
        let mut r = BigUint { digits: res };
        r.trim();
        r
    }

    pub fn shr(&self, shift: usize) -> Self {
        if self.is_zero() || shift == 0 {
            return self.clone();
        }
        let limb_shift = shift / 32;
        let bit_shift = shift % 32;
        if limb_shift >= self.digits.len() {
            return Self::zero();
        }
        let sliced = &self.digits[limb_shift..];
        if bit_shift == 0 {
            let mut r = BigUint {
                digits: sliced.to_vec(),
            };
            r.trim();
            return r;
        }
        let mut res = Vec::with_capacity(sliced.len());
        for i in 0..sliced.len() {
            let cur = sliced[i] as u64 >> bit_shift;
            let next = if i + 1 < sliced.len() {
                (sliced[i + 1] as u64) << (32 - bit_shift)
            } else {
                0
            };
            res.push(((cur | next) & 0xFFFF_FFFF) as u32);
        }
        let mut r = BigUint { digits: res };
        r.trim();
        r
    }

    pub fn mask_bits(&self, bits: usize) -> Self {
        if self.is_zero() || bits == 0 {
            return Self::zero();
        }
        let limb_count = bits / 32;
        let rem_bits = bits % 32;
        let mut res = Vec::new();
        for i in 0..limb_count {
            if i < self.digits.len() {
                res.push(self.digits[i]);
            }
        }
        if rem_bits > 0 && limb_count < self.digits.len() {
            let mask = ((1u64 << rem_bits) - 1) as u32;
            res.push(self.digits[limb_count] & mask);
        }
        let mut r = BigUint { digits: res };
        r.trim();
        r
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut temp = self.clone();
        let mut chunks = Vec::new();
        let divisor = 1_000_000_000u64;
        while !temp.is_zero() {
            let mut rem = 0u64;
            for d in temp.digits.iter_mut().rev() {
                let cur = (rem << 32) | (*d as u64);
                *d = (cur / divisor) as u32;
                rem = cur % divisor;
            }
            temp.trim();
            chunks.push(rem);
        }
        let mut s = String::new();
        let last = chunks.pop().unwrap();
        s.push_str(&last.to_string());
        for rem in chunks.into_iter().rev() {
            use std::fmt::Write;
            write!(&mut s, "{:09}", rem).unwrap();
        }
        s
    }
}

#[allow(clippy::manual_is_multiple_of)]
pub fn lucas_lehmer(p: u32) -> Result<bool, MathError> {
    if p < 2 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent p must be an integer >= 2",
        ));
    }
    if p > 5000 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent p exceeds maximum bound of 5000",
        ));
    }
    if p == 2 {
        return Ok(true);
    }
    if p % 2 == 0 || !is_prime(p as i64) {
        return Ok(false);
    }

    let mp = BigUint::mersenne(p);
    let two = BigUint::from_u64(2);
    let mut s = BigUint::from_u64(4);

    for _ in 0..(p - 2) {
        let sq = s.mul(&s);
        let high = sq.shr(p as usize);
        let low = sq.mask_bits(p as usize);
        let mut rem = high.add(&low);
        if rem >= mp {
            rem = rem.sub(&mp).unwrap();
        }
        if rem >= two {
            rem = rem.sub(&two).unwrap();
        } else {
            rem = rem.add(&mp).sub(&two).unwrap();
        }
        if rem >= mp {
            rem = rem.sub(&mp).unwrap();
        }
        s = rem;
    }

    Ok(s.is_zero())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MersenneInfo {
    pub p: u32,
    pub mp: String,
    pub p_is_prime: bool,
    pub is_prime: bool,
    pub bits: u32,
    pub digits: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perfect_number: Option<String>,
}

pub fn mersenne_info(p: u32) -> Result<MersenneInfo, MathError> {
    if p < 2 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent p must be an integer >= 2",
        ));
    }
    if p > 5000 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent p exceeds maximum bound of 5000",
        ));
    }

    let p_is_prime = is_prime(p as i64);
    let is_m_prime = if p_is_prime { lucas_lehmer(p)? } else { false };
    let mp_big = BigUint::mersenne(p);
    let mp_str = mp_big.to_string();
    let digits = mp_str.len();
    let perfect_number = if is_m_prime {
        Some(BigUint::perfect_number(p).to_string())
    } else {
        None
    };

    Ok(MersenneInfo {
        p,
        mp: mp_str,
        p_is_prime,
        is_prime: is_m_prime,
        bits: p,
        digits,
        perfect_number,
    })
}

pub fn mersenne_primes(limit_p: u32) -> Vec<u32> {
    let limit = limit_p.min(5000);
    if limit < 2 {
        return Vec::new();
    }
    let mut primes = Vec::new();
    for p in 2..=limit {
        if is_prime(p as i64) && lucas_lehmer(p).unwrap_or(false) {
            primes.push(p);
        }
    }
    primes
}

pub fn is_perfect(n: i64) -> bool {
    if n <= 1 {
        return false;
    }
    match sigma(n, 1) {
        Ok(s) => s == 2 * n,
        Err(_) => false,
    }
}

pub fn perfect_number(p: u32) -> Result<String, MathError> {
    if p < 2 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent p must be an integer >= 2",
        ));
    }
    if p > 5000 {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Exponent p exceeds maximum bound of 5000",
        ));
    }
    if !lucas_lehmer(p)? {
        return Err(MathError(
            StatusCode::BAD_REQUEST,
            "Mersenne number 2^p - 1 is not prime; no even perfect number corresponds to this exponent",
        ));
    }
    Ok(BigUint::perfect_number(p).to_string())
}

// Simple macro for format string to avoid pulling extra dependencies
macro_rules! f {
    ($($arg:tt)*) => {
        format!($($arg)*)
    };
}
pub(crate) use f;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legendre_symbol() {
        assert_eq!(legendre_symbol(2, 7).unwrap(), 1);
        assert_eq!(legendre_symbol(5, 17).unwrap(), -1);
        assert_eq!(legendre_symbol(0, 17).unwrap(), 0);
        assert!(legendre_symbol(2, 2).is_err());
        assert!(legendre_symbol(2, 1).is_err());
    }

    #[test]
    fn test_jacobi_symbol() {
        assert_eq!(jacobi_symbol(2, 15).unwrap(), 1);
        assert_eq!(jacobi_symbol(10, 21).unwrap(), -1);
        assert_eq!(jacobi_symbol(14, 21).unwrap(), 0);
        assert_eq!(jacobi_symbol(0, 15).unwrap(), 0);
        assert_eq!(jacobi_symbol(0, 1).unwrap(), 1);
        assert_eq!(jacobi_symbol(1, 1).unwrap(), 1);
        assert_eq!(jacobi_symbol(2, 1).unwrap(), 1);
        assert_eq!(jacobi_symbol(-1, 15).unwrap(), -1);
        assert_eq!(jacobi_symbol(2, 7).unwrap(), 1);
        assert_eq!(jacobi_symbol(5, 17).unwrap(), -1);
        assert!(jacobi_symbol(2, 0).is_err());
        assert!(jacobi_symbol(2, -3).is_err());
        assert!(jacobi_symbol(2, 4).is_err());
    }

    #[test]
    fn test_solovay_strassen() {
        let mut rng = SimpleRng::new(42);
        assert!(solovay_strassen(2, 20, &mut rng).unwrap());
        assert!(solovay_strassen(3, 20, &mut rng).unwrap());
        assert!(solovay_strassen(104729, 20, &mut rng).unwrap());
        assert!(!solovay_strassen(1, 20, &mut rng).unwrap());
        assert!(!solovay_strassen(4, 20, &mut rng).unwrap());
        assert!(!solovay_strassen(561, 20, &mut rng).unwrap());
        assert!(!solovay_strassen(1105, 20, &mut rng).unwrap());
        assert!(!solovay_strassen(1729, 20, &mut rng).unwrap());
        assert!(!solovay_strassen(91, 20, &mut rng).unwrap());
        assert!(solovay_strassen(0, 20, &mut rng).is_err());
        assert!(solovay_strassen(7, 0, &mut rng).is_err());
    }

    #[test]
    fn test_sqrtmod() {
        assert_eq!(sqrt_mod(10, 13).unwrap(), vec![6, 7]);
        assert_eq!(sqrt_mod(9, 11).unwrap(), vec![3, 8]);
        assert_eq!(sqrt_mod(0, 17).unwrap(), vec![0]);
        assert_eq!(sqrt_mod(8, 41).unwrap(), vec![7, 34]);
        assert_eq!(sqrt_mod(2, 97).unwrap(), vec![14, 83]);
        assert!(sqrt_mod(5, 17).is_err());
    }

    #[test]
    fn test_crt() {
        let (x, m) = crt_solve(&[2, 3, 2], &[3, 5, 7]).unwrap();
        assert_eq!(x, 23);
        assert_eq!(m, 105);

        assert!(crt_solve(&[1, 2], &[4, 6]).is_err());
    }

    #[test]
    fn test_cf() {
        let (terms, convs) = continued_fraction(355, 113, 10).unwrap();
        assert_eq!(terms, vec![3, 7, 16]);
        assert_eq!(convs, vec![[3, 1], [22, 7], [355, 113]]);
    }

    #[test]
    fn test_elliptic_curves() {
        let preset = get_curve_preset("toy193").unwrap();
        let g = Point {
            x: preset.gx,
            y: preset.gy,
        };
        assert!(is_on_curve(&g, preset.a, preset.b, preset.p));

        // k = 7 gives (66, 24)
        let pt7 = ec_mul(7, g, preset.a, preset.p).unwrap().unwrap();
        assert_eq!(pt7, Point { x: 66, y: 24 });
        assert!(is_on_curve(&pt7, preset.a, preset.b, preset.p));

        // k = 193 (order of group) gives identity (None)
        let pt193 = ec_mul(193, g, preset.a, preset.p).unwrap();
        assert_eq!(pt193, None);
    }

    #[test]
    fn test_factorize() {
        assert_eq!(factorize(1).unwrap(), vec![]);
        assert_eq!(
            factorize(360).unwrap(),
            vec![
                PrimeFactor { prime: 2, power: 3 },
                PrimeFactor { prime: 3, power: 2 },
                PrimeFactor { prime: 5, power: 1 },
            ]
        );
        assert_eq!(
            format_factorization(&factorize(360).unwrap()),
            "2^3 * 3^2 * 5"
        );
        assert_eq!(
            factorize(104729).unwrap(),
            vec![PrimeFactor {
                prime: 104729,
                power: 1
            }]
        );
        assert!(factorize(0).is_err());
        assert!(factorize(-10).is_err());
        // Large semiprime (1000003 * 1000033 = 1000036000099)
        assert_eq!(
            factorize(1000036000099).unwrap(),
            vec![
                PrimeFactor {
                    prime: 1000003,
                    power: 1
                },
                PrimeFactor {
                    prime: 1000033,
                    power: 1
                },
            ]
        );
    }

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(13));
        assert!(!is_prime(561)); // Carmichael number
        assert!(is_prime(104729));
    }

    #[test]
    fn test_challenge_deterministic() {
        let mut rng1 = SimpleRng::new(42);
        let mut rng2 = SimpleRng::new(42);

        let p1 = generate_problem("legendre", &mut rng1).unwrap();
        let p2 = generate_problem("legendre", &mut rng2).unwrap();

        assert_eq!(p1.question, p2.question);
        assert_eq!(p1.answer, p2.answer);

        let f1 = generate_problem("factor", &mut rng1).unwrap();
        assert_eq!(f1.topic, "factor");

        let t1 = generate_problem("totient", &mut rng1).unwrap();
        assert_eq!(t1.topic, "totient");
    }

    #[test]
    fn test_totient_and_order() {
        assert_eq!(euler_phi(1).unwrap(), 1);
        assert_eq!(euler_phi(2).unwrap(), 1);
        assert_eq!(euler_phi(7).unwrap(), 6);
        assert_eq!(euler_phi(360).unwrap(), 96);
        assert!(euler_phi(0).is_err());
        assert!(euler_phi(-10).is_err());

        assert_eq!(carmichael_lambda(1).unwrap(), 1);
        assert_eq!(carmichael_lambda(2).unwrap(), 1);
        assert_eq!(carmichael_lambda(4).unwrap(), 2);
        assert_eq!(carmichael_lambda(8).unwrap(), 2);
        assert_eq!(carmichael_lambda(16).unwrap(), 4);
        assert_eq!(carmichael_lambda(7).unwrap(), 6);
        assert_eq!(carmichael_lambda(360).unwrap(), 12);
        assert!(carmichael_lambda(0).is_err());

        assert!(is_cyclic(1).unwrap());
        assert!(is_cyclic(2).unwrap());
        assert!(is_cyclic(4).unwrap());
        assert!(is_cyclic(7).unwrap());
        assert!(!is_cyclic(8).unwrap());
        assert!(!is_cyclic(360).unwrap());

        assert_eq!(multiplicative_order(3, 7).unwrap(), 6);
        assert_eq!(multiplicative_order(2, 7).unwrap(), 3);
        assert_eq!(multiplicative_order(1, 7).unwrap(), 1);
        assert!(multiplicative_order(0, 7).is_err());
        assert!(multiplicative_order(14, 7).is_err());
        assert!(multiplicative_order(2, 1).is_err());

        assert!(is_primitive_root(3, 7).unwrap());
        assert!(!is_primitive_root(2, 7).unwrap());
        assert!(!is_primitive_root(1, 7).unwrap());
        assert!(!is_primitive_root(3, 8).unwrap());
    }

    #[test]
    fn test_discrete_log() {
        assert_eq!(discrete_log(3, 6, 7).unwrap(), 3);
        assert_eq!(discrete_log(2, 5, 13).unwrap(), 9);
        assert_eq!(discrete_log(2, 1, 13).unwrap(), 0);
        assert_eq!(discrete_log(2, 2, 13).unwrap(), 1);
        assert!(discrete_log(2, 3, 7).is_err());
        assert!(discrete_log(2, 5, 1).is_err());
        assert!(discrete_log(2, 5, 6).is_err());
    }

    #[test]
    fn test_arithmetic_functions() {
        // mobius
        assert_eq!(mobius(1).unwrap(), 1);
        assert_eq!(mobius(2).unwrap(), -1);
        assert_eq!(mobius(3).unwrap(), -1);
        assert_eq!(mobius(4).unwrap(), 0);
        assert_eq!(mobius(6).unwrap(), 1);
        assert_eq!(mobius(12).unwrap(), 0);
        assert_eq!(mobius(30).unwrap(), -1);
        assert!(mobius(0).is_err());
        assert!(mobius(-5).is_err());

        // tau
        assert_eq!(tau(1).unwrap(), 1);
        assert_eq!(tau(2).unwrap(), 2);
        assert_eq!(tau(12).unwrap(), 6);
        assert_eq!(tau(36).unwrap(), 9);
        assert!(tau(0).is_err());

        // sigma
        assert_eq!(sigma(1, 1).unwrap(), 1);
        assert_eq!(sigma(12, 1).unwrap(), 28);
        assert_eq!(sigma(12, 0).unwrap(), 6);
        assert_eq!(sigma(6, 2).unwrap(), 50);
        assert!(sigma(0, 1).is_err());

        // arithmetic_suite
        assert_eq!(arithmetic_suite(1, 1).unwrap(), (1, 1, 1));
        assert_eq!(arithmetic_suite(12, 1).unwrap(), (0, 28, 6));
        assert_eq!(arithmetic_suite(30, 1).unwrap(), (-1, 72, 8));
        assert_eq!(arithmetic_suite(6, 2).unwrap(), (1, 50, 4));
        assert!(arithmetic_suite(0, 1).is_err());
    }

    #[test]
    fn test_rsa() {
        let mut rng = SimpleRng::new(12345);

        // Prime generation
        let prime_16 = generate_prime(16, &mut rng).unwrap();
        assert!(is_prime(prime_16));
        assert!((32768..65536).contains(&prime_16));

        // Keygen
        let keys = rsa_keygen(32, 65537, &mut rng).unwrap();
        assert_eq!(keys.p * keys.q, keys.n);
        assert!(keys.p > keys.q);
        assert_eq!(
            ((keys.e as i128) * (keys.d as i128)).rem_euclid(keys.lambda as i128),
            1
        );

        // Encrypt & Decrypt standard
        let message = 42i64;
        let c = rsa_encrypt(message, keys.e, keys.n).unwrap();
        let decrypted = rsa_decrypt(c, keys.d, keys.n, None, None).unwrap();
        assert_eq!(decrypted, message);

        // Decrypt with CRT acceleration
        let decrypted_crt = rsa_decrypt(c, keys.d, keys.n, Some(keys.p), Some(keys.q)).unwrap();
        assert_eq!(decrypted_crt, message);

        // Crack toy modulus
        let cracked = rsa_crack(keys.n, keys.e, Some(c)).unwrap();
        assert_eq!(cracked.p, keys.p);
        assert_eq!(cracked.q, keys.q);
        assert_eq!(cracked.d, keys.d);
        assert_eq!(cracked.m, Some(message));

        // Error cases
        assert!(rsa_keygen(6, 65537, &mut rng).is_err());
        assert!(rsa_keygen(32, 2, &mut rng).is_err());
        assert!(rsa_keygen(32, 4, &mut rng).is_err());
        assert!(rsa_encrypt(keys.n + 1, keys.e, keys.n).is_err());
        assert!(rsa_decrypt(c, keys.d, keys.n, Some(keys.p), None).is_err());
        assert!(rsa_decrypt(c, keys.d, keys.n, Some(keys.p), Some(keys.p)).is_err());
    }

    #[test]
    fn test_dh() {
        let mut rng = SimpleRng::new(54321);

        // Alice generates keys
        let alice = dh_keygen(23, 5, Some(6), &mut rng).unwrap();
        assert_eq!(alice.private_key, 6);
        assert_eq!(alice.public_key, 8); // 5^6 mod 23 = 8

        // Bob generates keys with random private scalar
        let bob = dh_keygen(23, 5, Some(15), &mut rng).unwrap();
        assert_eq!(bob.private_key, 15);
        assert_eq!(bob.public_key, 19); // 5^15 mod 23 = 19

        // Derive shared secret
        let s_alice = dh_derive(alice.private_key, bob.public_key, 23).unwrap();
        let s_bob = dh_derive(bob.private_key, alice.public_key, 23).unwrap();
        assert_eq!(s_alice, 2);
        assert_eq!(s_bob, 2);

        // Crack private key via discrete log
        let cracked = dh_crack(5, alice.public_key, 23).unwrap();
        assert_eq!(cracked.private_key, 6);

        // Error cases
        assert!(dh_keygen(24, 5, None, &mut rng).is_err()); // composite p
        assert!(dh_keygen(23, 23, None, &mut rng).is_err()); // g >= p
        assert!(dh_derive(0, 8, 23).is_err());
        assert!(dh_derive(6, 25, 23).is_err());
    }

    #[test]
    fn test_challenge_rsa_dh() {
        let mut rng = SimpleRng::new(999);
        let rsa_prob = generate_problem("rsa", &mut rng).unwrap();
        assert_eq!(rsa_prob.topic, "rsa");
        assert!(rsa_prob.question.contains("In RSA with"));

        let dh_prob = generate_problem("dh", &mut rng).unwrap();
        assert_eq!(dh_prob.topic, "dh");
        assert!(dh_prob.question.contains("Diffie-Hellman"));
    }

    #[test]
    fn test_mersenne_and_perfect() {
        assert_eq!(lucas_lehmer(2), Ok(true));
        assert_eq!(lucas_lehmer(3), Ok(true));
        assert_eq!(lucas_lehmer(5), Ok(true));
        assert_eq!(lucas_lehmer(7), Ok(true));
        assert_eq!(lucas_lehmer(11), Ok(false));
        assert_eq!(lucas_lehmer(13), Ok(true));
        assert_eq!(lucas_lehmer(17), Ok(true));
        assert_eq!(lucas_lehmer(19), Ok(true));
        assert_eq!(lucas_lehmer(23), Ok(false));
        assert_eq!(lucas_lehmer(31), Ok(true));
        assert_eq!(lucas_lehmer(61), Ok(true));
        assert_eq!(lucas_lehmer(127), Ok(true));

        // Error cases
        assert!(lucas_lehmer(1).is_err());
        assert!(lucas_lehmer(6000).is_err());

        // Mersenne info
        let m7 = mersenne_info(7).unwrap();
        assert_eq!(m7.p, 7);
        assert_eq!(m7.mp, "127");
        assert!(m7.p_is_prime);
        assert!(m7.is_prime);
        assert_eq!(m7.digits, 3);
        assert_eq!(m7.perfect_number.as_deref(), Some("8128"));

        let m11 = mersenne_info(11).unwrap();
        assert_eq!(m11.p, 11);
        assert_eq!(m11.mp, "2047");
        assert!(m11.p_is_prime);
        assert!(!m11.is_prime);
        assert_eq!(m11.perfect_number, None);

        // Euclid-Euler perfect numbers
        assert!(is_perfect(6));
        assert!(is_perfect(28));
        assert!(is_perfect(496));
        assert!(is_perfect(8128));
        assert!(!is_perfect(12));
        assert!(!is_perfect(1));
        assert!(!is_perfect(0));
        assert!(!is_perfect(-28));

        assert_eq!(perfect_number(2).unwrap(), "6");
        assert_eq!(perfect_number(3).unwrap(), "28");
        assert_eq!(perfect_number(5).unwrap(), "496");
        assert_eq!(perfect_number(7).unwrap(), "8128");
        assert_eq!(perfect_number(31).unwrap(), "2305843008139952128");
        assert!(perfect_number(11).is_err());

        // Mersenne primes list
        let primes_31 = mersenne_primes(31);
        assert_eq!(primes_31, vec![2, 3, 5, 7, 13, 17, 19, 31]);
    }

    #[test]
    fn test_challenge_mersenne_perfect() {
        let mut rng = SimpleRng::new(42);
        let prob_m = generate_problem("mersenne", &mut rng).unwrap();
        assert_eq!(prob_m.topic, "mersenne");
        assert!(prob_m.question.contains("Mersenne"));

        let prob_p = generate_problem("perfect", &mut rng).unwrap();
        assert_eq!(prob_p.topic, "perfect");
        assert!(prob_p.question.contains("perfect"));
    }

    #[test]
    fn test_pollard_p_minus_1() {
        // Stage 1 test: n = 1009 * 1381 = 1393429.
        // 1009 - 1 = 1008 = 2^4 * 3^2 * 7. All prime powers <= 16.
        let res1 = pollard_p_minus_1(1393429, Some(20), None, Some(2)).unwrap();
        assert!(res1.success);
        assert_eq!(res1.factor, Some(1009));
        assert_eq!(res1.cofactor, Some(1381));
        assert_eq!(res1.stage, Some(1));
        assert_eq!(res1.p_minus_1, Some(1008));
        assert_eq!(res1.p_minus_1_expression.as_deref(), Some("2^4 * 3^2 * 7"));

        // Stage 2 test: n = 1019 * 1381 = 1407239.
        // 1019 - 1 = 2 * 509. With B1=2, B2=600, Stage 2 catches 1019 at prime 509!
        let res2 = pollard_p_minus_1(1407239, Some(2), Some(600), Some(2)).unwrap();
        assert!(res2.success);
        assert_eq!(res2.factor, Some(1019));
        assert_eq!(res2.cofactor, Some(1381));
        assert_eq!(res2.stage, Some(2));
        assert_eq!(res2.p_minus_1, Some(1018));

        // Prime test
        let res_prime = pollard_p_minus_1(1009, Some(100), None, Some(2)).unwrap();
        assert!(!res_prime.success);
        assert_eq!(res_prime.factor, None);
        assert!(res_prime.message.contains("already prime"));

        // Even test
        let res_even = pollard_p_minus_1(2048, Some(100), None, Some(2)).unwrap();
        assert!(res_even.success);
        assert_eq!(res_even.factor, Some(2));

        // Failure when bounds too small
        let res_fail = pollard_p_minus_1(1407239, Some(3), Some(10), Some(2)).unwrap();
        assert!(!res_fail.success);
        assert_eq!(res_fail.factor, None);

        // Input validation
        assert!(pollard_p_minus_1(1, None, None, None).is_err());
        assert!(pollard_p_minus_1(15, None, None, Some(1)).is_err());
        assert!(pollard_p_minus_1(15, None, None, Some(15)).is_err());
    }

    #[test]
    fn test_challenge_pm1() {
        let mut rng = SimpleRng::new(42);
        let prob = generate_problem("pm1", &mut rng).unwrap();
        assert_eq!(prob.topic, "pm1");
        assert!(prob.question.contains("p - 1 is smooth"));
        assert!(prob.answer > 1);
    }
}
