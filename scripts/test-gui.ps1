param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$outputRoot = Join-Path ([IO.Path]::GetTempPath()) ('DynCursor-smoke-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $outputRoot | Out-Null
try {
    # This mode uses private instance names, remains paused and never writes startup settings.
    $process = Start-Process -FilePath $executablePath -ArgumentList '--smoke-test' `
        -WorkingDirectory ([IO.Path]::GetTempPath()) -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $outputRoot 'stdout.txt') `
        -RedirectStandardError (Join-Path $outputRoot 'stderr.txt')
    if (-not $process.WaitForExit(40000)) {
        Stop-Process -Id $process.Id -Force
        throw 'GUI smoke test timed out before the frontend completed its IPC handshake'
    }
    $process.Refresh()
    $stdout = [string](Get-Content -LiteralPath (Join-Path $outputRoot 'stdout.txt') -Raw)
    $stderr = [string](Get-Content -LiteralPath (Join-Path $outputRoot 'stderr.txt') -Raw)
    if ($process.ExitCode -ne 0 -or $stdout -notlike '*DynCursor GUI ready:*') {
        throw "GUI smoke test failed ($($process.ExitCode)): $stderr $stdout"
    }
    Write-Output $stdout.Trim()
} finally {
    $resolved = [IO.Path]::GetFullPath($outputRoot)
    $expectedParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    if ([IO.Path]::GetDirectoryName($resolved).TrimEnd('\') -ne $expectedParent -or
        -not [IO.Path]::GetFileName($resolved).StartsWith('DynCursor-smoke-')) {
        throw 'Refusing cleanup outside the smoke-test temporary directory'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
