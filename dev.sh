#!/usr/bin/env bash
# Dev helper for docker compose
# Usage: ./dev.sh             interactive menu
#        ./dev.sh up|down|logs [service]|reset|ps

compose="docker compose -f docker-compose.yml -f docker-compose.dev.yml"
local_routes_file="traefik/local.yml"

# Host port of each service in dev (same as the README table)
dev_port() {
  case "$1" in
    users) echo 8081 ;;
    movies) echo 8082 ;;
    screenings) echo 8083 ;;
    reservations) echo 8084 ;;
    payments) echo 8085 ;;
    reviews) echo 8086 ;;
    notifications) echo 8087 ;;
    *) echo "?" ;;
  esac
}

# Routes that work without a token, same as the -public labels in docker-compose.yml
public_rule() {
  case "$1" in
    users) printf 'Path(`/api/users/register`) || Path(`/api/users/login`) || Path(`/api/users/refresh`)' ;;
    movies|screenings|reviews) printf 'Method(`GET`) && PathPrefix(`/api/%s`)' "$1" ;;
    *) printf '' ;;
  esac
}

create_env_file() {
  if [ ! -f .env ]; then
    cp .env.example .env
    echo "Created .env from .env.example"
  fi
}

# Enabled services without databases, caches and traefik
list_services() {
  $compose config --services 2>/dev/null | grep -v -e '-db$' -e '-cache$' -e '^traefik$' | sort
}

# Traefik routes to services running on this machine (read by Traefik from the traefik folder)
write_local_routes() {
  routers=""
  middlewares=""
  services=""
  for name in "$@"; do
    port=$(dev_port "$name")
    rule=$(public_rule "$name")
    routers+=$(printf '\n    %s-local:\n      rule: "PathPrefix(`/api/%s`)"\n      middlewares: [auth, strip-api]\n      service: %s-local' "$name" "$name" "$name")
    if [ -n "$rule" ]; then
      routers+=$(printf '\n    %s-local-public:\n      rule: "%s"\n      middlewares: [strip-api]\n      service: %s-local' "$name" "$rule" "$name")
    fi
    routers+=$(printf '\n    %s-local-docs:\n      rule: "Path(`/api/%s/health`) || PathPrefix(`/api/%s/docs`) || Path(`/api/%s/openapi.json`)"\n      middlewares: [%s-local-strip]\n      service: %s-local' "$name" "$name" "$name" "$name" "$name" "$name")
    middlewares+=$(printf '\n    %s-local-strip:\n      stripPrefix:\n        prefixes: [/api/%s]' "$name" "$name")
    services+=$(printf '\n    %s-local:\n      loadBalancer:\n        servers:\n          - url: http://host.docker.internal:%s' "$name" "$port")
  done
  printf '# Written by dev.sh, deleted on the next normal start\nhttp:\n  routers:%s\n  middlewares:%s\n  services:%s\n' "$routers" "$middlewares" "$services" > "$local_routes_file"
}

start_all() {
  create_env_file
  rm -f "$local_routes_file"
  $compose up --build -d && echo "Gateway: http://localhost:8080"
}

start_with_local_services() {
  create_env_file
  all_services=($(list_services))
  echo "Which services do you want to run locally?"
  for index in "${!all_services[@]}"; do
    echo "  $((index + 1))) ${all_services[$index]}"
  done
  read -r -p "Numbers (like 1 3 or 1,3): " answer || return

  local_services=()
  for number in ${answer//,/ }; do
    if ! [[ "$number" =~ ^[0-9]+$ ]] || [ "$number" -lt 1 ] || [ "$number" -gt "${#all_services[@]}" ]; then
      echo "Invalid number: $number"
      return
    fi
    local_services+=("${all_services[$((number - 1))]}")
  done
  if [ ${#local_services[@]} -eq 0 ]; then
    echo "No services chosen"
    return
  fi

  # Local services get no container, containers call them on the host (like USERS_URL=http://host.docker.internal:8081)
  scale_options=""
  url_variables=()
  for name in "${local_services[@]}"; do
    scale_options="$scale_options --scale $name=0"
    upper_name=$(echo "$name" | tr '[:lower:]' '[:upper:]')
    url_variables+=("${upper_name}_URL=http://host.docker.internal:$(dev_port "$name")")
  done
  write_local_routes "${local_services[@]}"
  env "${url_variables[@]}" $compose up --build -d $scale_options || return

  echo
  echo "Gateway: http://localhost:8080"
  for name in "${local_services[@]}"; do
    port=$(dev_port "$name")
    echo "$name runs locally:"
    echo "  cd backend/$name, copy .env.example to .env and start it on port $port"
    echo "  Swagger: http://localhost:$port/docs, through Traefik: http://localhost:8080/api/$name/..."
  done
}

# Runs one option and exits, only invalid input shows the menu again
show_menu() {
  while true; do
    echo
    echo "CinemaGo dev menu"
    echo "  1) Start all in Docker"
    echo "  2) Start, but run some services locally"
    echo "  3) Stop"
    echo "  4) Logs"
    echo "  5) Status"
    echo "  6) Reset (deletes databases)"
    echo "  0) Exit"
    read -r -p "Choose: " choice || exit 0

    case "$choice" in
      1) start_all ;;
      2) start_with_local_services ;;
      3) rm -f "$local_routes_file"; $compose down ;;
      4)
        read -r -p "Service (empty = all): " service_name
        $compose logs -f $service_name
        ;;
      5) $compose ps ;;
      6)
        read -r -p "Delete all databases? (y/N): " confirm
        if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
          rm -f "$local_routes_file"
          $compose down -v
        else
          echo "Cancelled"
        fi
        ;;
      0) ;;
      *) echo "Invalid choice: $choice"; continue ;;
    esac
    return
  done
}

case "$1" in
  "")    show_menu ;;
  up)    create_env_file; rm -f "$local_routes_file"; $compose up --build ;;
  down)  rm -f "$local_routes_file"; $compose down ;;
  logs)  $compose logs -f $2 ;;
  reset) rm -f "$local_routes_file"; $compose down -v ;;
  ps)    $compose ps ;;
  *)
    echo "Usage: ./dev.sh [up|down|logs [service]|reset|ps]"
    echo "  no argument  interactive menu"
    echo "  up           build and start everything"
    echo "  down         stop everything"
    echo "  logs         follow logs, optionally for one service"
    echo "  reset        stop and delete database volumes"
    echo "  ps           list running containers"
    ;;
esac
