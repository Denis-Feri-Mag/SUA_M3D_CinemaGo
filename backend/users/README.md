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

## Env variables

| Name              | What it is                                  |
|-------------------|---------------------------------------------|
| PORT              | Port the service listens on                 |
| DB_URL            | Postgres connection string for m3d_users    |
| JWT_SECRET        | Secret used to sign tokens                  |
| NOTIFICATIONS_URL | Base URL of the notifications service       |

## Endpoints

| Method | Path                  | Token         | What it does                          |
|--------|-----------------------|---------------|---------------------------------------|
| GET    | /health               | no            | Health check                          |
| GET    | /users                | admin         | List all users                        |
| GET    | /users/{id}           | no            | Get one user                          |
| POST   | /users/register       | no            | Create a user (201)                   |
| POST   | /users/login          | no            | Returns a JWT                         |
| PUT    | /users/{id}           | self or admin | Change name and email                 |
| PUT    | /users/{id}/role      | admin         | Set role to `user` or `admin`         |
| DELETE | /users/{id}           | self or admin | Delete the user (204)                 |
| DELETE | /users/{id}/sessions  | self or admin | Logout, all old tokens stop working   |

Send the token as `Authorization: Bearer <token>`.
The first registered user becomes admin.
Passwords are hashed with argon2 and are never returned.

## Calls to other services

After register the service calls `POST {NOTIFICATIONS_URL}/notifications` (Mila) with `{ userId, type: "welcome", message }`.
If notifications is not running, it only logs a warning and register still works.

## OpenAPI

The spec is served on `/openapi.json`. The same spec is saved in `openapi.json` in this folder.
After changing endpoints, update it with:

```powershell
Invoke-WebRequest http://localhost:8081/openapi.json -OutFile openapi.json
```
