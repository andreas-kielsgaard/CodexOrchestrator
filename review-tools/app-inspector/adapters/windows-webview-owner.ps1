param(
    [Parameter(Mandatory = $true)]
    [string]$OwnerExecutablePath,
    [Parameter(Mandatory = $true)]
    [int]$OwnerProcessId,
    [Parameter(Mandatory = $true)]
    [int]$DebugPort
)

$ErrorActionPreference = 'Stop'

function Emit-Payload($value) {
    $encoded = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes(($value | ConvertTo-Json -Depth 6 -Compress)))
    [Console]::Out.WriteLine("REVIEW_APP_WEBVIEW_OWNER_V1:$encoded")
}

function Test-ProcessDescendantOf([uint32]$candidateProcessId, [uint32]$ownerProcessId) {
    $current = $candidateProcessId
    $visited = [System.Collections.Generic.HashSet[uint32]]::new()
    while ($current -ne 0 -and $visited.Add($current)) {
        if ($current -eq $ownerProcessId) { return $true }
        $record = Get-CimInstance Win32_Process -Filter "ProcessId=$current" -ErrorAction SilentlyContinue
        if ($null -eq $record) { return $false }
        $current = [uint32]$record.ParentProcessId
    }
    return $false
}

if ($DebugPort -lt 1 -or $DebugPort -gt 65535) { throw 'DebugPort must be from 1 through 65535.' }

$resolvedOwnerExecutable = [System.IO.Path]::GetFullPath($OwnerExecutablePath)
$owners = @(Get-CimInstance Win32_Process | Where-Object {
    $_.ProcessId -eq $OwnerProcessId -and $_.ExecutablePath -and [string]::Equals(
        [System.IO.Path]::GetFullPath($_.ExecutablePath),
        $resolvedOwnerExecutable,
        [System.StringComparison]::OrdinalIgnoreCase
    )
})
if ($owners.Count -ne 1) {
    throw "No running owner matches PID $OwnerProcessId and executable $resolvedOwnerExecutable."
}

$listenerProcessIds = @(Get-NetTCPConnection -State Listen -LocalPort $DebugPort -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty OwningProcess -Unique)
if ($listenerProcessIds.Count -ne 1) {
    throw "Expected exactly one local listener for debug port $DebugPort; observed $($listenerProcessIds.Count)."
}

$debugger = Get-CimInstance Win32_Process -Filter "ProcessId=$($listenerProcessIds[0])" -ErrorAction SilentlyContinue
if ($null -eq $debugger) { throw "The debug-port listener process could not be inspected." }
if (-not (Test-ProcessDescendantOf $debugger.ProcessId $OwnerProcessId)) {
    throw "Debug-port listener PID $($debugger.ProcessId) is not a descendant of owner PID $OwnerProcessId."
}
if ($debugger.CommandLine -notmatch "(?:^|\s)--remote-debugging-port=$DebugPort(?:\s|$)") {
    throw "Debug-port listener PID $($debugger.ProcessId) does not declare --remote-debugging-port=$DebugPort."
}

Emit-Payload ([ordered]@{
    owner = [ordered]@{ pid = $OwnerProcessId; executablePath = $resolvedOwnerExecutable }
    debugger = [ordered]@{ pid = $debugger.ProcessId; executablePath = $debugger.ExecutablePath; port = $DebugPort }
    transport = [ordered]@{ foregrounded = $false; ownership = 'owner_pid_verified_with_descendant_debug_listener' }
})
