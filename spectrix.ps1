# Bootstrap Spectrix on Windows without requiring Python up front.
# uv installs the Python version pinned by .python-version.
param(
    [switch] $CheckOnly,
    [switch] $DryRun,
    [switch] $DebugBuild,
    [switch] $Info,
    [switch] $Debug,
    [switch] $ResetState,
    [switch] $NoSync
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = $PSScriptRoot
$LauncherArguments = @($args)

function Write-ProofPlan {
    $proofArguments = @($LauncherArguments)
    if ($DebugBuild) { $proofArguments = @('--debug-build') + $proofArguments }
    if ($Info) { $proofArguments = @('--info') + $proofArguments }
    if ($Debug) { $proofArguments = @('--debug') + $proofArguments }
    if ($ResetState) { $proofArguments = @('--reset-state') + $proofArguments }
    if ($NoSync) { $proofArguments = @('--no-sync') + $proofArguments }
    Write-Output 'Spectrix bootstrap proof: windows'
    Write-Output '  check/install: Visual Studio C++ Build Tools'
    Write-Output '  check/install: uv'
    Write-Output '  check/install: rustup and Cargo'
    Write-Output '  provision:     Python from .python-version via uv'
    Write-Output '  synchronize:   Python packages from uv.lock'
    Write-Output "  launch:        spectrix.py $($proofArguments -join ' ')"
}

if ($DryRun) {
    if ($Info -and $Debug) {
        throw '-Info and -Debug cannot be used together.'
    }
    Write-ProofPlan
    exit 0
}

function Add-BootstrapPaths {
    $paths = @(
        (Join-Path $HOME '.local\bin'),
        (Join-Path $HOME '.cargo\bin'),
        (Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Links')
    )
    $env:Path = (($paths + @($env:Path)) -join [IO.Path]::PathSeparator)
}

function Test-VcBuildTools {
    if (Get-Command cl.exe -ErrorAction SilentlyContinue) {
        return $true
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) {
        return $false
    }

    $installation = & $vswhere -latest -products '*' `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath
    return $LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($installation)
}

function Get-ToolVersion([string] $Name) {
    $tool = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $tool) {
        Write-Host "[missing] $Name"
        return $false
    }
    # Do not execute Cargo here: the rustup proxy can install the repository's
    # pinned toolchain, which would violate CheckOnly's read-only contract.
    Write-Host "[ok] $Name"
    return $true
}

Add-BootstrapPaths

if ($Info -and $Debug) {
    throw '-Info and -Debug cannot be used together.'
}

if ($CheckOnly) {
    Write-Output 'Spectrix prerequisite check: windows'
    $ready = $true
    if (Test-VcBuildTools) {
        Write-Output '[ok] Visual Studio C++ Build Tools'
    }
    else {
        Write-Output '[missing] Visual Studio C++ Build Tools with the VCTools workload'
        $ready = $false
    }
    if (-not (Get-ToolVersion uv)) { $ready = $false }
    if (-not (Get-ToolVersion rustup)) { $ready = $false }
    if (-not (Get-ToolVersion cargo)) { $ready = $false }
    if ((Get-Command uv -ErrorAction SilentlyContinue) -and
        ((& uv python find 3.13 2>$null) -ne $null)) {
        Write-Output '[ok] Python 3.13 available to uv'
    }
    else {
        Write-Output '[missing] Python 3.13 (uv installs it during normal bootstrap)'
        $ready = $false
    }
    if ($ready) { exit 0 } else { exit 1 }
}

function Invoke-WingetInstall {
    param(
        [Parameter(Mandatory = $true)][string] $Id,
        [string[]] $OverrideArguments = @()
    )
    $arguments = @(
        'install', '--id', $Id, '--exact', '--source', 'winget',
        '--accept-package-agreements', '--accept-source-agreements'
    ) + $OverrideArguments
    & winget @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "winget failed to install $Id (exit code $LASTEXITCODE)"
    }
}

function Install-VcBuildTools {
    if (Test-VcBuildTools) { return }
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw 'Visual Studio C++ Build Tools are missing and winget is unavailable. Install the VCTools workload from https://visualstudio.microsoft.com/visual-cpp-build-tools/'
    }
    Write-Output 'Installing Visual Studio 2022 C++ Build Tools (a UAC prompt may appear)...'
    Invoke-WingetInstall -Id 'Microsoft.VisualStudio.2022.BuildTools' -OverrideArguments @(
        '--override',
        '--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
    )
}

function Install-Uv {
    if (Get-Command uv -ErrorAction SilentlyContinue) { return }
    Write-Output "Installing uv from Astral's official installer..."
    $installScript = Join-Path ([IO.Path]::GetTempPath()) "spectrix-uv-install-$PID.ps1"
    try {
        Invoke-WebRequest -UseBasicParsing -Uri 'https://astral.sh/uv/install.ps1' -OutFile $installScript
        & $installScript
    }
    finally {
        if (Test-Path -LiteralPath $installScript) {
            Remove-Item -LiteralPath $installScript -Force
        }
    }
    Add-BootstrapPaths
    if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
        throw 'uv installation completed but uv was not found on PATH'
    }
}

function Install-Rust {
    if ((Get-Command rustup -ErrorAction SilentlyContinue) -and
        (Get-Command cargo -ErrorAction SilentlyContinue)) { return }
    Write-Output 'Installing Rust from the official rustup installer...'
    $rustup = Join-Path ([IO.Path]::GetTempPath()) "spectrix-rustup-$PID.exe"
    try {
        Invoke-WebRequest -UseBasicParsing -Uri 'https://win.rustup.rs/x86_64' -OutFile $rustup
        & $rustup -y --profile minimal
        if ($LASTEXITCODE -ne 0) {
            throw "rustup failed (exit code $LASTEXITCODE)"
        }
    }
    finally {
        if (Test-Path -LiteralPath $rustup) {
            Remove-Item -LiteralPath $rustup -Force
        }
    }
    Add-BootstrapPaths
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Rust installation completed but Cargo was not found on PATH'
    }
}

Install-VcBuildTools
Install-Uv
Install-Rust

Push-Location $ProjectRoot
try {
    if ($NoSync) {
        Write-Output 'Skipping Python dependency synchronization.'
    }
    else {
        Write-Output 'Synchronizing the locked Python 3.13 environment...'
        & uv sync --locked
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    $arguments = @('--no-sync') + $LauncherArguments
    if ($DebugBuild) { $arguments = @('--debug-build') + $arguments }
    if ($Info) { $arguments = @('--info') + $arguments }
    if ($Debug) { $arguments = @('--debug') + $arguments }
    if ($ResetState) { $arguments = @('--reset-state') + $arguments }

    Write-Output 'Launching Spectrix...'
    & uv run --locked --no-sync python (Join-Path $ProjectRoot 'spectrix.py') @arguments
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
