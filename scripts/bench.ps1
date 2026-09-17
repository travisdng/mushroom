<#
.SYNOPSIS
  Measure time to an interactive window and idle memory (R5.1, R5.5, R5.6).

.DESCRIPTION
  The other three numbers in R5 — search, note open, index build — are measured
  by `cargo test --release --test bench -- --ignored --nocapture`, because they
  are properties of the library. These two are properties of the running
  application and cannot be measured from a test: a window appearing is not
  something the Rust side can time honestly.

  "Interactive" here means the main window is visible and responding to
  messages, which is what `MainWindowHandle` plus `Responding` together mean.
  That is stricter than "the process exists" and looser than "the first paint
  has landed"; the gap is a frame or two.

  Run against the release build, never the debug one, and against a corpus:

    node scripts/make-corpus.mjs C:\Users\you\Mushroom\bench-notes
    # point the app's notes root at it, then:
    .\scripts\bench.ps1 -Exe .\src-tauri\target\release\mushroom.exe

.PARAMETER Exe
  The built executable to measure.

.PARAMETER Runs
  How many launches to average. Each one closes the app again.

.PARAMETER IdleSeconds
  How long to leave the app alone before reading CPU and memory.
#>
param(
  [string] $Exe = ".\src-tauri\target\release\mushroom.exe",
  [int] $Runs = 5,
  [int] $IdleSeconds = 30
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Exe)) {
  throw "No executable at $Exe. Build it with: npm run tauri build"
}
$Exe = (Resolve-Path $Exe).Path

function Stop-Mushroom {
  Get-Process mushroom -ErrorAction SilentlyContinue | ForEach-Object {
    $_.CloseMainWindow() | Out-Null
  }
  Start-Sleep -Milliseconds 500
  Get-Process mushroom -ErrorAction SilentlyContinue |
    Stop-Process -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 500
}

# Wait for a window that is both up and answering, and report how long it took.
function Measure-Launch {
  Stop-Mushroom
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $proc = Start-Process -FilePath $Exe -PassThru

  while ($sw.Elapsed.TotalSeconds -lt 30) {
    $proc.Refresh()
    if ($proc.HasExited) { throw "the app exited during launch" }
    if ($proc.MainWindowHandle -ne 0 -and $proc.Responding) {
      $sw.Stop()
      return [pscustomobject]@{ Ms = $sw.Elapsed.TotalMilliseconds; Proc = $proc }
    }
    Start-Sleep -Milliseconds 20
  }
  throw "no window after 30 s"
}

$times = @()
for ($i = 1; $i -le $Runs; $i++) {
  $r = Measure-Launch
  $times += $r.Ms
  Write-Host ("launch {0}: {1:N0} ms" -f $i, $r.Ms)
  # Leave the last one running, to measure idle against.
  if ($i -lt $Runs) { Stop-Mushroom }
}

$sorted = $times | Sort-Object
Write-Host ""
Write-Host ("time to an interactive window: median {0:N0} ms, worst {1:N0} ms, over {2} runs" -f `
  $sorted[[int][math]::Floor($sorted.Count / 2)], ($sorted | Select-Object -Last 1), $Runs)

# Idle: WebView2 runs the UI in its own processes, so the working set of
# mushroom.exe alone would understate it by most of the total. Everything
# descended from the app counts.
Write-Host ""
Write-Host "leaving the app idle for $IdleSeconds s..."

function Get-AppProcesses {
  $roots = Get-Process mushroom -ErrorAction SilentlyContinue
  if (-not $roots) { return @() }
  $ids = @($roots.Id)
  # WebView2 hosts are children (and grandchildren) of the app process.
  for ($depth = 0; $depth -lt 3; $depth++) {
    $kids = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
      Where-Object { $ids -contains $_.ParentProcessId }
    $new = @($kids.ProcessId) | Where-Object { $ids -notcontains $_ }
    if (-not $new) { break }
    $ids += $new
  }
  Get-Process -Id $ids -ErrorAction SilentlyContinue
}

$before = Get-AppProcesses
$cpuBefore = ($before | Measure-Object TotalProcessorTime -Sum).Sum
Start-Sleep -Seconds $IdleSeconds
$after = Get-AppProcesses
$cpuAfter = ($after | Measure-Object TotalProcessorTime -Sum).Sum

$workingSet = ($after | Measure-Object WorkingSet64 -Sum).Sum / 1MB
$private = ($after | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB

# Across N cores, a fully busy app would burn N seconds of CPU per second.
$cpuSeconds = ($cpuAfter - $cpuBefore)
$cores = [Environment]::ProcessorCount
$cpuPercent = ($cpuSeconds / $IdleSeconds) * 100 / $cores

Write-Host ""
Write-Host ("processes: {0} ({1})" -f $after.Count, (($after | Group-Object ProcessName |
  ForEach-Object { "$($_.Count)x $($_.Name)" }) -join ", "))
Write-Host ("idle working set: {0:N1} MB" -f $workingSet)
Write-Host ("idle private bytes: {0:N1} MB" -f $private)
Write-Host ("idle CPU over {0} s: {1:N2} s of CPU time = {2:N3}% of {3} cores" -f `
  $IdleSeconds, $cpuSeconds, $cpuPercent, $cores)
Write-Host ""
Write-Host "machine: $((Get-CimInstance Win32_Processor).Name), $cores cores, $([math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)) GB RAM"
Write-Host "os: $((Get-CimInstance Win32_OperatingSystem).Caption) $([Environment]::OSVersion.Version)"
