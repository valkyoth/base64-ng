# Shared command planning/execution for the native Windows gate and its tests.
Set-StrictMode -Version Latest

function Set-WindowsEnvironment([string] $Name, [AllowNull()][object] $Value) {
    if ($null -eq $Value) { Remove-Item "Env:$Name" -ErrorAction SilentlyContinue }
    else { [Environment]::SetEnvironmentVariable($Name, [string]$Value) }
}

function Get-WindowsCommands([string] $Compiler, [switch] $Full) {
    $prefix = @("+$Compiler")
    $native = @('--locked', '--release', '--target', 'x86_64-pc-windows-msvc')
    $features = @('default', '', 'alloc', 'std', 'std,simd', 'std,checked-backend')
    if (-not $Full) { $features = @('std,simd', 'std,checked-backend') }
    foreach ($feature in $features) {
        $args = $prefix + @('test') + $native
        if ($feature -ne 'default') {
            $args += '--no-default-features'
            if ($feature) { $args += @('--features', $feature) }
        }
        if ($Full) { $args += '--all-targets' }
        else {
            foreach ($test in @('decode_validation', 'decode_ref', 'in_place_bulk', 'incremental_bulk')) {
                $args += @('--test', $test)
            }
        }
        ,$args
    }
    if ($Full) {
        ,($prefix + @('test') + $native + @('--workspace', '--all-features'))
        ,($prefix + @('clippy') + $native + @('--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings'))
        ,($prefix + @('doc') + $native + @('--workspace', '--all-features', '--no-deps'))
        ,($prefix + @('package', '--locked', '--target', 'x86_64-pc-windows-msvc', '-p', 'base64-ng'))
    } else {
        $args = $prefix + @('test') + $native + @('--all-features', '--all-targets')
        foreach ($package in @('bytes', 'tokio', 'serde', 'multibase', 'pem', 'openpgp')) {
            $args += @('-p', "base64-ng-$package")
        }
        ,$args
    }
}

function Invoke-WindowsCommand {
    param([string] $Tool, [string[]] $Arguments, [string] $Log,
          [string] $Root, [System.Collections.Generic.List[object]] $Records)
    $start = [DateTime]::UtcNow
    $PSNativeCommandUseErrorActionPreference = $false
    $output = @(& $Tool @Arguments 2>&1 | ForEach-Object { $_.ToString() })
    $code = $LASTEXITCODE
    $redacted = ($output -join "`n").Replace($Root, '<REPOSITORY>').Replace($Root.Replace('\', '/'), '<REPOSITORY>')
    if ($HOME) { $redacted = $redacted.Replace($HOME, '<HOME>').Replace($HOME.Replace('\', '/'), '<HOME>') }
    [IO.File]::WriteAllText($Log, $redacted)
    $Records.Add([ordered]@{ tool = $Tool; arguments = $Arguments; exit_code = $code
        rustflags = [Environment]::GetEnvironmentVariable('RUSTFLAGS')
        seconds = ([DateTime]::UtcNow - $start).TotalSeconds
        log = [IO.Path]::GetFileName($Log); sha256 = (Get-FileHash -Algorithm SHA256 $Log).Hash.ToLowerInvariant() })
    Write-Host $redacted
    if ($code -ne 0) { throw "$Tool failed with exit code $code; see $Log" }
    return $output
}
