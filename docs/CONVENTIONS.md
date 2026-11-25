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

## Networks

- `public`: only Traefik, it is the only container with a published port (8080)
- `backend` (internal): Traefik and all services, services call each other here
- `<service>-db-net` (internal): one service and its database, no other service can reach that database
- Internal networks have no internet. If your service calls an outside API, also add it to `public`
- In dev every service and database also joins the `dev` network so their ports can be published

## Adding your service

- Uncomment your service, database, network and volume blocks in `docker-compose.yml`
- Uncomment your blocks in `docker-compose.dev.yml` to get your host ports (see the README table)
- Routes are protected by default. The movies, screenings and reviews blocks show a public GET router

## Gateway (Traefik)

- The browser only talks to Traefik on http://localhost:8080, every service is under `/api/<service>`
- Traefik removes `/api`, so `/api/movies/1` reaches your service as `/movies/1`
- Routes use the `auth` middleware by default. It asks the users service if the token is valid
- After that your service gets `X-User-Id` and `X-User-Role` headers, read them instead of checking the JWT yourself
- Only trust these headers on routes with `auth`. Public routes need their own router without `auth`
- Calls between services go directly (like `http://users:8080`), not through Traefik
- Dashboard (dev only): http://localhost:8090

## Ports

Dev host ports and owners are in the services table in the [README](../README.md#services). Databases use host ports 5432 to 5438 in the same order. Inside docker every service listens on 8080.
