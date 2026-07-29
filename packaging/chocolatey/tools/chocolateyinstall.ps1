$ErrorActionPreference = 'Stop'

# __URL64__ and __CHECKSUM64__ are substituted with the release's real values by
# the packaging workflow before `choco pack`, so the published package is fully
# self-contained (the end user's `choco install` needs no env vars).
$packageName = 'moonlit'
$url64       = '__URL64__'
$checksum64  = '__CHECKSUM64__'
$toolsDir    = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

Install-ChocolateyZipPackage `
  -PackageName    $packageName `
  -Url64bit       $url64 `
  -Checksum64     $checksum64 `
  -ChecksumType64 'sha256' `
  -UnzipLocation  $toolsDir

# Install-ChocolateyZipPackage extracts moonlit.exe into $toolsDir; Chocolatey
# auto-shims any .exe in the package, so `moonlit` lands on PATH.
