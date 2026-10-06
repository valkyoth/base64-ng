#requires -Version 7.0
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/windows_gate.ps1"
function Assert([bool] $Condition, [string] $Message) { if (-not $Condition) { throw $Message } }
$env:BASE64_WINDOWS_GATE_ENV_TEST = 'inherited'
Set-WindowsEnvironment BASE64_WINDOWS_GATE_ENV_TEST $null
Assert (-not (Test-Path Env:BASE64_WINDOWS_GATE_ENV_TEST)) 'Unset produced an empty environment variable'
Set-WindowsEnvironment BASE64_WINDOWS_GATE_ENV_TEST 'restored'
Assert ($env:BASE64_WINDOWS_GATE_ENV_TEST -eq 'restored') 'Environment restore failed'
Remove-Item Env:BASE64_WINDOWS_GATE_ENV_TEST
foreach ($file in @('check_windows.ps1', 'windows_gate.ps1')) {
    $tokens = $null; $errors = $null
    $null = [Management.Automation.Language.Parser]::ParseFile("$PSScriptRoot/$file", [ref]$tokens, [ref]$errors)
    Assert ($errors.Count -eq 0) "PowerShell parse failure: $file"
}
$quick = @(Get-WindowsCommands '1.99.0')
$full = @(Get-WindowsCommands '1.90.0' -Full)
Assert ($quick.Count -eq 3) 'Quick gate unexpectedly expanded'
Assert ($full.Count -eq 10) 'Full feature/docs/package matrix incomplete'
foreach ($commands in @($quick, $full)) {
    foreach ($command in $commands) {
        Assert ($command -contains '--locked') 'Unlocked invocation'
        Assert ($command -contains 'x86_64-pc-windows-msvc') 'Non-native target'
    }
}
Assert ((($quick | ForEach-Object { $_ -join ' ' }) -join "`n") -notmatch 'package| doc |benchmark') 'Quick gate contains release work'
Assert ($full[1] -contains '--no-default-features') 'Core-only mode missing'
Assert ($full[2] -contains 'alloc') 'Alloc mode missing'
Assert ($full[3] -contains 'std') 'Std mode missing'
Assert ($quick[2] -contains 'base64-ng-tokio') 'Companion coverage missing'
$directory = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $directory | Out-Null
try {
    $records = [Collections.Generic.List[object]]::new()
    $log = Join-Path $directory 'command.log'
    $pwsh = (Get-Process -Id $PID).Path
    $PSNativeCommandUseErrorActionPreference = $true
    $null = Invoke-WindowsCommand $pwsh @('-NoProfile', '-Command', 'Write-Output "C:\fixture\hello"; exit 0') $log 'C:\fixture' $records
    Assert ((Get-Content $log -Raw).Contains('<REPOSITORY>')) 'Path redaction failed'
    Assert ($records[0].sha256 -eq (Get-FileHash $log -Algorithm SHA256).Hash.ToLowerInvariant()) 'Log digest mismatch'
    $failed = $false
    try { $null = Invoke-WindowsCommand $pwsh @('-NoProfile', '-Command', 'Write-Output failed; exit 23') $log $directory $records }
    catch { $failed = $true }
    Assert $failed 'Native failure was ignored'
    Assert ($records.Count -eq 2 -and $records[1].exit_code -eq 23) 'Failed command was not recorded'
} finally { Remove-Item -Recurse -Force $directory }
Write-Host 'Windows gate: command matrix, parser, native exit propagation and log integrity passed'
