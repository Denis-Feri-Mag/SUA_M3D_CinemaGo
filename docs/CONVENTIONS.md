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

- Run `.\dev.ps1` (Windows) or `./dev.sh` (Mac/Linux) from the repo root for the menu: start all, start with some services local, stop, logs, status, reset
- Quick commands without the menu: `up`, `down`, `logs [service]`, `reset`, `ps`

## Running locally (dev only)

Every call to another service uses an env variable like `USERS_URL`, never a fixed address.

| Service       | URL variable      | In Docker (default)         | From a local service     | Dev port | DB port |
|---------------|-------------------|-----------------------------|--------------------------|----------|---------|
| users         | USERS_URL         | http://users:8080           | http://localhost:8081    | 8081     | 5432    |
| movies        | MOVIES_URL        | http://movies:8080          | http://localhost:8082    | 8082     | 5433    |
| screenings    | SCREENINGS_URL    | http://screenings:8080      | http://localhost:8083    | 8083     | 5434    |
| reservations  | RESERVATIONS_URL  | http://reservations:8080    | http://localhost:8084    | 8084     | 5435    |
| payments      | PAYMENTS_URL      | http://payments:8080        | http://localhost:8085    | 8085     | 5436    |
| reviews       | REVIEWS_URL       | http://reviews:8080         | http://localhost:8086    | 8086     | 5437    |
| notifications | NOTIFICATIONS_URL | http://notifications:8080   | http://localhost:8087    | 8087     | 5438    |

- In Docker the defaults are set in `docker-compose.yml`, like `${MOVIES_URL:-http://movies:8080}`
- Your `backend/<service>/.env.example` is for running locally: `PORT` is your dev port, `DB_URL` uses `localhost:<your DB port>`, other services use the "From a local service" column
- A Docker service calling a locally running one needs `http://host.docker.internal:<port>`. When you pick local services in the menu, it sets their URL variables to that for all containers
- `host.docker.internal` works on Linux because of `extra_hosts` in `docker-compose.dev.yml`, Docker Desktop has it built in
- The menu also writes `traefik/local.yml` so Traefik routes `/api/<service>` to your local service. If users runs locally, the `auth` middleware calls it there too

## Networks

- `public`: only Traefik, it is the only container with a published port (8080)
- `backend` (internal): Traefik and all services, services call each other here
- `<service>-db-net` (internal): one service and its database, no other service can reach that database
- Internal networks have no internet. If your service calls an outside API, also add it to `public`
- In dev every service and database also joins the `dev` network so their ports can be published

## Adding your service

- Uncomment your service, database, network and volume blocks in `docker-compose.yml`
- Uncomment your blocks in `docker-compose.dev.yml` to get your host ports (see the README table)
- Use `auth@file` and `strip-api@file` in your labels, they are defined in `traefik/dynamic.yml`
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
