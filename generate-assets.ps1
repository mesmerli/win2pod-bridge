Add-Type -AssemblyName System.Drawing

$assetsDir = "package\msix\Assets"
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null

$guiAssetsDir = "crates\homepod-cast-gui\assets"
New-Item -ItemType Directory -Force -Path $guiAssetsDir | Out-Null

function Draw-PodIcon {
    param(
        [System.Drawing.Graphics]$g,
        [int]$w,
        [int]$h,
        [bool]$showText = $false,
        [bool]$transparentBg = $false,
        [string]$statusDot = $null
    )

    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic

    if (-not $transparentBg) {
        # Dark sleek modern gradient
        $bgBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            [System.Drawing.Point]::new(0, 0),
            [System.Drawing.Point]::new($w, $h),
            [System.Drawing.Color]::FromArgb(255, 24, 30, 48),
            [System.Drawing.Color]::FromArgb(255, 12, 16, 26)
        )
        $g.FillRectangle($bgBrush, 0, 0, $w, $h)
        $bgBrush.Dispose()
    } else {
        $g.Clear([System.Drawing.Color]::Transparent)
    }

    # Center and radius calculation
    $cx = [int]($w / 2)
    $cy = [int]($h / 2)
    $radius = if ($transparentBg) { [int]([Math]::Min($w, $h) * 0.44) } else { [int]([Math]::Min($w, $h) * 0.38) }

    if ($showText) {
        $cx = [int]($w * 0.3)
        $radius = [int]($h * 0.35)
    }

    # Outer glow / circular body
    $bodyRect = [System.Drawing.Rectangle]::new($cx - $radius, $cy - $radius, $radius * 2, $radius * 2)
    $gradBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $bodyRect,
        [System.Drawing.Color]::FromArgb(255, 0, 168, 255),
        [System.Drawing.Color]::FromArgb(255, 114, 9, 183),
        [System.Drawing.Drawing2D.LinearGradientMode]::ForwardDiagonal
    )
    $g.FillEllipse($gradBrush, $bodyRect)
    $gradBrush.Dispose()

    # Inner mesh / Pod center
    $innerR = [int]($radius * 0.72)
    $innerRect = [System.Drawing.Rectangle]::new($cx - $innerR, $cy - $innerR, $innerR * 2, $innerR * 2)
    $innerBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(235, 20, 24, 36))
    $g.FillEllipse($innerBrush, $innerRect)
    $innerBrush.Dispose()

    # Center triangle (AirPlay / Play icon)
    $triSize = [int]($radius * 0.32)
    $p1 = [System.Drawing.PointF]::new($cx - $triSize * 0.4, $cy - $triSize * 0.6)
    $p2 = [System.Drawing.PointF]::new($cx - $triSize * 0.4, $cy + $triSize * 0.6)
    $p3 = [System.Drawing.PointF]::new($cx + $triSize * 0.7, $cy)
    $triBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $g.FillPolygon($triBrush, @($p1, $p2, $p3))
    $triBrush.Dispose()

    # Acoustic wave arcs
    if ($radius -ge 12) {
        $penWidth = [Math]::Max(1.5, $radius * 0.08)
        $pen = [System.Drawing.Pen]::new([System.Drawing.Color]::White, [float]$penWidth)
        $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
        $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round

        $arcR1 = [int]($innerR * 0.55)
        if ($arcR1 -gt 2) {
            $g.DrawArc($pen, $cx - $arcR1, $cy - $arcR1, $arcR1 * 2, $arcR1 * 2, -45, 90)
        }
        $arcR2 = [int]($innerR * 0.85)
        if ($arcR2 -gt 4) {
            $g.DrawArc($pen, $cx - $arcR2, $cy - $arcR2, $arcR2 * 2, $arcR2 * 2, -50, 100)
        }
        $pen.Dispose()
    }

    # Status indicator dot for tray icons (e.g., green dot when streaming)
    if ($statusDot) {
        $dotR = [int]($radius * 0.38)
        $dotCx = $cx + [int]($radius * 0.62)
        $dotCy = $cy + [int]($radius * 0.62)

        # Dark halo to separate from body
        $haloBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 12, 16, 26))
        $g.FillEllipse($haloBrush, $dotCx - $dotR - 1, $dotCy - $dotR - 1, ($dotR + 1) * 2, ($dotR + 1) * 2)
        $haloBrush.Dispose()

        $dotColor = switch ($statusDot) {
            "green" { [System.Drawing.Color]::FromArgb(255, 50, 205, 50) }
            "amber" { [System.Drawing.Color]::FromArgb(255, 240, 160, 40) }
            "red"   { [System.Drawing.Color]::FromArgb(255, 240, 60, 60) }
            default { [System.Drawing.Color]::FromArgb(255, 140, 140, 140) }
        }
        $dotBrush = [System.Drawing.SolidBrush]::new($dotColor)
        $g.FillEllipse($dotBrush, $dotCx - $dotR, $dotCy - $dotR, $dotR * 2, $dotR * 2)
        $dotBrush.Dispose()
    }

    if ($showText) {
        $font = [System.Drawing.Font]::new("Segoe UI", [float]($h * 0.22), [System.Drawing.FontStyle]::Bold)
        $textBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
        $subFont = [System.Drawing.Font]::new("Segoe UI", [float]($h * 0.12), [System.Drawing.FontStyle]::Regular)
        $subBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(200, 160, 180, 210))

        $g.DrawString("Win2Pod", $font, $textBrush, [float]($w * 0.52), [float]($h * 0.28))
        $g.DrawString("Bridge", $subFont, $subBrush, [float]($w * 0.53), [float]($h * 0.56))

        $font.Dispose()
        $textBrush.Dispose()
        $subFont.Dispose()
        $subBrush.Dispose()
    }
}

# Function to pack PNG byte arrays into an ICO file
function Build-Ico {
    param(
        [string[]]$PngPaths,
        [string]$OutPath
    )
    $count = $PngPaths.Length
    $header = [byte[]](0, 0, 1, 0, ($count -band 0xFF), (($count -shr 8) -band 0xFF))
    $dirSize = 16 * $count
    $currentOffset = 6 + $dirSize

    $entries = [System.Collections.Generic.List[byte]]::new()
    $data = [System.Collections.Generic.List[byte]]::new()

    foreach ($p in $PngPaths) {
        $bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $p).Path)
        $ms = [System.IO.MemoryStream]::new($bytes)
        $img = [System.Drawing.Image]::FromStream($ms)
        $w = if ($img.Width -ge 256) { [byte]0 } else { [byte]$img.Width }
        $h = if ($img.Height -ge 256) { [byte]0 } else { [byte]$img.Height }
        $img.Dispose()
        $ms.Dispose()

        $len = $bytes.Length
        $entry = [byte[]](
            $w, $h, 0, 0,
            1, 0,
            32, 0,
            ($len -band 0xFF), (($len -shr 8) -band 0xFF), (($len -shr 16) -band 0xFF), (($len -shr 24) -band 0xFF),
            ($currentOffset -band 0xFF), (($currentOffset -shr 8) -band 0xFF), (($currentOffset -shr 16) -band 0xFF), (($currentOffset -shr 24) -band 0xFF)
        )
        $entries.AddRange($entry)
        $data.AddRange($bytes)
        $currentOffset += $len
    }

    $all = [System.Collections.Generic.List[byte]]::new()
    $all.AddRange($header)
    $all.AddRange($entries)
    $all.AddRange($data)
    [System.IO.File]::WriteAllBytes($OutPath, $all.ToArray())
}

# 1. Generate MSIX Assets
$msixSizes = @(
    @{ Name = "StoreLogo.png"; Width = 50; Height = 50; Text = $false; Trans = $true },
    @{ Name = "Square44x44Logo.png"; Width = 44; Height = 44; Text = $false; Trans = $true },
    @{ Name = "Square150x150Logo.png"; Width = 150; Height = 150; Text = $false; Trans = $false },
    @{ Name = "Wide310x150Logo.png"; Width = 310; Height = 150; Text = $true; Trans = $false },
    @{ Name = "SplashScreen.png"; Width = 620; Height = 300; Text = $true; Trans = $false }
)

foreach ($s in $msixSizes) {
    $bmp = [System.Drawing.Bitmap]::new($s.Width, $s.Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    Draw-PodIcon -g $g -w $s.Width -h $s.Height -showText $s.Text -transparentBg $s.Trans
    $g.Dispose()
    $outPath = Join-Path $assetsDir $s.Name
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Generated: $outPath ($($s.Width)x$($s.Height))"
}

# 2. Generate Tray Icons (32x32)
$trayVariants = @(
    @{ Name = "tray_idle.png"; Dot = $null },
    @{ Name = "tray_streaming.png"; Dot = "green" },
    @{ Name = "tray_connecting.png"; Dot = "amber" },
    @{ Name = "tray_error.png"; Dot = "red" }
)

foreach ($t in $trayVariants) {
    $bmp = [System.Drawing.Bitmap]::new(32, 32)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    Draw-PodIcon -g $g -w 32 -h 32 -showText $false -transparentBg $true -statusDot $t.Dot
    $g.Dispose()
    $outPath = Join-Path $guiAssetsDir $t.Name
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Generated Tray Icon: $outPath"
}

# 3. Generate Window & App PNG (64x64) for eframe viewport icon
$appBmp = [System.Drawing.Bitmap]::new(64, 64)
$appG = [System.Drawing.Graphics]::FromImage($appBmp)
Draw-PodIcon -g $appG -w 64 -h 64 -showText $false -transparentBg $true
$appG.Dispose()
$appPngPath = Join-Path $guiAssetsDir "app_icon.png"
$appBmp.Save($appPngPath, [System.Drawing.Imaging.ImageFormat]::Png)
$appBmp.Dispose()
Write-Host "Generated App Icon PNG: $appPngPath"

# 4. Generate Multi-Resolution ICO files for Windows Executable Resource
$icoTmpDir = "package\msix\Assets\ico_tmp"
New-Item -ItemType Directory -Force -Path $icoTmpDir | Out-Null
$icoSizes = @(16, 24, 32, 48, 64, 128, 256)
$icoPngPaths = @()

foreach ($sz in $icoSizes) {
    $bmp = [System.Drawing.Bitmap]::new($sz, $sz)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    Draw-PodIcon -g $g -w $sz -h $sz -showText $false -transparentBg $true
    $g.Dispose()
    $p = Join-Path $icoTmpDir "icon_$sz.png"
    $bmp.Save($p, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    $icoPngPaths += $p
}

$msixIco = Join-Path $assetsDir "app_icon.ico"
$guiIco = "crates\homepod-cast-gui\app_icon.ico"
Build-Ico -PngPaths $icoPngPaths -OutPath $msixIco
Copy-Item $msixIco $guiIco -Force
Remove-Item -Recurse -Force $icoTmpDir
Write-Host "Generated Windows Executable ICO: $guiIco & $msixIco"
