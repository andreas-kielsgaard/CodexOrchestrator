param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath,
    [int]$ProcessId = 0,
    [string]$ScreenshotPath,
    [switch]$SkipAccessibility
)

$ErrorActionPreference = 'Stop'

function Observed($value, [string]$source) {
    return [ordered]@{ disposition = 'observed'; source = $source; value = $value }
}

function Unavailable([string]$reason) {
    return [ordered]@{ disposition = 'unavailable'; reason = $reason }
}

function Emit-Payload($value) {
    $json = $value | ConvertTo-Json -Depth 8 -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
    $encoded = [Convert]::ToBase64String($bytes)
    [Console]::Out.WriteLine("REVIEW_APP_JSON_V1:$encoded")
}

$resolvedExecutable = [System.IO.Path]::GetFullPath($ExecutablePath)
$candidates = @(Get-CimInstance Win32_Process | Where-Object {
    $_.ExecutablePath -and [string]::Equals(
        [System.IO.Path]::GetFullPath($_.ExecutablePath),
        $resolvedExecutable,
        [System.StringComparison]::OrdinalIgnoreCase
    )
})

if ($ProcessId -gt 0) {
    $candidates = @($candidates | Where-Object { $_.ProcessId -eq $ProcessId })
}

if ($candidates.Count -eq 0) {
    $reason = if ($ProcessId -gt 0) {
        "No running process matches PID $ProcessId and executable $resolvedExecutable."
    } else {
        "No running process matches executable $resolvedExecutable."
    }
    Emit-Payload ([ordered]@{
        process = Unavailable $reason
        window = Unavailable $reason
        screenshot = Unavailable $reason
        accessibility = Unavailable $reason
    })
    exit 0
}

if ($candidates.Count -gt 1) {
    $ids = ($candidates.ProcessId | Sort-Object) -join ', '
    $reason = "More than one process matches the executable ($ids); pass --pid to select one."
    Emit-Payload ([ordered]@{
        process = Unavailable $reason
        window = Unavailable $reason
        screenshot = Unavailable $reason
        accessibility = Unavailable $reason
    })
    exit 0
}

$candidate = $candidates[0]
$process = Get-Process -Id $candidate.ProcessId
$processObservation = Observed ([ordered]@{
    running = $true
    pid = [int]$candidate.ProcessId
    parentPid = [int]$candidate.ParentProcessId
    executablePath = $candidate.ExecutablePath
    createdAt = ([DateTime]$candidate.CreationDate).ToUniversalTime().ToString('o')
}) 'Win32_Process exact executable-path match'

if ($process.MainWindowHandle -eq 0) {
    $reason = 'The selected process has no observable main window handle.'
    Emit-Payload ([ordered]@{
        process = $processObservation
        window = Unavailable $reason
        screenshot = Unavailable $reason
        accessibility = Unavailable $reason
    })
    exit 0
}

$windowObservation = Observed ([ordered]@{
    handle = [Int64]$process.MainWindowHandle
    title = $process.MainWindowTitle
    responding = $process.Responding
}) 'Get-Process main window'

$accessibilityObservation = Unavailable 'Accessibility inspection was not requested during change polling.'
if (-not $SkipAccessibility) {
  try {
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $items = $root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    $elements = @()
    for ($index = 0; $index -lt [Math]::Min($items.Count, 400); $index++) {
        try {
            $current = $items.Item($index).Current
            if (-not $current.IsOffscreen -and ($current.Name -or $current.AutomationId)) {
                $elements += [ordered]@{
                    name = $current.Name
                    automationId = $current.AutomationId
                    controlType = $current.ControlType.ProgrammaticName
                    className = $current.ClassName
                    enabled = $current.IsEnabled
                }
            }
        } catch {
            # A WebView child may disappear while the tree is enumerated; omit only that child.
        }
    }
    $accessibilityObservation = Observed ([ordered]@{
        visibleElements = $elements
        semanticContentAvailable = @($elements | Where-Object {
            $_.controlType -notin @('ControlType.Pane', 'ControlType.Window')
        }).Count -gt 0
    }) 'Windows UI Automation read-only tree'
  } catch {
    $accessibilityObservation = Unavailable "Windows UI Automation failed: $($_.Exception.Message)"
  }
}

$screenshotObservation = Unavailable 'Screenshot capture was not requested.'
if ($ScreenshotPath) {
    try {
        Add-Type -AssemblyName System.Drawing
        if (-not ('ReviewAppCaptureNative' -as [type])) {
            Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ReviewAppCaptureNative {
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdcBlt, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
'@
        }
        $rect = New-Object ReviewAppCaptureNative+RECT
        if (-not [ReviewAppCaptureNative]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) {
            throw 'GetWindowRect returned false.'
        }
        $width = $rect.Right - $rect.Left
        $height = $rect.Bottom - $rect.Top
        if ($width -le 0 -or $height -le 0) {
            throw "Invalid window bounds ${width}x${height}."
        }
        $bitmap = New-Object System.Drawing.Bitmap $width, $height
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $deviceContext = $graphics.GetHdc()
        try {
            if (-not [ReviewAppCaptureNative]::PrintWindow(
                $process.MainWindowHandle,
                $deviceContext,
                2
            )) {
                throw 'PrintWindow returned false.'
            }
        } finally {
            $graphics.ReleaseHdc($deviceContext)
            $graphics.Dispose()
        }
        $bitmap.Save($ScreenshotPath, [System.Drawing.Imaging.ImageFormat]::Png)
        $bitmap.Dispose()
        $file = Get-Item -LiteralPath $ScreenshotPath
        $hash = Get-FileHash -LiteralPath $ScreenshotPath -Algorithm SHA256
        $screenshotObservation = Observed ([ordered]@{
            path = $file.FullName
            width = $width
            height = $height
            bytes = [Int64]$file.Length
            sha256 = $hash.Hash.ToLowerInvariant()
            captureMethod = 'PrintWindow(PW_RENDERFULLCONTENT)'
        }) 'Win32 window render capture'
    } catch {
        $screenshotObservation = Unavailable "Native window capture failed: $($_.Exception.Message)"
    }
}

Emit-Payload ([ordered]@{
    process = $processObservation
    window = $windowObservation
    screenshot = $screenshotObservation
    accessibility = $accessibilityObservation
})
