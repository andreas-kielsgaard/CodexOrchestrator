param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath,
    [Parameter(Mandatory = $true)]
    [int]$ProcessId,
    [Parameter(Mandatory = $true)]
    [ValidateSet('click')]
    [string]$Action,
    [Parameter(Mandatory = $true)]
    [int]$X,
    [Parameter(Mandatory = $true)]
    [int]$Y
)

$ErrorActionPreference = 'Stop'

if ($X -lt 0 -or $Y -lt 0 -or $X -gt 32767 -or $Y -gt 32767) {
    throw 'Client coordinates must be integers from 0 through 32767.'
}

function Emit-Payload($value) {
    $encoded = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes(($value | ConvertTo-Json -Depth 8 -Compress)))
    [Console]::Out.WriteLine("REVIEW_APP_INTERACTION_V1:$encoded")
}

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ReviewAppInteractionNative {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT point);
  [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr hWnd, ref POINT point);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc callback, IntPtr lParam);
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd, uint message, UIntPtr wParam, IntPtr lParam, uint flags, uint timeout, out UIntPtr result);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
}
'@

function Send-TargetMessage([IntPtr]$target, [uint32]$message, [IntPtr]$lParam) {
    $result = [UIntPtr]::Zero
    $sent = [ReviewAppInteractionNative]::SendMessageTimeout(
        $target,
        $message,
        [UIntPtr]::Zero,
        $lParam,
        0x0002,
        5000,
        [ref]$result
    )
    if ($sent -eq [IntPtr]::Zero) { throw "Window message 0x$('{0:X}' -f $message) timed out or failed." }
}

function Find-TargetChild([IntPtr]$parent, $screenPoint) {
    $candidates = [System.Collections.Generic.List[System.IntPtr]]::new()
    $callback = [ReviewAppInteractionNative+EnumWindowsProc] {
        param([IntPtr]$child, [IntPtr]$ignored)
        $candidates.Add($child)
        return $true
    }
    [ReviewAppInteractionNative]::EnumChildWindows($parent, $callback, [IntPtr]::Zero) | Out-Null
    $selected = [IntPtr]::Zero
    $selectedArea = [Int64]::MaxValue
    foreach ($candidate in $candidates) {
        $rect = New-Object ReviewAppInteractionNative+RECT
        if (-not [ReviewAppInteractionNative]::GetWindowRect($candidate, [ref]$rect)) { continue }
        if ($screenPoint.X -lt $rect.Left -or $screenPoint.X -ge $rect.Right -or $screenPoint.Y -lt $rect.Top -or $screenPoint.Y -ge $rect.Bottom) { continue }
        $area = [Int64]($rect.Right - $rect.Left) * [Int64]($rect.Bottom - $rect.Top)
        if ($area -gt 0 -and $area -lt $selectedArea) { $selected = $candidate; $selectedArea = $area }
    }
    return $selected
}

function Test-SelectedProcessOrDescendant([uint32]$candidateProcessId, [uint32]$selectedProcessId) {
    $current = $candidateProcessId
    $visited = [System.Collections.Generic.HashSet[uint32]]::new()
    while ($current -ne 0 -and $visited.Add($current)) {
        if ($current -eq $selectedProcessId) { return $true }
        $record = Get-CimInstance Win32_Process -Filter "ProcessId=$current" -ErrorAction SilentlyContinue
        if ($null -eq $record) { return $false }
        $current = [uint32]$record.ParentProcessId
    }
    return $false
}

$resolvedExecutable = [System.IO.Path]::GetFullPath($ExecutablePath)
$candidates = @(Get-CimInstance Win32_Process | Where-Object {
    $_.ProcessId -eq $ProcessId -and $_.ExecutablePath -and [string]::Equals(
        [System.IO.Path]::GetFullPath($_.ExecutablePath),
        $resolvedExecutable,
        [System.StringComparison]::OrdinalIgnoreCase
    )
})
if ($candidates.Count -ne 1) { throw "No running process matches PID $ProcessId and executable $resolvedExecutable." }
$process = Get-Process -Id $ProcessId
if ($process.MainWindowHandle -eq 0) { throw 'The selected process has no observable main window handle.' }

$clientRect = New-Object ReviewAppInteractionNative+RECT
if (-not [ReviewAppInteractionNative]::GetClientRect($process.MainWindowHandle, [ref]$clientRect)) {
    throw 'GetClientRect failed for the selected main window.'
}
if ($X -lt $clientRect.Left -or $Y -lt $clientRect.Top -or $X -ge $clientRect.Right -or $Y -ge $clientRect.Bottom) {
    throw "The requested client coordinate ($X, $Y) lies outside the selected main window client area."
}

$point = New-Object ReviewAppInteractionNative+POINT
$point.X = $X
$point.Y = $Y
if (-not [ReviewAppInteractionNative]::ClientToScreen($process.MainWindowHandle, [ref]$point)) {
    throw 'ClientToScreen failed for the selected main window.'
}
$target = Find-TargetChild $process.MainWindowHandle $point
if ($target -eq [IntPtr]::Zero) { throw 'No descendant child window exists at the requested client coordinate.' }
$targetProcessId = 0
[ReviewAppInteractionNative]::GetWindowThreadProcessId($target, [ref]$targetProcessId) | Out-Null
if (-not (Test-SelectedProcessOrDescendant $targetProcessId $ProcessId)) {
    throw "The target child window belongs to PID $targetProcessId, which is not the selected PID $ProcessId or its descendant."
}
if (-not [ReviewAppInteractionNative]::ScreenToClient($target, [ref]$point)) {
    throw 'ScreenToClient failed for the target child window.'
}

$packedPoint = [Int64](($point.Y -band 0xffff) -shl 16) -bor [Int64]($point.X -band 0xffff)
Send-TargetMessage $target 0x0201 ([IntPtr]$packedPoint)
Send-TargetMessage $target 0x0202 ([IntPtr]$packedPoint)

Emit-Payload ([ordered]@{
    process = [ordered]@{ pid = $ProcessId; executablePath = $resolvedExecutable }
    action = [ordered]@{ kind = $Action; clientPoint = [ordered]@{ x = $X; y = $Y }; targetHandle = [Int64]$target; targetProcessId = $targetProcessId }
    transport = [ordered]@{ foregrounded = $false; delivery = 'window_messages_acknowledged'; semanticOutcome = 'not_observed' }
})
