Add-Type -AssemblyName System.Drawing

$pngPath = Join-Path $PSScriptRoot 'ui\assets\app_logo.png'
$png = [System.Drawing.Image]::FromFile($pngPath)
$sizes = @(256, 128, 64, 48, 32, 16)
$icoPath = Join-Path $PSScriptRoot 'app.ico'

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)

# ICO header
$bw.Write([int16]0)              # Reserved
$bw.Write([int16]1)              # Type: ICO
$bw.Write([int16]$sizes.Count)   # Number of images

# Render each size to PNG bytes
$imageDataList = @()
foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($png, [System.Drawing.Size]::new($size, $size))
    $imgMs = New-Object System.IO.MemoryStream
    $bmp.Save($imgMs, [System.Drawing.Imaging.ImageFormat]::Png)
    $imageDataList += ,@{ Size=$size; Data=$imgMs.ToArray() }
    $bmp.Dispose()
    $imgMs.Dispose()
}

# Directory entries (6 header + count*16 bytes)
$offset = 6 + $sizes.Count * 16
foreach ($img in $imageDataList) {
    $s = if ($img.Size -eq 256) { 0 } else { $img.Size }
    $bw.Write([byte]$s)
    $bw.Write([byte]$s)
    $bw.Write([byte]0)           # Color count
    $bw.Write([byte]0)           # Reserved
    $bw.Write([int16]1)          # Planes
    $bw.Write([int16]32)         # Bit count
    $bw.Write([int32]$img.Data.Length)
    $bw.Write([int32]$offset)
    $offset += $img.Data.Length
}

# Image data
foreach ($img in $imageDataList) {
    $bw.Write($img.Data)
}

[System.IO.File]::WriteAllBytes($icoPath, $ms.ToArray())
$png.Dispose()
$bw.Dispose()
$ms.Dispose()

Write-Host "ICO created successfully: $icoPath"
