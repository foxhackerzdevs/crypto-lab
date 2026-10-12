# crypto-lab

A small self-hosted Rust service for hashing, HMAC, and number-theoretic cryptography exploration (FoxMath algorithms). It is designed to run alongside `lab-api` on the same EC2 instance without a database, queue, container, or monitoring stack.

## Architecture

```text
Internet
   |
HTTPS :443
   |
Nginx
   |-- /api/*        -> 127.0.0.1:8088 -> lab-api
   `-- /crypto-api/* -> 127.0.0.1:8089 -> crypto-lab
```

The Rust service binds to `127.0.0.1:8089`. Nginx terminates HTTPS, optionally applies Basic Auth, and proxies the public `/crypto-api/` prefix to the local service.

JSON requests must use `Content-Type: application/json`. All `/v1/math/*` endpoints support both `GET` (query parameters) for instant browser/CLI exploration and `POST` (JSON bodies) for programmatic pipelines.

## API

| Method | Route | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/health` | No | Service health |
| GET | `/v1/info` | No | Application metadata and endpoint discovery |
| POST | `/v1/hash` | Basic Auth at Nginx | SHA-256 or SHA-512 digest |
| POST | `/v1/hmac` | Basic Auth at Nginx | HMAC generation |
| POST | `/v1/hmac/verify` | Basic Auth at Nginx | Constant-time HMAC verification |
| GET / POST | `/v1/math/legendre` | Configurable | Legendre symbol $(a/p)$ via Euler's criterion |
| GET / POST | `/v1/math/sqrtmod` | Configurable | Modular square roots via Tonelli-Shanks |
| GET / POST | `/v1/math/crt` | Configurable | Chinese Remainder Theorem solver (Extended GCD) |
| GET / POST | `/v1/math/cf` | Configurable | Continued fraction expansion & convergents |
| GET / POST | `/v1/math/ecadd` | Configurable | Elliptic curve point addition over $\mathbb{F}_p$ |
| GET / POST | `/v1/math/ecmul` | Configurable | Elliptic curve scalar multiplication $k \cdot P$ |
| GET / POST | `/v1/math/challenge` | Configurable | Seeded, reproducible math & crypto challenges |

The public equivalents use the `/crypto-api/` prefix, for example `https://abhrankan.duckdns.org/crypto-api/v1/math/sqrtmod?a=10&p=13`.

### Application information

Request:

```bash
curl -sS https://abhrankan.duckdns.org/crypto-api/v1/info
```

Response:

```json
{
  "service": "crypto-lab",
  "api": "v1",
  "version": "0.1.0",
  "endpoints": [
    "GET /health",
    "GET /v1/info",
    "POST /v1/hash",
    "POST /v1/hmac",
    "POST /v1/hmac/verify",
    "GET /v1/math/legendre",
    "POST /v1/math/legendre",
    "GET /v1/math/sqrtmod",
    "POST /v1/math/sqrtmod",
    "GET /v1/math/crt",
    "POST /v1/math/crt",
    "GET /v1/math/cf",
    "POST /v1/math/cf",
    "GET /v1/math/ecadd",
    "POST /v1/math/ecadd",
    "GET /v1/math/ecmul",
    "POST /v1/math/ecmul",
    "GET /v1/math/challenge",
    "POST /v1/math/challenge"
  ],
  "build_profile": "release",
  "environment": "production"
}
```

The information endpoint is public and exposes only non-sensitive application metadata. `environment` comes from the optional `LAB_API_ENV` deployment label and defaults to `unknown`; it must never contain secrets.

### Hash

Request:

```json
{
  "algorithm": "sha256",
  "data": "hello"
}
```

Response:

```json
{
  "algorithm": "sha256",
  "digest": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
}
```

### HMAC

Request:

```json
{
  "algorithm": "sha256",
  "key": "secret",
  "data": "message"
}
```

Response:

```json
{
  "algorithm": "sha256",
  "mac": "8b5f48702995c1598c573db1e21866a9b825d4a794d169d7060a03605796360b"
}
```

Digest and MAC values are lowercase hexadecimal strings. Supported algorithms are exactly `sha256` and `sha512`.

### HMAC verification

Request:

```json
{
  "algorithm": "sha256",
  "key": "secret",
  "data": "message",
  "mac": "8b5f48702995c1598c573db1e21866a9b825d4a794d169d7060a03605796360b"
}
```

Response:

```json
{
  "algorithm": "sha256",
  "valid": true
}
```

Verification compares the decoded MAC using a constant-time equality operation. An invalid hexadecimal MAC is a `400 Bad Request`; a well-formed but incorrect MAC returns `200 OK` with `"valid": false`.

### Math & Number Theory (FoxMath Core)

All math endpoints accept either query parameters (`GET`) or a JSON payload (`POST`).

#### 1. Legendre Symbol (`/v1/math/legendre`)
Computes $(a/p)$ via Euler's criterion. Returns `1` (quadratic residue), `-1` (non-residue), or `0` ($a \equiv 0 \pmod p$).

```bash
# GET via query parameters
curl -sS "https://abhrankan.duckdns.org/crypto-api/v1/math/legendre?a=10&p=13"
# {"a":10,"p":13,"value":1}

# POST via JSON
curl -sS -H "content-type: application/json" \
  -d '{"a": 5, "p": 17}' \
  https://abhrankan.duckdns.org/crypto-api/v1/math/legendre
# {"a":5,"p":17,"value":-1}
```

#### 2. Modular Square Roots (`/v1/math/sqrtmod`)
Finds square roots modulo an odd prime $p$ via the Tonelli-Shanks algorithm ($O(\log p)$ fast path for $p \equiv 3 \pmod 4$).

```bash
curl -sS "https://abhrankan.duckdns.org/crypto-api/v1/math/sqrtmod?a=10&p=13"
```
```json
{
  "a": 10,
  "p": 13,
  "roots": [6, 7]
}
```

#### 3. Chinese Remainder Theorem (`/v1/math/crt`)
Solves simultaneous congruences $x \equiv r_i \pmod{m_i}$ using Extended Euclidean GCD.

```bash
# Query params: r=2,3,2 & m=3,5,7
curl -sS "https://abhrankan.duckdns.org/crypto-api/v1/math/crt?r=2,3,2&m=3,5,7"

# JSON request:
curl -sS -H "content-type: application/json" \
  -d '{"remainders": [2, 3, 2], "moduli": [3, 5, 7]}' \
  https://abhrankan.duckdns.org/crypto-api/v1/math/crt
```
```json
{
  "remainders": [2, 3, 2],
  "moduli": [3, 5, 7],
  "x": 23,
  "modulus": 105
}
```

#### 4. Continued Fraction Expansion (`/v1/math/cf`)
Expands rational fraction $num/den$ into simple continued fraction terms and computes intermediate convergents.

```bash
curl -sS "https://abhrankan.duckdns.org/crypto-api/v1/math/cf?num=355&den=113"
```
```json
{
  "num": 355,
  "den": 113,
  "cf": [3, 7, 16],
  "convergents": [
    [3, 1],
    [22, 7],
    [355, 113]
  ]
}
```

#### 5. Elliptic Curve Point Addition & Multiplication (`/v1/math/ecadd`, `/v1/math/ecmul`)
Performs group operations over $\mathbb{F}_p$ for Weierstrass curves $y^2 \equiv x^3 + ax + b \pmod p$. Built-in presets: `toy97` and `toy193`. Returns `null` if the result is the point at infinity (identity).

```bash
# Scalar multiplication k·G on toy193
curl -sS -H "content-type: application/json" \
  -d '{"curve": "toy193", "k": 7}' \
  https://abhrankan.duckdns.org/crypto-api/v1/math/ecmul
```
```json
{
  "curve": "toy193",
  "k": 7,
  "point": [1, 59],
  "result": [66, 24],
  "result_on_curve": true
}
```

#### 6. Challenge Generator (`/v1/math/challenge`)
Generates seeded, reproducible practice problems for self-study and learning. Supports topics: `legendre`, `sqrtmod`, `crt`, `ecdlp`, and `mixed`.

```bash
curl -sS "https://abhrankan.duckdns.org/crypto-api/v1/math/challenge?topic=legendre&seed=42&count=1"
```
```json
{
  "topic": "legendre",
  "seed": 42,
  "count": 1,
  "problems": [
    {
      "topic": "legendre",
      "question": "Legendre symbol (10/13)",
      "answer": 1
    }
  ]
}
```

## Limits and status codes

- Request bodies are limited to 64 KiB.
- Each textual input (`data`, `key`, and `mac`) is limited to 32 KiB.
- `200 OK`: successful health, hash, generation, or verification request.
- `400 Bad Request`: unsupported algorithm or malformed MAC.
- `415 Unsupported Media Type`: operation request does not use `application/json`.
- `401 Unauthorized`: missing or invalid Basic Auth at Nginx for crypto operations.
- `413 Payload Too Large`: body or textual input exceeds its limit.
- `404 Not Found`: route does not exist.

## Local development

With Rust and Cargo installed:

```bash
cargo test
cargo run
curl -sS http://127.0.0.1:8089/health
curl -sS -u 'username:password' \
  -H 'content-type: application/json' \
  -d '{"algorithm":"sha256","data":"hello"}' \
  http://127.0.0.1:8089/v1/hash
```

The service is intentionally not a wallet, miner, exchange, payment processor, or key-management system. Do not send private keys or production secrets to it.

## Nginx routing

A representative location block is:

```nginx
location = /crypto-api/health {
  proxy_pass http://127.0.0.1:8089/health;
}

location = /crypto-api/v1/info {
    proxy_pass http://127.0.0.1:8089/v1/info;
}

# Optional: expose math endpoints publicly for web playgrounds / exploration
location /crypto-api/v1/math/ {
    proxy_pass http://127.0.0.1:8089/v1/math/;
}

location /crypto-api/ {
    auth_basic "crypto-lab";
    auth_basic_user_file /etc/nginx/.htpasswd;
    proxy_pass http://127.0.0.1:8089/;
}
```

The exact health and info matches keep `/crypto-api/health` and `/crypto-api/v1/info` unauthenticated (as well as `/crypto-api/v1/math/` if exposed publicly). The trailing slash on the authenticated location preserves the backend route shape: `/crypto-api/v1/hash` becomes `/v1/hash`.

## systemd

Install `crypto-lab.service` as `/etc/systemd/system/crypto-lab.service`, then run:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now crypto-lab
sudo systemctl status crypto-lab --no-pager
```

The unit binds the service to localhost and applies process, filesystem, and network hardening. Build the release binary with `cargo build --release` and install it at `/usr/local/bin/crypto-lab`.
