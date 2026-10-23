use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const LEGENDRE_TABLES: &[(u64, &[i8])] = &[
    (3, &[0, 1, -1]),
    (5, &[0, 1, -1, -1, 1]),
    (7, &[0, 1, 1, -1, 1, -1, -1]),
    (11, &[0, 1, -1, 1, 1, 1, -1, -1, -1, 1, -1]),
    (13, &[0, 1, -1, 1, 1, -1, -1, -1, -1, 1, 1, -1, 1]),
    (
        17,
        &[0, 1, 1, -1, 1, -1, -1, -1, 1, 1, -1, -1, -1, 1, -1, 1, 1],
    ),
    (
        19,
        &[
            0, 1, -1, -1, 1, 1, 1, 1, -1, 1, -1, 1, -1, -1, -1, -1, 1, 1, -1,
        ],
    ),
    (
        23,
        &[
            0, 1, 1, 1, 1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, -1, 1, -1, 1, -1, -1, -1, -1,
        ],
    ),
    (
        29,
        &[
            0, 1, -1, -1, 1, 1, 1, 1, -1, 1, -1, -1, -1, 1, -1, -1, 1, -1, -1, -1, 1, -1, 1, 1, 1,
            1, -1, -1, 1,
        ],
    ),
    (
        31,
        &[
            0, 1, 1, -1, 1, 1, -1, 1, 1, 1, 1, -1, -1, -1, 1, -1, 1, -1, 1, 1, 1, -1, -1, -1, -1,
            1, -1, -1, 1, -1, -1,
        ],
    ),
    (
        37,
        &[
            0, 1, -1, 1, 1, -1, -1, 1, -1, 1, 1, 1, 1, -1, -1, -1, 1, -1, -1, -1, -1, 1, -1, -1,
            -1, 1, 1, 1, 1, -1, 1, -1, -1, 1, 1, -1, 1,
        ],
    ),
    (
        41,
        &[
            0, 1, 1, -1, 1, 1, -1, -1, 1, 1, 1, -1, -1, -1, -1, -1, 1, -1, 1, -1, 1, 1, -1, 1, -1,
            1, -1, -1, -1, -1, -1, 1, 1, 1, -1, -1, 1, 1, -1, 1, 1,
        ],
    ),
    (
        43,
        &[
            0, 1, -1, -1, 1, -1, 1, -1, -1, 1, 1, 1, -1, 1, 1, 1, 1, 1, -1, -1, -1, 1, -1, 1, 1, 1,
            -1, -1, -1, -1, -1, 1, -1, -1, -1, 1, 1, -1, 1, -1, 1, 1, -1,
        ],
    ),
    (
        47,
        &[
            0, 1, 1, 1, 1, -1, 1, 1, 1, 1, -1, -1, 1, -1, 1, -1, 1, 1, 1, -1, -1, 1, -1, -1, 1, 1,
            -1, 1, 1, -1, -1, -1, 1, -1, 1, -1, 1, 1, -1, -1, -1, -1, 1, -1, -1, -1, -1,
        ],
    ),
    (
        53,
        &[
            0, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1, 1, -1, 1, -1, 1, 1, 1, -1, -1, -1, -1, -1, -1, 1,
            1, -1, -1, 1, 1, -1, -1, -1, -1, -1, -1, 1, 1, 1, -1, 1, -1, 1, 1, 1, -1, 1, 1, -1, 1,
            -1, -1, 1,
        ],
    ),
    (
        59,
        &[
            0, 1, -1, 1, 1, 1, -1, 1, -1, 1, -1, -1, 1, -1, -1, 1, 1, 1, -1, 1, 1, 1, 1, -1, -1, 1,
            1, 1, 1, 1, -1, -1, -1, -1, -1, 1, 1, -1, -1, -1, -1, 1, -1, -1, -1, 1, 1, -1, 1, 1,
            -1, 1, -1, 1, -1, -1, -1, 1, -1,
        ],
    ),
    (
        61,
        &[
            0, 1, -1, 1, 1, 1, -1, -1, -1, 1, -1, -1, 1, 1, 1, 1, 1, -1, -1, 1, 1, -1, 1, -1, -1,
            1, -1, 1, -1, -1, -1, -1, -1, -1, 1, -1, 1, -1, -1, 1, -1, 1, 1, -1, -1, 1, 1, 1, 1, 1,
            -1, -1, 1, -1, -1, -1, 1, 1, 1, -1, 1,
        ],
    ),
    (
        67,
        &[
            0, 1, -1, -1, 1, -1, 1, -1, -1, 1, 1, -1, -1, -1, 1, 1, 1, 1, -1, 1, -1, 1, 1, 1, 1, 1,
            1, -1, -1, 1, -1, -1, -1, 1, -1, 1, 1, 1, -1, 1, 1, -1, -1, -1, -1, -1, -1, 1, -1, 1,
            -1, -1, -1, -1, 1, 1, 1, -1, -1, 1, 1, -1, 1, -1, 1, 1, -1,
        ],
    ),
    (
        71,
        &[
            0, 1, 1, 1, 1, 1, 1, -1, 1, 1, 1, -1, 1, -1, -1, 1, 1, -1, 1, 1, 1, -1, -1, -1, 1, 1,
            -1, 1, -1, 1, 1, -1, 1, -1, -1, -1, 1, 1, 1, -1, 1, -1, -1, 1, -1, 1, -1, -1, 1, 1, 1,
            -1, -1, -1, 1, -1, -1, 1, 1, -1, 1, -1, -1, -1, 1, -1, -1, -1, -1, -1, -1,
        ],
    ),
    (
        73,
        &[
            0, 1, 1, 1, 1, -1, 1, -1, 1, 1, -1, -1, 1, -1, -1, -1, 1, -1, 1, 1, -1, -1, -1, 1, 1,
            1, -1, 1, -1, -1, -1, -1, 1, -1, -1, 1, 1, 1, 1, -1, -1, 1, -1, -1, -1, -1, 1, -1, 1,
            1, 1, -1, -1, -1, 1, 1, -1, 1, -1, -1, -1, 1, -1, -1, 1, 1, -1, 1, -1, 1, 1, 1, 1,
        ],
    ),
];

/// Exact integer square root floor: floor(sqrt(n)) using binary digit-by-digit algorithm.
pub fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut root = 0u128;
    // 1 << 126 is the largest power of 4 fitting in u128
    let mut bit = 1u128 << 126;
    while bit > n {
        bit >>= 2;
    }
    let mut rem = n;
    while bit != 0 {
        if rem >= root + bit {
            rem -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// Smallest integer x such that x^2 >= n.
pub fn sqrt_ceil(n: u128) -> u128 {
    let s = integer_sqrt(n);
    if s.checked_mul(s) == Some(n) {
        s
    } else {
        s + 1
    }
}

/// Checks if n is a perfect square; returns Some(sqrt(n)) if true, None otherwise.
pub fn sqrt_exact(n: u128) -> Option<u128> {
    let s = integer_sqrt(n);
    if s.checked_mul(s) == Some(n) {
        Some(s)
    } else {
        None
    }
}

/// Fast quadratic residue check across small primes.
/// Returns Ok(()) if candidate passes all sieve primes, or Err(p) where p rejected it.
pub fn check_sieve(r: u128) -> Result<(), u64> {
    for &(p, table) in LEGENDRE_TABLES {
        let rem = (r % (p as u128)) as usize;
        if table[rem] == -1 {
            return Err(p);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FermatStepRecord {
    pub step: u64,
    pub x: u128,
    pub r: u128,
    pub sieve_passed: bool,
    pub rejected_by: Option<u64>,
    pub is_square: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Step(FermatStepRecord),
    Found {
        step: u64,
        x: u128,
        y: u128,
        p: u128,
        q: u128,
        record: FermatStepRecord,
    },
    Trivial {
        step: u64,
        x: u128,
        y: u128,
        p: u128,
        q: u128,
        record: FermatStepRecord,
    },
    DoneMaxX {
        step: u64,
        x: u128,
        r: u128,
    },
    Overflow,
    AlreadyFinished,
}

pub struct FermatStepper {
    pub n: u128,
    pub x: u128,
    pub r: u128,
    pub step: u64,
    pub max_x: u128,
    pub finished: bool,
}

impl FermatStepper {
    #[allow(clippy::manual_is_multiple_of, clippy::manual_div_ceil)]
    pub fn new(n: u128) -> Result<Self, &'static str> {
        if n < 3 || n % 2 == 0 {
            return Err("n must be an odd integer >= 3");
        }
        let x = sqrt_ceil(n);
        let x2 = x.checked_mul(x).ok_or("initial x^2 overflow")?;
        let r = x2 - n;
        let max_x = (n + 1) / 2;
        Ok(Self {
            n,
            x,
            r,
            step: 0,
            max_x,
            finished: false,
        })
    }

    pub fn next_step(&mut self) -> StepOutcome {
        if self.finished {
            return StepOutcome::AlreadyFinished;
        }

        self.step += 1;
        let curr_x = self.x;
        let curr_r = self.r;

        // Check sieve
        let sieve_res = check_sieve(curr_r);
        let (sieve_passed, rejected_by) = match sieve_res {
            Ok(()) => (true, None),
            Err(p) => (false, Some(p)),
        };

        let root_y = if sieve_passed {
            sqrt_exact(curr_r)
        } else {
            None
        };

        if let Some(y) = root_y {
            self.finished = true;
            let p = curr_x.saturating_sub(y);
            let q = curr_x + y;
            let record = FermatStepRecord {
                step: self.step,
                x: curr_x,
                r: curr_r,
                sieve_passed,
                rejected_by: None,
                is_square: true,
                y: Some(y),
            };

            if p == 1 || q == 1 || p == self.n || q == self.n {
                return StepOutcome::Trivial {
                    step: self.step,
                    x: curr_x,
                    y,
                    p,
                    q,
                    record,
                };
            } else {
                return StepOutcome::Found {
                    step: self.step,
                    x: curr_x,
                    y,
                    p,
                    q,
                    record,
                };
            }
        }

        let record = FermatStepRecord {
            step: self.step,
            x: curr_x,
            r: curr_r,
            sieve_passed,
            rejected_by,
            is_square: false,
            y: None,
        };

        // Advance to next attempt: r_next = curr_r + 2 * curr_x + 1
        let two_x_plus_one = match curr_x.checked_mul(2).and_then(|t| t.checked_add(1)) {
            Some(v) => v,
            None => {
                self.finished = true;
                return StepOutcome::Overflow;
            }
        };

        let next_r = match curr_r.checked_add(two_x_plus_one) {
            Some(v) => v,
            None => {
                self.finished = true;
                return StepOutcome::Overflow;
            }
        };

        let next_x = curr_x + 1;
        if next_x > self.max_x {
            self.finished = true;
            return StepOutcome::DoneMaxX {
                step: self.step,
                x: curr_x,
                r: curr_r,
            };
        }

        self.x = next_x;
        self.r = next_r;

        StepOutcome::Step(record)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FermatSummary {
    pub ok: bool,
    pub n: u128,
    pub initial_x: u128,
    pub steps_count: u64,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<u128>,
    pub elapsed_us: u128,
    pub reason: &'static str,
    pub steps: Vec<FermatStepRecord>,
    pub steps_truncated: bool,
}

pub fn fermat_factor(
    n: u128,
    max_steps: u64,
    record_steps: bool,
) -> Result<FermatSummary, &'static str> {
    let start = Instant::now();
    let mut stepper = FermatStepper::new(n)?;
    let initial_x = stepper.x;
    let mut steps = Vec::new();
    let mut steps_count = 0u64;

    while steps_count < max_steps {
        match stepper.next_step() {
            StepOutcome::Step(rec) => {
                steps_count = rec.step;
                if record_steps && steps.len() < 1000 {
                    steps.push(rec);
                }
            }
            StepOutcome::Found {
                step,
                x,
                y,
                p,
                q,
                record,
            } => {
                let elapsed_us = start.elapsed().as_micros();
                if record_steps {
                    steps.push(record);
                }
                return Ok(FermatSummary {
                    ok: true,
                    n,
                    initial_x,
                    steps_count: step,
                    found: true,
                    p: Some(p),
                    q: Some(q),
                    x: Some(x),
                    y: Some(y),
                    elapsed_us,
                    reason: "found",
                    steps,
                    steps_truncated: false,
                });
            }
            StepOutcome::Trivial {
                step,
                x,
                y,
                p,
                q,
                record,
            } => {
                let elapsed_us = start.elapsed().as_micros();
                if record_steps {
                    steps.push(record);
                }
                return Ok(FermatSummary {
                    ok: true,
                    n,
                    initial_x,
                    steps_count: step,
                    found: false,
                    p: Some(p),
                    q: Some(q),
                    x: Some(x),
                    y: Some(y),
                    elapsed_us,
                    reason: "trivial",
                    steps,
                    steps_truncated: false,
                });
            }
            StepOutcome::DoneMaxX { step, .. } => {
                let elapsed_us = start.elapsed().as_micros();
                return Ok(FermatSummary {
                    ok: true,
                    n,
                    initial_x,
                    steps_count: step,
                    found: false,
                    p: None,
                    q: None,
                    x: None,
                    y: None,
                    elapsed_us,
                    reason: "exhausted",
                    steps,
                    steps_truncated: false,
                });
            }
            StepOutcome::Overflow => {
                let elapsed_us = start.elapsed().as_micros();
                return Ok(FermatSummary {
                    ok: false,
                    n,
                    initial_x,
                    steps_count,
                    found: false,
                    p: None,
                    q: None,
                    x: None,
                    y: None,
                    elapsed_us,
                    reason: "overflow",
                    steps,
                    steps_truncated: false,
                });
            }
            StepOutcome::AlreadyFinished => break,
        }
    }

    let elapsed_us = start.elapsed().as_micros();
    let steps_truncated = steps.len() < (steps_count as usize);
    Ok(FermatSummary {
        ok: true,
        n,
        initial_x,
        steps_count,
        found: false,
        p: None,
        q: None,
        x: None,
        y: None,
        elapsed_us,
        reason: "max_steps",
        steps,
        steps_truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_sqrt() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(2), 1);
        assert_eq!(integer_sqrt(3), 1);
        assert_eq!(integer_sqrt(4), 2);
        assert_eq!(integer_sqrt(8), 2);
        assert_eq!(integer_sqrt(9), 3);
        assert_eq!(integer_sqrt(10), 3);
        assert_eq!(integer_sqrt(125), 11);
        assert_eq!(integer_sqrt(441), 21);
        assert_eq!(integer_sqrt(5959), 77);
        assert_eq!(sqrt_ceil(5959), 78);
        assert_eq!(sqrt_exact(441), Some(21));
        assert_eq!(sqrt_exact(442), None);
    }

    #[test]
    fn test_sieve_rejection() {
        // r = 125 mod 3 = 2, Legendre table for 3 at index 2 is -1 -> rejected by 3
        assert_eq!(check_sieve(125), Err(3));
        // r = 282 mod 3 = 0, mod 5 = 2 -> rejected by 5
        assert_eq!(check_sieve(282), Err(5));
        // r = 441 = 21^2 is a square, passes all sieve primes
        assert_eq!(check_sieve(441), Ok(()));
    }

    #[test]
    fn test_fermat_factor_5959() {
        let res = fermat_factor(5959, 1000, true).unwrap();
        assert!(res.found);
        assert_eq!(res.steps_count, 3);
        assert_eq!(res.x, Some(80));
        assert_eq!(res.y, Some(21));
        let p = res.p.unwrap();
        let q = res.q.unwrap();
        assert_eq!(p * q, 5959);
        assert_eq!(p.min(q), 59);
        assert_eq!(p.max(q), 101);
        assert_eq!(res.steps.len(), 3);
        assert_eq!(res.steps[0].rejected_by, Some(3));
        assert_eq!(res.steps[1].rejected_by, Some(5));
        assert!(res.steps[2].is_square);
    }

    #[test]
    fn test_fermat_factor_8051() {
        // 8051 = 83 * 97
        let res = fermat_factor(8051, 1000, false).unwrap();
        assert!(res.found);
        let p = res.p.unwrap();
        let q = res.q.unwrap();
        assert_eq!(p * q, 8051);
        assert_eq!(p.min(q), 83);
        assert_eq!(p.max(q), 97);
    }

    #[test]
    fn test_fermat_prime_trivial() {
        // 17 is prime: max_x is (17+1)/2 = 9
        // ceil(sqrt(17)) = 5
        // x=5: r=25-17=8 (non-square)
        // x=6: r=36-17=19 (non-square)
        // x=7: r=49-17=32 (non-square)
        // x=8: r=64-17=47 (non-square)
        // x=9: r=81-17=64 = 8^2! y=8 => p=9-8=1, q=9+8=17. Trivial!
        let res = fermat_factor(17, 1000, false).unwrap();
        assert!(!res.found);
        assert_eq!(res.reason, "trivial");
        assert_eq!(res.p, Some(1));
        assert_eq!(res.q, Some(17));
    }
}
