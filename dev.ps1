# Dev helper for docker compose
# Usage: .\dev.ps1 [up|down|logs [service]|reset|ps]

param(
    [string]$Command = "up",
    [string]$Service = ""
)

$compose = @("compose", "-f", "docker-compose.yml", "-f", "docker-compose.dev.yml")

switch ($Command) {
    "up" {
        if (-not (Test-Path ".env")) {
            Copy-Item ".env.example" ".env"
            Write-Host "Created .env from .env.example"
        }
        docker @compose up --build
    }
    "down"  { docker @compose down }
    "logs"  {
        if ($Service) { docker @compose logs -f $Service } else { docker @compose logs -f }
    }
    "reset" { docker @compose down -v }
    "ps"    { docker @compose ps }
    default {
        Write-Host "Usage: .\dev.ps1 [up|down|logs [service]|reset|ps]"
        Write-Host "  up     build and start everything (default)"
        Write-Host "  down   stop everything"
        Write-Host "  logs   follow logs, optionally for one service"
        Write-Host "  reset  stop and delete database volumes"
        Write-Host "  ps     list running containers"
    }
}
