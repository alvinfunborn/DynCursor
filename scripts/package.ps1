param(
    [string]$Destination = 'target/dist/release',
    [switch]$SmokeTest
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $metadata = cargo metadata --no-deps --format-version 1 --locked | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $version = ($metadata.packages | Where-Object name -eq 'DynCursor').version
    $executable = Join-Path $projectRoot 'target/release/DynCursor.exe'
    if ((Get-Item -LiteralPath $executable).VersionInfo.FileVersion -ne $version) {
        throw 'Executable version does not match Cargo.toml. Build release first.'
    }
    $output = [IO.Path]::GetFullPath((Join-Path $projectRoot $Destination))
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    $temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $staging = Join-Path $temporaryRoot ('DynCursor-package-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $staging | Out-Null
    try {
        Copy-Item -LiteralPath $executable -Destination $staging
        Copy-Item -LiteralPath 'config' -Destination $staging -Recurse
        $resourceOutput = Join-Path $staging 'resources'
        New-Item -ItemType Directory -Path (Join-Path $resourceOutput 'cur') -Force | Out-Null
        Copy-Item -LiteralPath 'resources/logo.ico','resources/logo.png','resources/logo.svg' -Destination $resourceOutput
        Copy-Item -LiteralPath 'resources/cursor-source' -Destination $resourceOutput -Recurse
        Copy-Item -LiteralPath 'resources/cur/custom-arrow.cur','resources/cur/custom-beam.cur','resources/cur/custom-link.cur' -Destination (Join-Path $resourceOutput 'cur')
        Copy-Item -LiteralPath 'README.md','CHANGELOG.md','LICENSE','THIRD_PARTY_NOTICES.md','THIRD_PARTY_LICENSES.txt' -Destination $staging
        $check = Start-Process -FilePath (Join-Path $staging 'DynCursor.exe') `
            -ArgumentList '--check-config' -WorkingDirectory $temporaryRoot `
            -WindowStyle Hidden -Wait -PassThru `
            -RedirectStandardOutput (Join-Path $output 'check-config.stdout.txt') `
            -RedirectStandardError (Join-Path $output 'check-config.stderr.txt')
        if ($check.ExitCode -ne 0) { throw 'Packaged configuration validation failed' }
        if ($SmokeTest) { & "$PSScriptRoot/test-gui.ps1" -Executable (Join-Path $staging 'DynCursor.exe') }
        $zipPath = Join-Path $output 'DynCursor.zip'
        Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zipPath -Force
        $checksum = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
        [IO.File]::WriteAllText((Join-Path $output 'SHA256SUMS.txt'), "$checksum  DynCursor.zip`n", [Text.UTF8Encoding]::new($false))
        Write-Output "DynCursor $version packaged: $zipPath"
    } finally {
        $resolvedStaging = [IO.Path]::GetFullPath($staging)
        if ([IO.Path]::GetDirectoryName($resolvedStaging).TrimEnd('\') -ne $temporaryRoot.TrimEnd('\') -or
            -not [IO.Path]::GetFileName($resolvedStaging).StartsWith('DynCursor-package-')) {
            throw 'Refusing cleanup outside the temporary package directory'
        }
        Remove-Item -LiteralPath $resolvedStaging -Recurse -Force
    }
} finally {
    Pop-Location
}
