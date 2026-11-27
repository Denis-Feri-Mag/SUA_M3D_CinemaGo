#!/bin/sh
# Printed by the info container after docker compose up.
# When you enable your service, uncomment its line at the bottom
# (and its depends_on entry for info in docker-compose.yml).
#
# DEV_PORTS=true (set in docker-compose.dev.yml) also prints direct ports, databases and caches.
# LOCAL_SERVICES (set by the dev menu) lists services that run on your machine, like "users movies".

gateway="http://localhost:8080"
databases=""

# service <name> <dev port> "<databases and caches with their dev ports>"
service() {
  name=$1
  port=$2
  where="docker"
  direct="http://localhost:$port"
  case " $LOCAL_SERVICES " in
    *" $name "*)
      where="local"
      direct="$direct (start it yourself)"
      ;;
  esac
  if [ "$DEV_PORTS" = "true" ]; then
    printf '  %-14s %-7s %-40s %-46s %s\n' "$name" "$where" "$gateway/api/$name" "$gateway/api/$name/docs/" "$direct"
    databases="$databases$3"
  else
    printf '  %-14s %-40s %s\n' "$name" "$gateway/api/$name" "$gateway/api/$name/docs/"
  fi
}

echo
echo "CinemaGo is running"
printf '  %-12s %s\n' "Gateway" "$gateway"
if [ "$DEV_PORTS" = "true" ]; then
  printf '  %-12s %s\n' "Traefik UI" "http://localhost:8090"
  echo
  printf '  %-14s %-7s %-40s %-46s %s\n' "Service" "Where" "Gateway" "Swagger" "Direct"
else
  echo
  printf '  %-14s %-40s %s\n' "Service" "Gateway" "Swagger"
fi

# Enabled services
service users 8081 " users-db=5432 users-cache=6379"
# service movies 8082 " movies-db=5433"
# service screenings 8083 " screenings-db=5434"
service reservations 8084 " reservations-db=5435"
# service payments 8085 " payments-db=5436"
# service reviews 8086 " reviews-db=5437"
# service notifications 8087 " notifications-db=5438"

if [ "$DEV_PORTS" = "true" ]; then
  echo
  echo "  Databases and caches (from your machine)"
  for entry in $databases; do
    printf '  %-18s localhost:%s\n' "${entry%%=*}" "${entry#*=}"
  done
fi
echo
