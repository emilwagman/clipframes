# Stress run for Windows. Run it in the desktop session with the test page (demo/northwind or
# demo/stress/big.html) filling the screen: every click lands on that page or is taken by
# Clipframes. Writes its findings to %USERPROFILE%\cf-stress.txt.
param([int]$Picks = 200, [int]$Cycles = 30)

$exe = Join-Path $PSScriptRoot "..\src-tauri\target\release\clipframes.exe"
$out = Join-Path $env:USERPROFILE "cf-stress.txt"
Add-Type @"
using System; using System.Runtime.InteropServices;
public class In {
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr e);
  public static void Move(double x, double y) { mouse_event(0x8001, (uint)(x * 65535), (uint)(y * 65535), 0, UIntPtr.Zero); }
  public static void Click() { mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero); mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero); }
  public static void Esc() { keybd_event(0x1B, 0, 0, UIntPtr.Zero); keybd_event(0x1B, 0, 2, UIntPtr.Zero); }
}
"@
$lines = New-Object System.Collections.Generic.List[string]
function State([string]$label) {
  $p = Get-Process clipframes -ErrorAction SilentlyContinue | Sort-Object StartTime | Select-Object -First 1
  $w = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -match 'com.emil.clipframes' })
  $lines.Add(("{0,-34} core {1,6:N1} MB ({2,5:N1} private), threads {3,3}, handles {4,4}, web views {5} ({6:N0} MB)" -f $label, ($p.WorkingSet64/1MB), ($p.PrivateMemorySize64/1MB), $p.Threads.Count, $p.HandleCount, $w.Count, (($w | Measure-Object WorkingSetSize -Sum).Sum/1MB)))
}

Stop-Process -Name clipframes -Force -ErrorAction SilentlyContinue
Start-Process $exe; Start-Sleep 4
State "idle, just started"

Start-Process $exe -ArgumentList "--open"; Start-Sleep 4
State "round open, nothing picked"
$sw = [Diagnostics.Stopwatch]::StartNew()
for ($i = 0; $i -lt $Picks; $i++) {
  # A 20 x 10 grid over the middle of the screen, away from the bar at the bottom.
  [In]::Move(0.22 + 0.60 * (($i % 20) / 19.0), 0.25 + 0.40 * ([Math]::Floor($i / 20) % 10 / 9.0))
  Start-Sleep -Milliseconds 25
  [In]::Click()
  Start-Sleep -Milliseconds 35
}
$took = $sw.Elapsed.TotalSeconds
Start-Sleep 3
State "after $Picks clicks"
$folder = Get-ChildItem (Join-Path $env:USERPROFILE "Clipframes") | Sort-Object Name | Select-Object -Last 1
$json = Get-Content (Join-Path $folder.FullName "capture.json") -Raw -Encoding UTF8 | ConvertFrom-Json
$clip = Get-Clipboard -Raw
$lines.Add(("{0} clicks in {1:N1} s; picks saved: {2}; notes.md {3:N0} KB; clipboard {4:N0} characters, {5} lines" -f $Picks, $took, $json.picks.Count, ((Get-Item (Join-Path $folder.FullName "notes.md")).Length/1KB), $clip.Length, ($clip -split "`n").Count))
[In]::Esc(); Start-Sleep -Milliseconds 700; [In]::Esc(); Start-Sleep 2
State "round closed, bar warm"

for ($i = 0; $i -lt $Cycles; $i++) {
  Start-Process $exe -ArgumentList "--open"; Start-Sleep -Milliseconds 1500
  [In]::Esc(); Start-Sleep -Milliseconds 400; [In]::Esc(); Start-Sleep -Milliseconds 600
}
State "after $Cycles open/close cycles"
$open = @(Get-Process clipframes -ErrorAction SilentlyContinue).Count
Start-Sleep 100
State "idle again, bar let go"
$p = Get-Process clipframes | Sort-Object StartTime | Select-Object -First 1
$c0 = $p.CPU; Start-Sleep 30; $p.Refresh()
$lines.Add(("idle CPU over 30 s: {0:N3} s; clipframes processes: {1}" -f ($p.CPU - $c0), $open))
[IO.File]::WriteAllLines($out, $lines, [Text.Encoding]::UTF8)
