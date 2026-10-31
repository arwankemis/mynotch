param(
  [ValidateSet('Public')]
  [string]$Profile = 'Public',
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
$profileLower = $Profile.ToLowerInvariant()

$profileDistDir = Join-Path $packageRoot "dist\$profileLower"
$exePath = Join-Path $profileDistDir 'mynotch.exe'
if (-not (Test-Path -LiteralPath $exePath)) {
  $flag = ""
  throw "Executable mynotch.exe untuk profil $Profile belum dibangun di $profileDistDir. Jalankan .\scripts\build.ps1 $flag terlebih dahulu."
}

$destDir = if ($OutputDir) { $OutputDir } else { $packageRoot }
if (-not (Test-Path -LiteralPath $destDir)) {
  New-Item -ItemType Directory -Path $destDir -Force | Out-Null
}

$archiveName = "mynotch-update-$profileLower-windows-x64-$displayVersion.zip"
$archive = Join-Path $destDir $archiveName

$stage = Join-Path ([IO.Path]::GetTempPath()) ('mynotch-package-' + [Guid]::NewGuid().ToString('N'))
$payload = Join-Path $stage 'mynotch'
New-Item -ItemType Directory -Path $payload -Force | Out-Null

try {
  Copy-Item -LiteralPath $exePath -Destination (Join-Path $payload 'mynotch.exe') -Force

  # Copy root documentation and dependencies
  foreach ($name in @('libunwind.dll', 'README.md', 'THIRD_PARTY_NOTICES.md', 'CHANGELOG.md', 'LICENSE', 'licenses', 'examples')) {
    $src = Join-Path $packageRoot $name
    if (Test-Path -LiteralPath $src) {
      Copy-Item -LiteralPath $src -Destination $payload -Recurse -Force
    }
  }

  # Copy user documentation only (omit internal agent handoffs, audit notes, and mockups)
  $docsDest = Join-Path $payload 'docs'
  New-Item -ItemType Directory -Path $docsDest -Force | Out-Null
  foreach ($doc in @('README.md', 'PANDUAN.md', 'PANDUAN-MOD.md', 'PORTABILITAS.md')) {
    $docSrc = Join-Path $packageRoot "docs\$doc"
    if (Test-Path -LiteralPath $docSrc) {
      Copy-Item -LiteralPath $docSrc -Destination $docsDest -Force
    }
  }

  $indexPath = Join-Path $docsDest 'README.md'
  $indexText = [IO.File]::ReadAllText($indexPath).Replace('(PENGEMBANGAN.md)', '(https://github.com/arwankemis/mynotch/blob/main/docs/PENGEMBANGAN.md)')
  [IO.File]::WriteAllText($indexPath, $indexText, [Text.UTF8Encoding]::new($false))
  Copy-Item -LiteralPath (Join-Path $packageRoot 'docs/images') -Destination $docsDest -Recurse -Force

  if ($Profile -eq 'Public') {
    & python (Join-Path $PSScriptRoot 'check-publication.py') --directory $payload
    if ($LASTEXITCODE -ne 0) { throw 'Public package rejected by publication check.' }
  }

  if (Test-Path -LiteralPath $archive) {
    Remove-Item -LiteralPath $archive -Force
  }
  Compress-Archive -Path (Join-Path $payload '*') -DestinationPath $archive -Force

  if ($Profile -eq 'Public') {
    & python (Join-Path $PSScriptRoot 'check-publication.py') --archive $archive
    if ($LASTEXITCODE -ne 0) { throw 'Public ZIP rejected by publication check.' }
  }

  $hashFiles = @($exePath, (Join-Path $packageRoot 'libunwind.dll'), $archive)
  $lines = foreach ($file in $hashFiles) {
    $hash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
    $fileName = [IO.Path]::GetFileName($file)
    "$hash  $fileName"
  }
  $sumsFile = Join-Path $destDir "SHA256SUMS-$profileLower.txt"
  [IO.File]::WriteAllLines($sumsFile, $lines, [Text.UTF8Encoding]::new($false))
  if (-not $OutputDir) { [IO.File]::WriteAllLines((Join-Path $packageRoot 'SHA256SUMS.txt'), $lines, [Text.UTF8Encoding]::new($false)) }

  Write-Output $archive
} finally {
  $absoluteStage = [IO.Path]::GetFullPath($stage)
  $absoluteTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if ($absoluteStage.StartsWith($absoluteTemp, [StringComparison]::OrdinalIgnoreCase) -and [IO.Path]::GetFileName($absoluteStage).StartsWith('mynotch-package-')) {
    Remove-Item -LiteralPath $absoluteStage -Recurse -Force -ErrorAction SilentlyContinue
  }
}
