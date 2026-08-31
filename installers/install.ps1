# Thin wrapper redirecting to cargo-dist generated resumake-installer.ps1
# https://github.com/arvinduh/resumake

$ErrorActionPreference = 'Stop'

$version = if ($env:RESUMAKE_VERSION) { $env:RESUMAKE_VERSION } else { "latest" }
if ($version -eq "latest") {
    $url = "https://github.com/arvinduh/resumake/releases/latest/download/resumake-installer.ps1"
} else {
    $url = "https://github.com/arvinduh/resumake/releases/download/v$($version -replace '^v', '')/resumake-installer.ps1"
}

irm $url | iex

