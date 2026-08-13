import { execFileSync } from 'node:child_process'

export interface MonitorRect {
  x: number
  y: number
  w: number
  h: number
}

// Enumerate monitors and return the work area (physical pixels) of the first
// 100% (96 effective DPI) monitor. Physical == logical there, so screenshots
// come out at deterministic pixel size. The process must be per-monitor DPI
// aware before querying, or effective DPI is virtualized to 96 everywhere and
// physical coords are garbage. Null if no such monitor exists.
export function find100DpiMonitor(): MonitorRect | null {
  const ps = `
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class KtermMon {
  public struct RECT { public int Left, Top, Right, Bottom; }
  public delegate bool EnumProc(IntPtr h, IntPtr hdc, ref RECT l, IntPtr d);
  [DllImport("shcore.dll")] public static extern int SetProcessDpiAwareness(int v);
  [DllImport("user32.dll")] public static extern bool EnumDisplayMonitors(IntPtr h, IntPtr c, EnumProc f, IntPtr d);
  [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr h, ref MI lpmi);
  [DllImport("shcore.dll")] public static extern int GetDpiForMonitor(IntPtr h, int t, out uint x, out uint y);
  [StructLayout(LayoutKind.Sequential)] public struct MI { public int cbSize; public RECT rcMon; public RECT rcWork; public int flags; }
  public static string Found = "";
  static bool Cb(IntPtr h, IntPtr hdc, ref RECT l, IntPtr d) {
    var mi = new MI(); mi.cbSize = Marshal.SizeOf(typeof(MI));
    GetMonitorInfo(h, ref mi);
    uint dx = 0, dy = 0;
    GetDpiForMonitor(h, 0, out dx, out dy); // effective DPI, per-monitor aware
    if (dx == 96 && Found == "") {
      Found = mi.rcWork.Left + " " + mi.rcWork.Top + " " + (mi.rcWork.Right - mi.rcWork.Left) + " " + (mi.rcWork.Bottom - mi.rcWork.Top);
    }
    return true;
  }
  public static void Run() { SetProcessDpiAwareness(2); EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, Cb, IntPtr.Zero); }
}
'@
[KtermMon]::Run()
Write-Output ([KtermMon]::Found)
`
  try {
    const out = execFileSync('powershell.exe', ['-NoProfile', '-Command', ps], { encoding: 'utf8', timeout: 15000 })
    const line = out.trim().split(/\r?\n/).filter((l) => l.trim()).pop() ?? ''
    const [x, y, w, h] = line.split(' ').map(Number)
    if (!(w > 0 && h > 0)) return null
    return { x, y, w, h }
  } catch {
    return null
  }
}

// Move the win-1 window to the center of the given monitor, keeping its
// current size (MoveWindow, not SetWindowPos: position-only SetWindowPos
// glitches the window to 0x0). Returns true when a matching window was moved.
export function moveWindowTo(mon: MonitorRect): boolean {
  const cx = Math.max(0, mon.x + Math.floor(mon.w / 2))
  const cy = Math.max(0, mon.y + Math.floor(mon.h / 2))
  const ps = `
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class KtermMove {
  public struct RECT { public int L, T, R, B; }
  [DllImport("shcore.dll")] public static extern int SetProcessDpiAwareness(int v);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hwnd, int x, int y, int w, int hh, bool repaint);
  public static string Result = "";
  public static void Run(long hwnd, int cx, int cy) {
    var h = new IntPtr(hwnd);
    RECT r;
    if (!GetWindowRect(h, out r)) { Result = "norect"; return; }
    int w = r.R - r.L, hh = r.B - r.T;
    MoveWindow(h, cx - w / 2, cy - hh / 2, w, hh, true);
    Result = "moved";
  }
}
'@
[KtermMove]::SetProcessDpiAwareness(2) | Out-Null
$deadline = (Get-Date).AddSeconds(15)
$p = $null
while ((Get-Date) -lt $deadline -and -not $p) {
  $p = Get-Process -Name kterm -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -like '*win-1*' } | Select-Object -First 1
  if (-not $p) { Start-Sleep -Milliseconds 250 }
}
if ($p) {
  [KtermMove]::Run($p.MainWindowHandle.ToInt64(), ${cx}, ${cy})
  Write-Output ([KtermMove]::Result)
} else {
  Write-Output 'notfound'
}
`
  try {
    const out = execFileSync('powershell.exe', ['-NoProfile', '-Command', ps], { encoding: 'utf8', timeout: 15000 })
    return out.includes('moved')
  } catch {
    return false
  }
}
