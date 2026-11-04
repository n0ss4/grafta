# Install grafta from a GitHub release, on Windows.
#
#   irm https://raw.githubusercontent.com/n0ss4/grafta/main/install.ps1 | iex
#
#   $env:GRAFTA_VERSION      a tag such as v0.1.0 (default: the latest release)
#   $env:GRAFTA_INSTALL_DIR  where grafta.exe goes (default: ~\.grafta\bin)
#
# The archive's checksum is checked against the release's SHA256SUMS before
# anything is installed. Run it again to replace the binary.
$ErrorActionPreference = 'Stop'

$repo = 'n0ss4/grafta'
$dir = if ($env:GRAFTA_INSTALL_DIR) { $env:GRAFTA_INSTALL_DIR } else { Join-Path $HOME '.grafta\bin' }

if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
    throw "grafta: no prebuilt binary for this Windows; install with: cargo install grafta"
}
$target = 'x86_64-pc-windows-msvc'
$base = if ($env:GRAFTA_VERSION) {
    "https://github.com/$repo/releases/download/$($env:GRAFTA_VERSION)"
} else {
    "https://github.com/$repo/releases/latest/download"
}

$archive = "grafta-$target.zip"
$work = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
    Invoke-WebRequest -UseBasicParsing "$base/$archive" -OutFile (Join-Path $work $archive)
    Invoke-WebRequest -UseBasicParsing "$base/SHA256SUMS" -OutFile (Join-Path $work 'SHA256SUMS')

    $line = Get-Content (Join-Path $work 'SHA256SUMS') | Where-Object { $_ -match " $([regex]::Escape($archive))$" }
    if (-not $line) { throw "grafta: $archive is not listed in SHA256SUMS" }
    $expected = ($line -split ' ')[0]
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $work $archive)).Hash.ToLower()
    if ($actual -ne $expected) { throw "grafta: checksum mismatch for $archive; nothing installed" }

    Expand-Archive -Force (Join-Path $work $archive) -DestinationPath $work
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    Move-Item -Force (Join-Path $work 'grafta.exe') (Join-Path $dir 'grafta.exe')
} finally {
    Remove-Item -Recurse -Force $work
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not ($userPath -split ';' -contains $dir)) {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$dir", 'User')
    Write-Host "added $dir to your PATH (open a new terminal to use it)"
}
$version = & (Join-Path $dir 'grafta.exe') --version
Write-Host "installed $version to $dir"
Write-Host "next: claude mcp add grafta -- $(Join-Path $dir 'grafta.exe') mcp"
