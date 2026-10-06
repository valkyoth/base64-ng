#requires -Version 7.0
[CmdletBinding()]
param([switch] $Full, [switch] $Benchmark)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. "$PSScriptRoot/windows_gate.ps1"
if ($Benchmark -and -not $Full) { throw '-Benchmark requires -Full' }
if (-not $IsWindows -or [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64') {
    throw 'Native Windows x86_64 is required; WSL, Wine and cross-compiles are not native evidence.'
}
$root = (Resolve-Path "$PSScriptRoot/..").Path
$target = 'x86_64-pc-windows-msvc'
$active = (Select-String -Path "$root/rust-toolchain.toml" -Pattern '^channel = "([0-9.]+)"$').Matches.Groups[1].Value
if (-not $active) { throw 'Missing pinned Rust version' }
$compilers = @($active)
if ($Full) { $compilers += '1.90.0' }
$saved = @{}
$variables = @('CARGO_TARGET_DIR', 'CARGO_BUILD_BUILD_DIR', 'CARGO_BUILD_TARGET', 'CARGO_INCREMENTAL',
    'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTDOCFLAGS', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
    'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER', 'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS')
foreach ($key in $variables) { $saved[$key] = [Environment]::GetEnvironmentVariable($key) }
$records = [System.Collections.Generic.List[object]]::new()
$report = [ordered]@{ schema = 1; scope = 'native Windows compatibility; not performance admission'
    status = 'running'; target = $target; full = [bool]$Full; commands = $records
    unavailable_tools = @('Miri', 'Kani', 'Linux sanitizers and fuzz campaigns')
    assembly_review = 'pending'; performance = 'not run' }
Push-Location $root
$destination = Join-Path $root ("target/windows-evidence/" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $destination | Out-Null
try {
    foreach ($key in $variables) { Set-WindowsEnvironment $key $null }
    $env:CARGO_INCREMENTAL = '0'
    $env:RUSTDOCFLAGS = '-D warnings'
    $report.os = Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber, OSArchitecture
    $report.cpu = @(Get-CimInstance Win32_Processor | Select-Object Name, Manufacturer, NumberOfCores, NumberOfLogicalProcessors)
    function Run([string] $Tool, [string[]] $Arguments) {
        Invoke-WindowsCommand $Tool $Arguments (Join-Path $destination ("{0:D3}.log" -f $records.Count)) $root $records
    }
    $report.powershell = $PSVersionTable.PSVersion.ToString()
    $null = Run python @('-c', 'import sys,tomllib; assert sys.version_info >= (3,11); print(sys.version)')
    if ($Full) { $null = Run openssl @('version') }
    $report.commit = ((Run git @('rev-parse', 'HEAD')) -join '').Trim()
    $report.dirty = @((Run git @('status', '--porcelain')))
    $files = @(Run git @('ls-files', '--cached', '--others', '--exclude-standard'))
    $report.source_sha256 = [ordered]@{}
    foreach ($file in ($files | Sort-Object -Unique)) {
        if (Test-Path -LiteralPath $file -PathType Leaf) {
            $report.source_sha256[$file] = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
    $report.probes = [ordered]@{}
    foreach ($compiler in $compilers) {
        $rust = @(Run rustc @("+$compiler", '-vV'))
        if ($rust -notcontains "host: $target") { throw "Rust $compiler is not a native $target compiler" }
        $null = Run cargo @("+$compiler", '--version')
        $env:CARGO_TARGET_DIR = Join-Path $destination "build/$compiler"
        $env:CARGO_BUILD_BUILD_DIR = Join-Path $env:CARGO_TARGET_DIR 'build'
        $null = Run python @('scripts/test-windows-abi-contract.py', '--toolchain', $compiler)
        foreach ($command in (Get-WindowsCommands $compiler -Full:$Full)) { $null = Run cargo $command }
        foreach ($feature in @('plain', 'checked')) {
            $args = @("+$compiler", 'run', '--locked', '--release', '--target', $target,
                '--manifest-path', 'portability/windows_native/Cargo.toml')
            if ($feature -eq 'checked') { $args += @('--features', 'checked') }
            $output = @(Run cargo $args)
            $probe = ($output | Where-Object { $_.StartsWith('{') }) | ConvertFrom-Json
            if (-not $probe.native_windows_msvc -or $probe.xmm6_xmm15 -ne 'passed') { throw 'Missing native ABI probe result' }
            $report.probes["$compiler/$feature"] = $probe
            $testArgs = @($args)
            $testArgs[1] = 'test'
            $null = Run cargo $testArgs
        }
        if ($Full) {
            $null = Run python @('scripts/check-companion-features.py', '--toolchain', $compiler)
            foreach ($isa in @(
                @('ssse3_sse41', '+ssse3,+sse4.1'), @('avx2', '+avx2'),
                @('avx512_vbmi', '+avx512f,+avx512bw,+avx512vl,+avx512vbmi'))) {
                if (-not $probe.($isa[0])) { continue }
                $env:RUSTFLAGS = "-C target-feature=$($isa[1])"
                $null = Run cargo @("+$compiler", 'test', '--locked', '--release', '--target', $target,
                    '--manifest-path', 'portability/windows_native/Cargo.toml', '--all-features')
            }
            Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
            $null = Run cargo @("+$compiler", 'rustc', '--locked', '--release', '--target', $target,
                '--all-features', '--lib', '--', '--emit=asm')
        }
    }
    if ($Benchmark) {
        $env:CARGO_TARGET_DIR = Join-Path $destination 'benchmark-build'
        $env:CARGO_BUILD_BUILD_DIR = Join-Path $env:CARGO_TARGET_DIR 'build'
        $env:RUSTFLAGS = '--cfg base64_ng_perf_evidence'
        $null = Run cargo @("+$active", 'build', '--locked', '--release', '--target', $target,
            '--manifest-path', 'perf/public-api/Cargo.toml', '--no-default-features', '--features', 'std,simd,validation-policy')
        $binary = Join-Path $env:CARGO_TARGET_DIR "$target/release/base64-ng-public-api-perf.exe"
        $null = Run python @('scripts/measure-windows-public-api.py', $binary, (Join-Path $destination 'performance.json'))
        $report.performance = 'performance.json (exploratory paired public calls; not admission)'
    }
    $report.artifact_sha256 = [ordered]@{}
    foreach ($artifact in (Get-ChildItem $destination -Recurse -File | Where-Object { $_.Extension -in @('.s', '.exe', '.json') })) {
        $name = [IO.Path]::GetRelativePath($destination, $artifact.FullName)
        $report.artifact_sha256[$name] = (Get-FileHash $artifact.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    foreach ($file in $report.source_sha256.Keys) {
        if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $report.source_sha256[$file]) {
            throw "Source changed during verification: $file"
        }
    }
    $report.status = 'passed'
} catch {
    $report.status = 'failed'
    throw
} finally {
    $report.commands = $records.ToArray()
    $report | ConvertTo-Json -Depth 12 | Set-Content -Encoding utf8 (Join-Path $destination 'report.json')
    foreach ($key in $variables) { Set-WindowsEnvironment $key $saved[$key] }
    Pop-Location
    Write-Host "Windows evidence: $destination"
}
