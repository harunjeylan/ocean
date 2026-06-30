param(
    [string]$Version = "latest"
)

$Repo = "harunjeylan/ocean"
$AppName = "ocean"
$InstallDir = "$env:LOCALAPPDATA\$AppName"
$BinDir = "$InstallDir\bin"

if ($Version -eq "latest") {
    $ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
} else {
    $ReleaseUrl = "https://api.github.com/repos/$Repo/releases/tags/$Version"
}

function Add-ToUserPath {
    param([string]$Path)
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($currentPath -split ";" -notcontains $Path) {
        $newPath = "$currentPath;$Path"
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Host "Added '$Path' to user PATH."
    } else {
        Write-Host "'$Path' is already in PATH."
    }
}

Write-Host "Installing $AppName..." -ForegroundColor Cyan

# Detect architecture
$Arch = "x86_64"
if ([Environment]::Is64BitOperatingSystem -eq $false) {
    Write-Host "32-bit systems are not supported." -ForegroundColor Red
    exit 1
}

Write-Host "Fetching release info for $ReleaseUrl ..."

try {
    $release = Invoke-RestMethod -Uri $ReleaseUrl -UseBasicParsing
    $tag = $release.tag_name
    $assetName = "ocean-$tag-x86_64-pc-windows-msvc.zip"
    $asset = $release.assets | Where-Object { $_.name -eq $assetName }

    if (-not $asset) {
        Write-Host "No pre-built binary found for '$assetName'." -ForegroundColor Red
        Write-Host "Available assets:"
        $release.assets | ForEach-Object { Write-Host "  $($_.name)" }
        exit 1
    }

    $downloadUrl = $asset.browser_download_url
    Write-Host "Downloading $downloadUrl ..."

    $zipPath = "$env:TEMP\$assetName"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

    # Create install directories
    if (-not (Test-Path -LiteralPath $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    # Extract
    Write-Host "Extracting to $BinDir ..."
    Expand-Archive -Path $zipPath -DestinationPath $BinDir -Force

    # Cleanup
    Remove-Item -Path $zipPath -Force

    # Verify
    $exePath = "$BinDir\$AppName.exe"
    if (-not (Test-Path -LiteralPath $exePath)) {
        Write-Host "Binary not found at '$exePath' after extraction." -ForegroundColor Red
        exit 1
    }

    # Add to PATH
    Add-ToUserPath -Path $BinDir

    Write-Host ""
    Write-Host "$AppName $tag installed successfully!" -ForegroundColor Green
    Write-Host "Binary: $exePath"
    Write-Host ""
    Write-Host "You may need to restart your terminal for PATH changes to take effect."
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  1. Restart your terminal (or run: `$env:Path = [Environment]::GetEnvironmentVariable('PATH','User') + ';' + [Environment]::GetEnvironmentVariable('PATH','Machine'))"
    Write-Host "  2. Run: $AppName --help"
    Write-Host "  3. cd to a project directory and run: $AppName init"

} catch {
    Write-Host "Installation failed: $_" -ForegroundColor Red
    exit 1
}
