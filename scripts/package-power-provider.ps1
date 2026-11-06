param(
  [string]$Version,
  [string]$OutputDir
)
$ErrorActionPreference = 'Stop'
$packageRoot = Split-Path -Parent $PSScriptRoot

$versionManifestPath = Join-Path $packageRoot 'release\version.json'
if (-not (Test-Path -LiteralPath $versionManifestPath)) {
  throw "Manifest versi belum ditemukan: $versionManifestPath"
}
$versionManifest = Get-Content -LiteralPath $versionManifestPath -Raw | ConvertFrom-Json
$displayVersion = if ($Version) { $Version } else { $versionManifest.display_version }

$providerDistDir = Join-Path $packageRoot 'dist\power-provider'
if (-not (Test-Path -LiteralPath (Join-Path $providerDistDir 'mynotch-power.exe'))) {
  Write-Output "Membangun sensor provider terlebih dahulu..."
  & (Join-Path $PSScriptRoot 'build-power-provider.ps1') -OutputDir $providerDistDir
  if ($LASTEXITCODE -ne 0) { throw 'Pembangunan sensor provider gagal.' }
}

$destDir = if ($OutputDir) { $OutputDir } else { Join-Path $packageRoot 'releases' }
if (-not (Test-Path -LiteralPath $destDir)) {
  New-Item -ItemType Directory -Path $destDir -Force | Out-Null
}

$archiveName = "mynotch-power-provider-windows-x64-$displayVersion.zip"
$archive = Join-Path $destDir $archiveName

$stage = Join-Path ([IO.Path]::GetTempPath()) ('mynotch-provider-pkg-' + [Guid]::NewGuid().ToString('N'))
$payload = Join-Path $stage 'power-provider'
New-Item -ItemType Directory -Path $payload -Force | Out-Null

try {
  Copy-Item -Path (Join-Path $providerDistDir '*') -Destination $payload -Recurse -Force

  if (Test-Path -LiteralPath $archive) {
    Remove-Item -LiteralPath $archive -Force
  }
  Compress-Archive -Path $payload -DestinationPath $archive -Force

  $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
  Write-Output "Paket companion power-provider berhasil dibuat:"
  Write-Output "  Aset  : $archive"
  Write-Output "  Versi : $displayVersion"
  Write-Output "  SHA256: $hash"
}
finally {
  if (Test-Path -LiteralPath $stage) {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
  }
}
