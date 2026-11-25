# CinemaGo

Cinema ticket booking app made of microservices, for the SUA course at FERI.
Users can browse movies, see screenings, book seats, pay (simulated) and leave reviews.

## Run it

1. Run `.\dev.ps1` on Windows or `./dev.sh` on Mac/Linux. It copies `.env.example` to `.env` if needed and starts everything
2. Open the gateway on http://localhost:8080 (frontend port TBD)

Other commands: `down`, `logs [service]`, `reset` (also deletes the databases), `ps`. For example `.\dev.ps1 logs users`.

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
