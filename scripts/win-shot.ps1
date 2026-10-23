# Saves a PNG of the Refine window only (PrintWindow renders that window, never what's on top of it).
#   powershell -File scripts\win-shot.ps1 C:\path\out.png
param([string]$Out = "$env:TEMP\refine-shot.png")
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class W {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[W]::SetProcessDPIAware() | Out-Null
$p = Get-Process refine-app -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$r = New-Object W+RECT
[W]::GetWindowRect($p.MainWindowHandle, [ref]$r) | Out-Null
$w = $r.R - $r.L; $h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
# 2 = PW_RENDERFULLCONTENT: includes GPU-composited content such as WebView2.
$ok = [W]::PrintWindow($p.MainWindowHandle, $dc, 2)
$g.ReleaseHdc($dc)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
"saved $Out ($w x $h, printwindow=$ok)"
