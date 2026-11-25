#!/usr/bin/env bash
# Dev helper for docker compose
# Usage: ./dev.sh [up|down|logs [service]|reset|ps]

command="${1:-up}"
service="$2"
compose="docker compose -f docker-compose.yml -f docker-compose.dev.yml"

case "$command" in
  up)
    if [ ! -f .env ]; then
      cp .env.example .env
      echo "Created .env from .env.example"
    fi
    $compose up --build
    ;;
  down)  $compose down ;;
  logs)  $compose logs -f $service ;;
  reset) $compose down -v ;;
  ps)    $compose ps ;;
  *)
    echo "Usage: ./dev.sh [up|down|logs [service]|reset|ps]"
    echo "  up     build and start everything (default)"
    echo "  down   stop everything"
    echo "  logs   follow logs, optionally for one service"
    echo "  reset  stop and delete database volumes"
    echo "  ps     list running containers"
    ;;
esac
