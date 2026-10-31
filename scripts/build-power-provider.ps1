param([string]$OutputDir)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$project=Join-Path $root 'source/power-provider'
$vendor=Join-Path $root 'vendor/power'
$output=if($OutputDir){[IO.Path]::GetFullPath($OutputDir)}else{Join-Path $root 'dist/power-provider'}
$dist=[IO.Path]::GetFullPath((Join-Path $root 'dist'))+[IO.Path]::DirectorySeparatorChar
if(-not $output.StartsWith($dist,[StringComparison]::OrdinalIgnoreCase)){throw 'Provider output must remain inside this repository dist directory'}
foreach($entry in (Get-Content (Join-Path $vendor 'source-manifest.json') -Raw|ConvertFrom-Json)){
 $file=Join-Path $vendor $entry.file
 if(-not(Test-Path -LiteralPath $file)){throw "Missing pinned provider asset: $file"}
 if((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $entry.sha256){throw "Provider asset checksum mismatch: $file"}
}
$setup=Join-Path $vendor 'PawnIO_setup.exe'
$signature=Get-AuthenticodeSignature -LiteralPath $setup
if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=namazso'){throw 'Bundled driver installer signature verification failed'}
& dotnet restore (Join-Path $project 'PowerProvider.csproj') -r win-x64 --locked-mode
if($LASTEXITCODE -ne 0){throw 'Locked provider restore failed'}
& dotnet publish (Join-Path $project 'PowerProvider.csproj') -c Release -r win-x64 --self-contained true --no-restore -o $output
if($LASTEXITCODE -ne 0){throw 'Self-contained provider publish failed'}
foreach($dir in @('driver','notices','sources','source')){New-Item -ItemType Directory -Path (Join-Path $output $dir) -Force|Out-Null}
Copy-Item -LiteralPath $setup -Destination (Join-Path $output 'driver/PawnIO_setup.exe') -Force
Copy-Item -Path (Join-Path $vendor '*-*.txt') -Destination (Join-Path $output 'notices') -Force
Copy-Item -Path (Join-Path $vendor 'sources/*.zip') -Destination (Join-Path $output 'sources') -Force
Copy-Item -LiteralPath (Join-Path $vendor 'source-manifest.json') -Destination $output -Force
foreach($name in @('Program.cs','PowerProvider.csproj','packages.lock.json','NuGet.Config','README.md')){
 Copy-Item -LiteralPath (Join-Path $project $name) -Destination (Join-Path $output 'source') -Force
}
# Include package-provided notices as well as exact MPL/LGPL/GPL source archives above.
$lock=Get-Content (Join-Path $project 'packages.lock.json') -Raw|ConvertFrom-Json
foreach($frameworkProp in $lock.dependencies.psobject.Properties){foreach($idProp in $frameworkProp.Value.psobject.Properties){
 $id=$idProp.Name
 $version=$idProp.Value.resolved
 $cache=Join-Path $env:USERPROFILE ".nuget/packages/$($id.ToLowerInvariant())/$version"
 if(Test-Path -LiteralPath $cache){
  $noticeDir=Join-Path $output "notices/$id-$version"
  New-Item -ItemType Directory -Path $noticeDir -Force|Out-Null
  Get-ChildItem -LiteralPath $cache -File|Where-Object {$_.Name -match 'LICENSE|NOTICE|COPYING|\.nuspec$'}|Copy-Item -Destination $noticeDir -Force
 }
}}
Copy-Item -LiteralPath (Join-Path $project 'README.md') -Destination (Join-Path $output 'README.md') -Force
& (Join-Path $output 'mynotch-power.exe') --self-test
if($LASTEXITCODE -ne 0){throw 'Provider self-test failed'}
$files=Get-ChildItem -LiteralPath $output -Recurse -File|Where-Object {$_.Name -ne 'bundle-files.json'}|ForEach-Object {
 [pscustomobject]@{path=$_.FullName.Substring($output.Length).TrimStart('\','/').Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
}
$files|ConvertTo-Json -Depth 3|Set-Content (Join-Path $output 'bundle-files.json') -Encoding utf8
Write-Output "Self-contained sensor provider ready: $output"
