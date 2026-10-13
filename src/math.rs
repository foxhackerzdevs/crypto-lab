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

// ------------------- Challenge / Quiz Generator -------------------

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChallengeProblem {
    pub topic: String,
    pub question: String,
    pub answer: i64,
}

pub fn generate_problem(topic: &str, rng: &mut SimpleRng) -> Result<ChallengeProblem, MathError> {
    let selected_topic = if topic == "mixed" {
        let choices = ["legendre", "sqrtmod", "crt", "ecdlp", "factor"];
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
        _ => Err(MathError(
            StatusCode::BAD_REQUEST,
            "topic must be legendre, sqrtmod, crt, ecdlp, factor, or mixed",
        )),
    }
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
        assert_eq!(format_factorization(&factorize(360).unwrap()), "2^3 * 3^2 * 5");
        assert_eq!(
            factorize(104729).unwrap(),
            vec![PrimeFactor { prime: 104729, power: 1 }]
        );
        assert!(factorize(0).is_err());
        assert!(factorize(-10).is_err());
        // Large semiprime (1000003 * 1000033 = 1000036000099)
        assert_eq!(
            factorize(1000036000099).unwrap(),
            vec![
                PrimeFactor { prime: 1000003, power: 1 },
                PrimeFactor { prime: 1000033, power: 1 },
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
    }
}
