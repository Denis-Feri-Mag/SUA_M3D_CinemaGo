# Dev helper for docker compose
# Usage: .\dev.ps1             interactive menu
#        .\dev.ps1 up|down|logs [service]|reset|ps

param(
    [string]$Command = "",
    [string]$Service = ""
)

$compose = @("compose", "-f", "docker-compose.yml", "-f", "docker-compose.dev.yml")
$localRoutesFile = "traefik/local.yml"

# Host port of each service in dev (same as the README table)
$devPorts = @{
    users         = 8081
    movies        = 8082
    screenings    = 8083
    reservations  = 8084
    payments      = 8085
    reviews       = 8086
    notifications = 8087
}

# Routes that work without a token, same as the -public labels in docker-compose.yml
function Get-PublicRule($name) {
    if ($name -eq "users") { return 'Path(`/api/users/register`) || Path(`/api/users/login`) || Path(`/api/users/refresh`)' }
    if ($name -in @("movies", "screenings", "reviews")) { return "Method(``GET``) && PathPrefix(``/api/$name``)" }
    return ""
}

# Traefik routes to services running on this machine (read by Traefik from the traefik folder)
function Write-LocalRoutes($localServices) {
    $routers = @()
    $middlewares = @()
    $services = @()
    foreach ($name in $localServices) {
        $port = $devPorts[$name]
        $publicRule = Get-PublicRule $name
        $routers += "    $name-local:", "      rule: ""PathPrefix(``/api/$name``)""", "      middlewares: [auth, strip-api]", "      service: $name-local"
        if ($publicRule) {
            $routers += "    $name-local-public:", "      rule: ""$publicRule""", "      middlewares: [strip-api]", "      service: $name-local"
        }
        $routers += "    $name-local-docs:", "      rule: ""Path(``/api/$name/health``) || PathPrefix(``/api/$name/docs``) || Path(``/api/$name/openapi.json``)""", "      middlewares: [$name-local-strip]", "      service: $name-local"
        $middlewares += "    $name-local-strip:", "      stripPrefix:", "        prefixes: [/api/$name]"
        $services += "    $name-local:", "      loadBalancer:", "        servers:", "          - url: http://host.docker.internal:$port"
    }
    $lines = @("# Written by dev.ps1, deleted on the next normal start", "http:", "  routers:") + $routers + @("  middlewares:") + $middlewares + @("  services:") + $services
    Set-Content -Path $localRoutesFile -Value $lines
}

function Remove-LocalRoutes {
    if (Test-Path $localRoutesFile) { Remove-Item $localRoutesFile }
}

function Start-All {
    New-EnvFile
    Remove-LocalRoutes
    docker @compose up --build -d
    if ($LASTEXITCODE -eq 0) { Write-Host "Gateway: http://localhost:8080" }
}

function New-EnvFile {
    if (-not (Test-Path ".env")) {
        Copy-Item ".env.example" ".env"
        Write-Host "Created .env from .env.example"
    }
}

# Enabled services without databases and traefik
function Get-Services {
    $allServices = docker @compose config --services 2>$null
    return @($allServices | Where-Object { $_ -notlike "*-db" -and $_ -ne "traefik" } | Sort-Object)
}

function Start-WithLocalServices {
    New-EnvFile
    $services = @(Get-Services)
    Write-Host "Which services do you want to run locally?"
    for ($index = 0; $index -lt $services.Count; $index++) {
        Write-Host "  $($index + 1)) $($services[$index])"
    }
    $answer = Read-Host "Numbers (like 1 3 or 1,3)"
    if ($null -eq $answer) { return }

    $localServices = @()
    foreach ($number in ($answer -split "[ ,]+" | Where-Object { $_ -ne "" })) {
        if ($number -notmatch "^[0-9]+$" -or [int]$number -lt 1 -or [int]$number -gt $services.Count) {
            Write-Host "Invalid number: $number"
            return
        }
        $localServices += $services[[int]$number - 1]
    }
    if ($localServices.Count -eq 0) {
        Write-Host "No services chosen"
        return
    }

    # Local services get no container, containers call them on the host (like USERS_URL=http://host.docker.internal:8081)
    $scaleOptions = @()
    foreach ($name in $localServices) {
        $scaleOptions += @("--scale", "$name=0")
        Set-Item -Path "Env:$($name.ToUpper())_URL" -Value "http://host.docker.internal:$($devPorts[$name])"
    }
    Write-LocalRoutes $localServices
    docker @compose up --build -d @scaleOptions
    $exitCode = $LASTEXITCODE

    # Remove the URLs again so the next start uses the containers
    foreach ($name in $localServices) {
        Remove-Item -Path "Env:$($name.ToUpper())_URL"
    }
    if ($exitCode -ne 0) { return }

    Write-Host ""
    Write-Host "Gateway: http://localhost:8080"
    foreach ($name in $localServices) {
        $port = $devPorts[$name]
        Write-Host "$name runs locally:"
        Write-Host "  cd backend/$name, copy .env.example to .env and start it on port $port"
        Write-Host "  Swagger: http://localhost:$port/docs, through Traefik: http://localhost:8080/api/$name/..."
    }
}

# Runs one option and exits, only invalid input shows the menu again
function Show-Menu {
    :menuLoop while ($true) {
        Write-Host ""
        Write-Host "CinemaGo dev menu"
        Write-Host "  1) Start all in Docker"
        Write-Host "  2) Start, but run some services locally"
        Write-Host "  3) Stop"
        Write-Host "  4) Logs"
        Write-Host "  5) Status"
        Write-Host "  6) Reset (deletes databases)"
        Write-Host "  0) Exit"
        $choice = Read-Host "Choose"
        # No more input (for example piped input ended)
        if ($null -eq $choice) { return }

        switch ($choice) {
            "1" { Start-All }
            "2" { Start-WithLocalServices }
            "3" {
                Remove-LocalRoutes
                docker @compose down
            }
            "4" {
                $serviceName = Read-Host "Service (empty = all)"
                if ($serviceName) { docker @compose logs -f $serviceName } else { docker @compose logs -f }
            }
            "5" { docker @compose ps }
            "6" {
                $confirm = Read-Host "Delete all databases? (y/N)"
                if ($confirm -eq "y" -or $confirm -eq "Y") {
                    Remove-LocalRoutes
                    docker @compose down -v
                }
                else { Write-Host "Cancelled" }
            }
            "0" { }
            default {
                Write-Host "Invalid choice: $choice"
                continue menuLoop
            }
        }
        return
    }
}

switch ($Command) {
    ""      { Show-Menu }
    "up"    {
        New-EnvFile
        Remove-LocalRoutes
        docker @compose up --build
    }
    "down"  {
        Remove-LocalRoutes
        docker @compose down
    }
    "logs"  {
        if ($Service) { docker @compose logs -f $Service } else { docker @compose logs -f }
    }
    "reset" {
        Remove-LocalRoutes
        docker @compose down -v
    }
    "ps"    { docker @compose ps }
    default {
        Write-Host "Usage: .\dev.ps1 [up|down|logs [service]|reset|ps]"
        Write-Host "  no argument  interactive menu"
        Write-Host "  up           build and start everything"
        Write-Host "  down         stop everything"
        Write-Host "  logs         follow logs, optionally for one service"
        Write-Host "  reset        stop and delete database volumes"
        Write-Host "  ps           list running containers"
    }
}
