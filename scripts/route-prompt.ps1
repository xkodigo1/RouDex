param(
    [string]$RouterPath = (Join-Path $PSScriptRoot 'router-cli.exe')
)

$ErrorActionPreference = 'Stop'
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = $OutputEncoding
[Console]::InputEncoding = $OutputEncoding
try {
    $inputText = [Console]::In.ReadToEnd()
    if (-not (Test-Path -LiteralPath $RouterPath -PathType Leaf)) {
        throw 'Router executable is unavailable.'
    }
    $output = $inputText | & $RouterPath route-prompt
    if ($LASTEXITCODE -ne 0) {
        throw 'Router process failed.'
    }
    $response = ($output -join "`n") | ConvertFrom-Json -ErrorAction Stop
    if ($response -isnot [System.Management.Automation.PSCustomObject]) {
        throw 'Router response must be a JSON object.'
    }
    $internalWorker = (Test-Path Env:ROUDEX_ROUTER_INTERNAL) -or (Test-Path Env:ROUDEX_ACTIVE_TASK_ID)
    $blocked = ($response.continue -is [bool]) -and ($response.continue -eq $false) -and ($response.decision -is [string]) -and ($response.decision -eq 'block')
    $workerPass = $internalWorker -and ($response.continue -is [bool]) -and ($response.continue -eq $true)
    if (-not ($blocked -or $workerPass)) {
        throw 'Router returned an invalid hook decision.'
    }
    $response | ConvertTo-Json -Compress -Depth 20
} catch {
    @{ continue = $false; decision = 'block'; reason = 'RouDex ingress failed. Restore the local router before retrying; execution is blocked.' } | ConvertTo-Json -Compress
}
exit 0
