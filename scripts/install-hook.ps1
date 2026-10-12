param(
    [switch]$Release,
    [switch]$Managed
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$target = if ($Release) { "release" } else { "debug" }
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$cargo = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe" }
$source = Join-Path $repoRoot "target\$target\router-cli.exe"
$installRoot = Join-Path $env:LOCALAPPDATA "RouDex\bin"
$destination = Join-Path $installRoot "router-cli.exe"

Push-Location $repoRoot
try {
    if ($Release) {
        & $cargo build -p router-cli --release
    } else {
        & $cargo build -p router-cli
    }
    if ($LASTEXITCODE -ne 0) {
        throw "No se pudo compilar router-cli."
    }
} finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $source)) {
    throw "No se encontró el ejecutable compilado: $source"
}

New-Item -ItemType Directory -Path $installRoot -Force | Out-Null
Copy-Item -LiteralPath $source -Destination $destination -Force
$installMode = if ($Managed) { "install-managed-hook" } else { "install-hook" }
& $destination $installMode
if ($LASTEXITCODE -ne 0) {
    throw "No se pudo instalar el hook de mensajes."
}

Write-Output "Hook instalado ($installMode): $destination"
