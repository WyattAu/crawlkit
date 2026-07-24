# crawlkit-api

REST API server for crawlkit with JWT authentication and RBAC.

## Installation

```bash
cargo install crawlkit-api
```

## Usage

```bash
# Start API server
crawlkit-api --port 8080

# With custom JWT secret
JWT_SECRET=$(openssl rand -hex 32) crawlkit-api --port 8080
```

## Endpoints

- `POST /api/v1/auth/login` — Login with email/password
- `POST /api/v1/crawls` — Start a crawl
- `GET /api/v1/crawls/{id}` — Get crawl status
- `GET /api/v1/metrics` — Prometheus metrics

## Documentation

- [API Reference](https://wyattau.github.io/crawlkit/api-reference/)
- [GitHub Repository](https://github.com/WyattAu/crawlkit)
