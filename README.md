# CinemaGo

Cinema ticket booking app made of microservices, for the SUA course at FERI.
Users can browse movies, see screenings, book seats, pay (simulated) and leave reviews.

## Run it

1. Copy `.env.example` to `.env`
2. Run `docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build`
3. Open the frontend (port TBD)

## Run one service locally

Start the other services with compose, then run yours on the host.
Copy `backend/<service>/.env.example` to `.env` in the same folder and load it with a dotenv library (`dotenvy` for Rust, `DotNetEnv` for C#, `dotenv` for Node, `python-dotenv` for Python).
Locally, other services and databases are on `localhost` with their host ports. Containers reach your service at `host.docker.internal`.
On Linux, compose needs `extra_hosts: ["host.docker.internal:host-gateway"]` for that name to work. See `docker-compose.dev.yml`.

## Services

| Service                | Folder                | Port | Owner    | Git user |
|------------------------|-----------------------|------|----------|----------|
| users                  | backend/users         | TBD  | TBD      | TBD      |
| movies                 | backend/movies        | TBD  | TBD      | TBD      |
| screenings             | backend/screenings    | TBD  | TBD      | TBD      |
| reservations           | backend/reservations  | TBD  | TBD      | TBD      |
| payments               | backend/payments      | TBD  | TBD      | TBD      |
| reviews                | backend/reviews       | TBD  | TBD      | TBD      |
| notifications          | backend/notifications | TBD  | TBD      | TBD      |
| frontend               | frontend              | TBD  | TBD      | TBD      |
| gateway, docs, compose |                       |      | everyone | everyone |

More info is in [docs/CONVENTIONS.md](docs/CONVENTIONS.md) and in each service's own README.

## Team

M3D group: Matea, Mila, Marija, Denis
