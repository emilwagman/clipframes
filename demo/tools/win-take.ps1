# One take of the Clipframes walkthrough, driven with real input events at a human pace,
# captured with ffmpeg. Coordinates are physical pixels on the 3840 x 2160 screen.
param([string]$Name = "take1", [int]$Seconds = 42, [string]$Scene = "pick")
$demo = "C:\Users\User\cf-demo"
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Threading;
public class H {
  [DllImport("user32.dll")] static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
  [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr e);
  [DllImport("user32.dll")] static extern bool GetCursorPos(out P p);
  [StructLayout(LayoutKind.Sequential)] struct P { public int x, y; }
  [StructLayout(LayoutKind.Explicit, Size = 40)] struct KI { [FieldOffset(0)] public uint type; [FieldOffset(8)] public ushort vk; [FieldOffset(10)] public ushort scan; [FieldOffset(12)] public uint flags; [FieldOffset(16)] public uint time; [FieldOffset(24)] public IntPtr extra; }
  [DllImport("user32.dll")] static extern uint SendInput(uint n, KI[] inputs, int size);
  static Random r = new Random(7);
  static void To(double x, double y) { mouse_event(0x8001, (uint)(x * 65535 / 3839), (uint)(y * 65535 / 2159), 0, UIntPtr.Zero); }
  // An eased, slightly curved path, the way a hand moves a mouse.
  public static void Glide(double x, double y, int ms) {
    P p; GetCursorPos(out p); double x0 = p.x, y0 = p.y, dx = x - x0, dy = y - y0;
    double len = Math.Sqrt(dx * dx + dy * dy), bend = len * 0.06 * (r.Next(2) == 0 ? 1 : -1);
    int steps = Math.Max(1, ms / 8);
    for (int i = 1; i <= steps; i++) {
      double t = (double)i / steps, e = t < 0.5 ? 4 * t * t * t : 1 - Math.Pow(-2 * t + 2, 3) / 2, arc = Math.Sin(Math.PI * e) * bend;
      To(x0 + dx * e - (len > 0 ? dy / len : 0) * arc, y0 + dy * e + (len > 0 ? dx / len : 0) * arc);
      Thread.Sleep(8);
    }
    To(x, y);
  }
  public static void Click() { mouse_event(2, 0, 0, 0, UIntPtr.Zero); Thread.Sleep(70); mouse_event(4, 0, 0, 0, UIntPtr.Zero); }
  public static void Down() { mouse_event(2, 0, 0, 0, UIntPtr.Zero); }
  public static void Up() { mouse_event(4, 0, 0, 0, UIntPtr.Zero); }
  public static void Key(params byte[] vks) { foreach (byte v in vks) keybd_event(v, 0, 0, UIntPtr.Zero); Thread.Sleep(60); for (int i = vks.Length - 1; i >= 0; i--) keybd_event(vks[i], 0, 2, UIntPtr.Zero); }
  public static void Type(string text) {
    foreach (char c in text) {
      KI[] k = new KI[2];
      k[0].type = 1; k[0].scan = c; k[0].flags = 4; k[1].type = 1; k[1].scan = c; k[1].flags = 6;
      SendInput(2, k, Marshal.SizeOf(typeof(KI)));
      Thread.Sleep(c == ' ' ? 95 + r.Next(60) : 55 + r.Next(75));
    }
  }
}
"@
function Wait([int]$ms) { Start-Sleep -Milliseconds $ms }

$exe = "C:\Users\User\Development\clipframes\desktop\src-tauri\target\release\clipframes.exe"
if (-not (Get-Process clipframes -ErrorAction SilentlyContinue)) { Start-Process $exe -ArgumentList "--hidden"; Wait 2500 }
[H]::Glide(1500, 1250, 300)
$ff = Start-Process ffmpeg -ArgumentList "-y","-filter_complex","ddagrab=output_idx=0:framerate=30:video_size=3840x2088,hwdownload,format=bgra","-c:v","libx264","-preset","ultrafast","-crf","12","-pix_fmt","yuv420p","-t","$Seconds","$demo\$Name.mkv" -WindowStyle Hidden -PassThru -RedirectStandardError "$demo\$Name.log"
Wait 2500

# Places on the 3840 x 2160 screen: the bar's controls, and the terminal's prompt.
$area = @(1608, 1956); $clip = @(1699, 1956); $history = @(2220, 1956); $done = @(2316, 1956); $prompt = @(3000, 1963)
function Go($p, [int]$ms) { [H]::Glide($p[0], $p[1], $ms) }

if ($Scene -eq "pick") {
  [H]::Key(0x11, 0x12, 0x20)            # Ctrl+Alt+Space: open Clipframes
  Wait 1500
  [H]::Glide(468, 206, 750);  Wait 550   # the page title
  [H]::Glide(494, 386, 650);  Wait 450   # a figure
  [H]::Glide(2040, 230, 850); Wait 400   # Export
  [H]::Glide(2189, 230, 380); Wait 600   # New invoice
  [H]::Click();               Wait 1100
  [H]::Type("make this green"); Wait 650
  [H]::Glide(1745, 386, 850); Wait 500   # the overdue figure
  [H]::Click();               Wait 1100
  [H]::Type("too alarming, use the normal text colour"); Wait 500
  [H]::Key(0x0D);             Wait 900
  Go $done 800; Wait 350
  [H]::Click();               Wait 1200
  Go $prompt 950; Wait 300
  [H]::Click();               Wait 700
  [H]::Key(0x11, 0x56);       Wait 1500  # paste
  [H]::Glide(3300, 1500, 700)
}

if ($Scene -eq "show") {
  [H]::Key(0x11, 0x12, 0x20)
  Wait 1500
  Go $area 800; Wait 300; [H]::Click(); Wait 700           # the Area tool
  [H]::Glide(400, 455, 800); Wait 300
  [H]::Down(); Wait 120; [H]::Glide(2290, 1010, 1100); Wait 250; [H]::Up(); Wait 1200   # the table
  [H]::Type("rows are too tall, tighten them"); Wait 500
  [H]::Key(0x0D); Wait 900
  Go $clip 800; Wait 300; [H]::Click(); Wait 700           # the Clip tool
  [H]::Glide(380, 150, 800); Wait 300
  [H]::Down(); Wait 120; [H]::Glide(2290, 450, 1000); Wait 250; [H]::Up(); Wait 1300    # the header, recording
  [H]::Glide(1790, 230, 700); Wait 300; [H]::Click(); Wait 500                          # the search field
  [H]::Type("maersk"); Wait 1300
  Go $done 800; Wait 300; [H]::Click(); Wait 1400          # Stop
  [H]::Type("typing in search filters nothing"); Wait 500
  [H]::Key(0x0D); Wait 900
  Go $done 600; Wait 300; [H]::Click(); Wait 1200          # Done
  Go $prompt 950; Wait 300; [H]::Click(); Wait 700
  [H]::Key(0x11, 0x56); Wait 1500
  [H]::Glide(3300, 1500, 700)
}

if ($Scene -eq "return") {
  # Clipframes was used on this site before, so its tab is waiting at the bottom.
  Wait 800
  Go $prompt 900; Wait 300; [H]::Click(); Wait 2600        # to the terminal: the tab leaves
  [H]::Glide(1200, 1100, 900); Wait 300; [H]::Click(); Wait 2600   # back to the site: it returns
  [H]::Glide(1920, 1956, 800); Wait 400; [H]::Click(); Wait 1800   # the tab opens the bar
  Go $history 700; Wait 400; [H]::Click(); Wait 2600       # History
  [H]::Glide(2230, 735, 800); Wait 500; [H]::Click(); Wait 1500    # Copy the newest
  [H]::Glide(2300, 900, 600)
}

$ff.WaitForExit()
Get-Clipboard -Raw | Set-Content -Encoding UTF8 "$demo\$Name-clipboard.txt"
"done" | Set-Content "$demo\$Name.done"
