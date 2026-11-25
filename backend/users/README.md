# Users

Handles registration, login and user accounts. Written in Rust with Axum, sqlx and PostgreSQL (`m3d_users`).

## Run with docker

From the repo root:

```powershell
Copy-Item .env.example .env
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build users users-db
```

The service is on http://localhost:8081 and Swagger is on http://localhost:8081/docs.

## Run locally

Start only the database with compose, then:

```powershell
cd backend/users
Copy-Item .env.example .env
cargo run
```

The table is created on startup.

## Code layout

- `src/main.rs` starts the service, `src/config.rs` reads env variables
- `src/routes.rs` router, `src/openapi.rs` Swagger spec
- `src/handlers/` endpoint handlers
- `src/models/` database struct, requests and responses
- `src/db/` connection pool, table creation and SQL queries
- `src/auth/` passwords, JWT and the logged in user check
- `src/cache/` Valkey cache for users
- `src/clients/` calls to other services

## Env variables

| Name                 | What it is                                       |
|----------------------|--------------------------------------------------|
| PORT                 | Port the service listens on                      |
| DB_URL               | Postgres connection string for m3d_users         |
| JWT_SECRET           | Secret used to sign tokens                       |
| NOTIFICATIONS_URL    | Base URL of the notifications service            |
| ACCESS_TOKEN_MINUTES | How long an access token lasts (default 15)      |
| CACHE_URL            | Valkey address, like `redis://localhost:6379`    |
| CACHE_TTL_SECONDS    | How long a user stays in the cache (default 300) |
| ADMIN_EMAIL          | Admin created on start if it does not exist      |
| ADMIN_PASSWORD       | Password for that admin (empty = no admin)       |
| RUST_LOG             | Log level (default `info,sqlx=warn`)             |
| RATE_LIMIT_REQUESTS  | Max auth calls per IP in the window (default 10, 0 = off) |
| RATE_LIMIT_WINDOW_SECONDS | Length of the rate limit window (default 60) |

## Endpoints

| Method | Path                  | Token         | What it does                          |
|--------|-----------------------|---------------|---------------------------------------|
| GET    | /health               | no            | Health check                          |
| GET    | /users                | admin         | List users, `?page=1&pageSize=20`     |
| GET    | /users/{id}           | no            | Get one user                          |
| POST   | /users/register       | no            | Create a user (201)                   |
| POST   | /users/login          | no            | Returns access and refresh token      |
| POST   | /users/refresh        | no            | New access and refresh token          |
| PUT    | /users/{id}           | self or admin | Change name and email                 |
| PUT    | /users/{id}/role      | admin         | Set role to `user` or `admin`         |
| DELETE | /users/{id}           | self or admin | Delete the user (204)                 |
| DELETE | /users/{id}/sessions  | self or admin | Logout, all old tokens stop working   |
| PUT    | /users/me/password    | any user      | Change password, logs out everywhere  |
| GET    | /auth/verify          | any user      | Token check used by Traefik           |

Send the token as `Authorization: Bearer <token>`.
The access token lasts 15 minutes. When it expires, send `{ refreshToken }` to `/users/refresh`.
Each refresh token works once and lasts 7 days. Only its sha256 hash is saved in the `refresh_tokens` table.
Logout and deleting a user also remove the user's refresh tokens.
`/auth/verify` returns 200 with `X-User-Id` and `X-User-Role` headers, or 401.
On start the service creates an admin from `ADMIN_EMAIL` and `ADMIN_PASSWORD` if that email does not exist yet.
Without them, the first registered user becomes admin.
The last admin cannot be deleted or made a normal user (409).
Every request is logged with method, path, status and time.
Passwords are hashed with argon2 and are never returned.
All errors are JSON like `{ "error": "user not found" }`, also for bad JSON bodies (400, 415, 422), bad path values (400) and unknown paths (404).

`GET /users` returns `{ items, page, pageSize, total }`, pageSize is 1 to 100.

## Rate limit

`/users/register`, `/users/login` and `/users/refresh` allow `RATE_LIMIT_REQUESTS` calls per client IP and path every `RATE_LIMIT_WINDOW_SECONDS`.
After that they return 429 `{ "error": "too many requests, try again later" }` with a `Retry-After` header (seconds).
Behind Traefik the client IP is the last address in `X-Forwarded-For`. The counters are in memory (`src/rate_limit.rs`).

## Validation

Request bodies are checked with the `validator` crate (rules are on the structs in `src/models/requests.rs`):

- name: 1 to 100 characters
- email: a valid email, saved in lowercase
- password: 6 to 128 characters

A bad value returns 400, for example `{ "error": "email: must be a valid email" }`.

## Cache

Users are cached in Valkey (`users-cache`) under the key `users:user:<id>` for `CACHE_TTL_SECONDS`.
The cache is used by `GET /users/{id}` and by the token check on every protected request (`/auth/verify`).
The cached data has no password hash.
The key is deleted when the user is changed, gets a new role, is deleted or logs out.
If Valkey is not running, the service logs a warning and reads from the database.

## Calls to other services

After register the service calls `POST {NOTIFICATIONS_URL}/notifications` with `{ userId, type: "welcome", message }`.
If notifications is not running, it only logs a warning and register still works.

## Tests

```powershell
cargo test
```

Unit tests cover passwords, JWT and refresh tokens. Endpoint tests in `src/endpoint_tests.rs` call the router directly and do not need a database.
One test needs the dev database (start it with the dev menu first):

```powershell
$env:TEST_DB_URL = "postgres://m3d_users:changeme@localhost:5432/m3d_users"
cargo test -- --ignored
```

## OpenAPI

The spec is served on `/openapi.json`. The same spec is saved in `openapi.json` in this folder.
After changing endpoints, update it with:

```powershell
Invoke-WebRequest http://localhost:8081/openapi.json -OutFile openapi.json
```
