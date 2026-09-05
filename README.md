# crypto-lab

A small self-hosted Rust service for hashing and HMAC operations. It is designed to run alongside `lab-api` on the same EC2 instance without a database, queue, container, or monitoring stack.

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

The Rust service binds to `127.0.0.1:8089`. Nginx should terminate HTTPS, apply Basic Auth to crypto operations, and proxy the public `/crypto-api/` prefix to the local service.

All operation requests must use `Content-Type: application/json`. Text fields are interpreted as UTF-8, and cryptographic operations operate on their UTF-8 byte representation. Binary input and base64 encoding are intentionally outside the v0.1.0 contract.

## API

| Method | Route | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/health` | No | Service health |
| GET | `/v1/info` | No | Non-sensitive application metadata and endpoint discovery |
| POST | `/v1/hash` | Basic Auth at Nginx | SHA-256 or SHA-512 digest |
| POST | `/v1/hmac` | Basic Auth at Nginx | HMAC generation |
| POST | `/v1/hmac/verify` | Basic Auth at Nginx | Constant-time HMAC verification |

The public equivalents use the `/crypto-api/` prefix, for example `https://example.com/crypto-api/v1/hash`.

### Application information

Request:

```bash
curl -sS https://example.com/crypto-api/v1/info
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
    "POST /v1/hmac/verify"
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

location /crypto-api/ {
    auth_basic "crypto-lab";
    auth_basic_user_file /etc/nginx/.htpasswd;
    proxy_pass http://127.0.0.1:8089/;
}
```

The exact health and info matches keep `/crypto-api/health` and `/crypto-api/v1/info` unauthenticated. The trailing slash on the authenticated location preserves the backend route shape: `/crypto-api/v1/hash` becomes `/v1/hash`.

## systemd

Install `crypto-lab.service` as `/etc/systemd/system/crypto-lab.service`, then run:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now crypto-lab
sudo systemctl status crypto-lab --no-pager
```

The unit binds the service to localhost and applies process, filesystem, and network hardening. Build the release binary with `cargo build --release` and install it at `/usr/local/bin/crypto-lab`.
