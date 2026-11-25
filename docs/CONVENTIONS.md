# Conventions

## Git

- Never push straight to main, always make a PR
- Branch names: `feature/movies-crud`, `fix/seat-check`
- Short lowercase commit messages, like `add movie endpoints`
- At least 1 other person reviews the PR before merge

Tickets and PRs use the same plain style:

- Ticket title: `Movies: CRUD API`
- PR title: `Vaja01: movies crud api`
- Body starts with the tag line `Vaja01`, then one sentence and a few bullets
- PRs end with `closes #N` when they finish a ticket

## Services

- Every service is in its own folder in `backend/` with its own Dockerfile and README
- Every service has its own database, never read another service's database
- Database name and user are `m3d_<service>`, like `m3d_movies`. Only the password goes in `.env`
- Other services' URLs come from env variables, nothing hardcoded
- Root `.env` is for compose, `backend/<service>/.env` is for running that service locally. Keep both `.env.example` files up to date
- Every service has `GET /health` that returns 200
- Swagger UI on `/docs`, spec on `/openapi.json`

## API

- JSON everywhere, names in camelCase
- Times in ISO 8601 UTC, like `2026-11-20T18:30:00Z`
- Errors look like `{ "error": "movie not found" }` with the right status code

## Running

- Use `.\dev.ps1` (Windows) or `./dev.sh` (Mac/Linux) from the repo root, with `up`, `down`, `logs [service]`, `reset` or `ps`

## Gateway (Traefik)

- The browser only talks to Traefik on http://localhost:8080, every service is under `/api/<service>`
- Traefik removes `/api`, so `/api/movies/1` reaches your service as `/movies/1`
- To add your service, copy the label example at the top of `docker-compose.yml`
- Routes use the `auth` middleware by default. It asks the users service if the token is valid
- After that your service gets `X-User-Id` and `X-User-Role` headers, read them instead of checking the JWT yourself
- Only trust these headers on routes with `auth`. Public routes need their own router without `auth`
- Calls between services go directly (like `http://users:8080`), not through Traefik
- Dashboard (dev only): http://localhost:8090

## Ports

Ports and owners are in the services table in the [README](../README.md#services). Pick a free port there before you start.
