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

## Ports

Ports and owners are in the services table in the [README](../README.md#services). Pick a free port there before you start.
