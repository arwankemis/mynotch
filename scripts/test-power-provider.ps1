param([string]$ProviderDir)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$dir=if($ProviderDir){[IO.Path]::GetFullPath($ProviderDir)}else{Join-Path $root 'dist/power-provider'}
$exe=Join-Path $dir 'mynotch-power.exe'
foreach($entry in (Get-Content (Join-Path $dir 'bundle-files.json') -Raw|ConvertFrom-Json)){
 $file=Join-Path $dir $entry.path
 if(-not(Test-Path -LiteralPath $file) -or (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $entry.sha256){throw "Provider bundle integrity failure: $($entry.path)"}
}
& $exe --self-test
if($LASTEXITCODE -ne 0){throw 'Provider contract tests failed'}
$info=[Diagnostics.ProcessStartInfo]::new($exe,'--stream')
$info.UseShellExecute=$false;$info.CreateNoWindow=$true;$info.RedirectStandardInput=$true;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true;$info.WorkingDirectory=$dir
$process=[Diagnostics.Process]::new();$process.StartInfo=$info
try{
 if(-not $process.Start()){throw 'Provider did not start'}
 $read=$process.StandardOutput.ReadLineAsync()
 if(-not $read.Wait(15000)){throw 'Provider sample timeout'}
 $frame=$read.Result|ConvertFrom-Json
 if($frame.schema -ne 1 -or $frame.backend -ne 'librehardwaremonitor-0.9.6'){throw 'Invalid provider wire contract'}
 $process.Refresh();$workingSetMB=[Math]::Round($process.WorkingSet64/1MB,1)
 $process.StandardInput.Close()
 if(-not $process.WaitForExit(5000)){throw 'Provider remained alive after parent stdin EOF'}
 if($process.ExitCode -ne 0){throw 'Provider exited unsuccessfully'}
 [pscustomobject]@{schema=$frame.schema;cpu_watts=$frame.cpu_watts;gpu_watts=$frame.gpu_watts;status=$frame.status;working_set_mb=$workingSetMB;eof_shutdown='passed'}|ConvertTo-Json
}finally{if(-not $process.HasExited){$process.Kill();$process.WaitForExit()};$process.Dispose()}
