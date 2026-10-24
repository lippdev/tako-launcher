# Private Working Set (bytes residentes privados), nunca Private Bytes/commit.
param(
    [Parameter(Mandatory = $true)][int]$RootPid,
    [Parameter(Mandatory = $true)][string]$Label
)
$ErrorActionPreference = 'Stop'
$allProcesses = @(Get-CimInstance Win32_Process)
$treeIds = [System.Collections.Generic.HashSet[int]]::new()
[void]$treeIds.Add($RootPid)
do {
    $previousCount = $treeIds.Count
    foreach ($process in $allProcesses) {
        if ($treeIds.Contains([int]$process.ParentProcessId)) { [void]$treeIds.Add([int]$process.ProcessId) }
    }
} while ($treeIds.Count -ne $previousCount)
$counters = @{}
Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | ForEach-Object {
    $counters[[int]$_.IDProcess] = [long]$_.WorkingSetPrivate
}
$rows = @($allProcesses | Where-Object { $treeIds.Contains([int]$_.ProcessId) } | ForEach-Object {
    if (-not $counters.ContainsKey([int]$_.ProcessId)) { throw "Processo sem contador: $($_.ProcessId); repita a amostra." }
    $role = if ($_.ProcessId -eq $RootPid) { 'core+cef-browser' }
        elseif ($_.CommandLine -match '--type=([^\s]+)') { $Matches[1] } else { $_.Name }
    [PSCustomObject]@{ pid = [int]$_.ProcessId; parent = [int]$_.ParentProcessId; role = $role; privateWorkingSetBytes = $counters[[int]$_.ProcessId] }
})
if (-not ($rows | Where-Object { $_.pid -eq $RootPid })) { throw 'Processo principal ausente.' }
$total = [long](($rows | Measure-Object -Property privateWorkingSetBytes -Sum).Sum)
[PSCustomObject]@{
    timestamp = [DateTimeOffset]::Now.ToString('o'); label = $Label
    totalBytes = $total; totalMiB = [math]::Round($total / 1MB, 2)
    processCount = $rows.Count; subprocessCount = $rows.Count - 1
    cefSubprocessCount = @($rows | Where-Object { $_.role -in @('renderer', 'gpu-process', 'utility') }).Count
    processes = $rows
} | ConvertTo-Json -Depth 5
