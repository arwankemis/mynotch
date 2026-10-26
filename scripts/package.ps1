param([string]$Version='0.0.1-alpha')
$ErrorActionPreference='Stop'
$packageRoot=Split-Path -Parent $PSScriptRoot
foreach($required in @('mynotch.exe','libunwind.dll','README.md','THIRD_PARTY_NOTICES.md','CHANGELOG.md','docs','licenses')){if(-not(Test-Path -LiteralPath (Join-Path $packageRoot $required))){throw "Berkas paket belum lengkap: $required"}}
$archive=Join-Path $packageRoot "mynotch-$Version-windows-x64.zip"
$stage=Join-Path ([IO.Path]::GetTempPath()) ('mynotch-package-'+[Guid]::NewGuid().ToString('N'))
$payload=Join-Path $stage 'mynotch'
New-Item -ItemType Directory -Path $payload | Out-Null
foreach($name in @('mynotch.exe','libunwind.dll','README.md','THIRD_PARTY_NOTICES.md','CHANGELOG.md','docs','licenses')){Copy-Item -LiteralPath (Join-Path $packageRoot $name) -Destination $payload -Recurse}
Compress-Archive -LiteralPath $payload -DestinationPath $archive -Force
$lines=foreach($file in @('mynotch.exe','libunwind.dll',[IO.Path]::GetFileName($archive))){$hash=(Get-FileHash -LiteralPath (Join-Path $packageRoot $file) -Algorithm SHA256).Hash.ToLowerInvariant();"$hash  $file"}
[IO.File]::WriteAllLines((Join-Path $packageRoot 'SHA256SUMS.txt'),$lines,[Text.UTF8Encoding]::new($false))
# stage dibuat khusus oleh pemanggilan ini; pastikan tetap di direktori temp sebelum membersihkannya.
$absoluteStage=[IO.Path]::GetFullPath($stage)
$absoluteTemp=[IO.Path]::GetFullPath([IO.Path]::GetTempPath())
if($absoluteStage.StartsWith($absoluteTemp,[StringComparison]::OrdinalIgnoreCase) -and [IO.Path]::GetFileName($absoluteStage).StartsWith('mynotch-package-')){Remove-Item -LiteralPath $absoluteStage -Recurse -Force}
Write-Output $archive
