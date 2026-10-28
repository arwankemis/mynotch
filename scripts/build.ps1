param([switch]$Test,[string]$Target='x86_64-pc-windows-gnullvm',[string]$RuntimeDll)
$ErrorActionPreference='Stop'
$packageRoot=Split-Path -Parent $PSScriptRoot
$sourceRoot=Join-Path $packageRoot 'source'
$defaultCargoBin=Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $defaultCargoBin) { $env:PATH = "$defaultCargoBin;$env:PATH" }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust/cargo belum tersedia pada PATH lingkungan pengembangan.' }
$sysroot=(& rustc --print sysroot).Trim()
if ($Target -eq 'x86_64-pc-windows-gnullvm') {
  $hostLine=(& rustc -vV | Select-String '^host:').ToString()
  $hostTarget=$hostLine.Substring(5).Trim()
  $linkerCandidates=@((Join-Path $sysroot "lib/rustlib/$hostTarget/bin/rust-lld.exe"),(Join-Path $sysroot "lib/rustlib/$Target/bin/rust-lld.exe"))
  foreach($candidate in $linkerCandidates){if(Test-Path -LiteralPath $candidate){$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=$candidate;break}}
}
$cargoHome=if($env:CARGO_HOME){$env:CARGO_HOME}else{Join-Path $env:USERPROFILE '.cargo'}
$remap=@("--remap-path-prefix=$packageRoot=.","--remap-path-prefix=$cargoHome=dependencies","--remap-path-prefix=$sysroot=toolchain")
$crt2 = Join-Path $sysroot 'lib/rustlib/x86_64-pc-windows-gnu/lib/self-contained/crt2.o'
if (Test-Path -LiteralPath $crt2) { $remap += "-Clink-arg=$crt2" }
$env:CARGO_ENCODED_RUSTFLAGS=$remap -join [char]31

$env:PATH = "$packageRoot;$(Join-Path $sysroot 'bin');$env:PATH"

Push-Location $sourceRoot
try {
  if($Test){& cargo test --locked --target $Target}else{& cargo build --locked --release --target $Target}
  if($LASTEXITCODE -ne 0){throw 'Build/test mynotch gagal.'}
  if(-not $Test){
    $targetRoot=if($env:CARGO_TARGET_DIR){$env:CARGO_TARGET_DIR}else{Join-Path $sourceRoot 'target'}
    $executable=Join-Path $targetRoot "$Target/release/mynotch.exe"
    Copy-Item -LiteralPath $executable -Destination (Join-Path $packageRoot 'mynotch.exe') -Force
    if($RuntimeDll){Copy-Item -LiteralPath $RuntimeDll -Destination (Join-Path $packageRoot 'libunwind.dll') -Force}
    elseif(Test-Path -LiteralPath (Join-Path $sysroot 'bin/libunwind.dll')){Copy-Item -LiteralPath (Join-Path $sysroot 'bin/libunwind.dll') -Destination (Join-Path $packageRoot 'libunwind.dll') -Force}
    if($Target -eq 'x86_64-pc-windows-gnullvm' -and -not(Test-Path -LiteralPath (Join-Path $packageRoot 'libunwind.dll'))){throw 'Sertakan libunwind.dll yang cocok dengan toolchain melalui parameter RuntimeDll.'}
    Write-Output "Build selesai di $packageRoot"
  }
} finally {Pop-Location}
