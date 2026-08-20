param(
    [string]$ExePath = "$PSScriptRoot\..\target\release\raven-native.exe",
    [int]$Seconds = 30
)

$resolved = Resolve-Path $ExePath -ErrorAction Stop
$process = Start-Process -FilePath $resolved -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3

$samples = @()
$lastCpu = $process.TotalProcessorTime.TotalMilliseconds
$lastTime = Get-Date

try {
    for ($i = 0; $i -lt $Seconds; $i++) {
        Start-Sleep -Seconds 1
        $process.Refresh()
        $now = Get-Date
        $cpu = $process.TotalProcessorTime.TotalMilliseconds
        $elapsedMs = ($now - $lastTime).TotalMilliseconds
        $cpuPct = if ($elapsedMs -gt 0) {
            (($cpu - $lastCpu) / $elapsedMs) * 100 / [Environment]::ProcessorCount
        } else {
            0
        }
        $samples += [pscustomobject]@{
            Second = $i + 1
            CpuPct = [Math]::Round($cpuPct, 3)
            WorkingSetMB = [Math]::Round($process.WorkingSet64 / 1MB, 2)
            PrivateMB = [Math]::Round($process.PrivateMemorySize64 / 1MB, 2)
        }
        $lastCpu = $cpu
        $lastTime = $now
    }
}
finally {
    if (!$process.HasExited) {
        $process.CloseMainWindow() | Out-Null
        Start-Sleep -Milliseconds 500
        if (!$process.HasExited) {
            $process.Kill()
        }
    }
}

$avgCpu = ($samples | Measure-Object CpuPct -Average).Average
$maxCpu = ($samples | Measure-Object CpuPct -Maximum).Maximum
$avgPrivate = ($samples | Measure-Object PrivateMB -Average).Average
$maxPrivate = ($samples | Measure-Object PrivateMB -Maximum).Maximum

[pscustomobject]@{
    Seconds = $Seconds
    AverageCpuPct = [Math]::Round($avgCpu, 3)
    MaxCpuPct = [Math]::Round($maxCpu, 3)
    AveragePrivateMB = [Math]::Round($avgPrivate, 2)
    MaxPrivateMB = [Math]::Round($maxPrivate, 2)
    TargetCpuPct = "0-1"
    TargetRamMB = "40-120"
}
