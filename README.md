# CinemaGo

Cinema ticket booking app made of microservices, for the SUA course at FERI.
Users can browse movies, see screenings, book seats, pay (simulated) and leave reviews.

## Run it

1. Run `.\dev.ps1` on Windows or `./dev.sh` on Mac/Linux. It opens a menu, runs the option you pick and exits. It copies `.env.example` to `.env` if needed
2. Choose `1` to start everything in Docker, or `2` to run some services locally (see below)
3. When everything is up, the `info` container prints a table with the gateway, Swagger and direct URL of every service and the database ports. Open the gateway on http://localhost:8080 (frontend port TBD)

To see the table again: `docker compose -f docker-compose.yml -f docker-compose.dev.yml logs --no-log-prefix info`.

Quick commands without the menu: `up`, `down`, `logs [service]`, `reset` (also deletes the databases), `ps`. For example `.\dev.ps1 logs users`.

## Run one service locally

1. Run the menu and choose `2`, then the number of your service. Everything else starts in Docker, your service gets no container
2. Copy `backend/<service>/.env.example` to `.env` in the same folder and start your service (load the file with `dotenvy` for Rust, `DotNetEnv` for C#, `dotenv` for Node, `python-dotenv` for Python)
3. Use it directly on `http://localhost:<port>/docs` or through Traefik on `http://localhost:8080/api/<service>/...`

Your service reaches the others and its database on `localhost` with their dev ports. The containers reach your service on `host.docker.internal:<port>`, the menu sets this up. See "Running locally" in [docs/CONVENTIONS.md](docs/CONVENTIONS.md).

## Services

| Service                | Folder                | Framework   | Port | Owner    | Git user |
|------------------------|-----------------------|-------------|------|----------|----------|
| users                  | backend/users         | Rust (axum) | 8081 | Denis    | TBD      |
| movies                 | backend/movies        | TBD         | 8082 | Marija   | TBD      |
| screenings             | backend/screenings    | TBD         | 8083 | Matea    | TBD      |
| reservations           | backend/reservations  | Rust (axum) | 8084 | Denis    | TBD      |
| payments               | backend/payments      | TBD         | 8085 | Matea    | TBD      |
| reviews                | backend/reviews       | TBD         | 8086 | Mila     | TBD      |
| notifications          | backend/notifications | TBD         | 8087 | Mila     | TBD      |
| frontend               | frontend              | TBD         | TBD  | Marija   | TBD      |
| gateway, docs, compose |                       | Traefik v3  |      | everyone | everyone |

More info is in [docs/CONVENTIONS.md](docs/CONVENTIONS.md) and in each service's own README.

## Team

M3D group: Matea, Mila, Marija, Denis
