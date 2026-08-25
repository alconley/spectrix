param(
    [string]$Python = ".venv\Scripts\python.exe",
    [switch]$ReportOnly
)

$ErrorActionPreference = "Stop"
$nativeOutput = cargo bench -p spectrix-fitting --bench performance --quiet 2>&1
$nativeLine = $nativeOutput | Where-Object { $_ -match '^\{' } | Select-Object -Last 1
if (-not $nativeLine) {
    throw "Native benchmark did not emit JSON: $nativeOutput"
}
$oracleOutput = & $Python crates/spectrix-fitting/benches/lmfit_performance.py
$native = $nativeLine | ConvertFrom-Json -AsHashtable
$oracle = $oracleOutput | ConvertFrom-Json -AsHashtable

$speedups = [ordered]@{}
$logSum = 0.0
$slowerCases = @()
foreach ($case in ($native.Keys | Sort-Object)) {
    $speedup = [double]$oracle[$case] / [double]$native[$case]
    $speedups[$case] = $speedup
    $logSum += [Math]::Log($speedup)
    if ($speedup -lt 1.0) {
        $slowerCases += $case
    }
}
$geometricMean = [Math]::Exp($logSum / $speedups.Count)
$gatePassed = $slowerCases.Count -eq 0 -and $geometricMean -ge 5.0

$report = [ordered]@{
    generated_utc = [DateTime]::UtcNow.ToString("o")
    units = "microseconds"
    native = $native
    lmfit_1_3_4 = $oracle
    speedup = $speedups
    geometric_mean_speedup = $geometricMean
    slower_cases = $slowerCases
    gate_passed = $gatePassed
}
$report | ConvertTo-Json -Depth 5

if (-not $ReportOnly -and -not $gatePassed) {
    throw "Performance gate failed: geometric mean ${geometricMean}x; slower cases: $($slowerCases -join ', ')"
}
