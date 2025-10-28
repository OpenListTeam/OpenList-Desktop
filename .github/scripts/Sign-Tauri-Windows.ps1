# Sign-Tauri-Windows.ps1
# Signs Windows Tauri application binaries with Certum certificate
# Uses authenticated SimplySign Desktop connection

param(
    [string]$TargetDirectory = "tauri/src-tauri/target/x86_64-pc-windows-msvc/release",
    [string]$CertificateSHA1 = $env:CERTUM_CERTIFICATE_SHA1
)

Write-Host "=== TAURI WINDOWS BINARY SIGNING ==="
Write-Host "Target directory: $TargetDirectory"
Write-Host "Certificate SHA1: $($CertificateSHA1.Substring(0,16))... (truncated)"
Write-Host ""

# Validate inputs
if (-not $CertificateSHA1) {
    Write-Host "ERROR: CERTUM_CERTIFICATE_SHA1 environment variable not provided"
    exit 1
}

if (-not (Test-Path $TargetDirectory)) {
    Write-Host "ERROR: Target directory not found: $TargetDirectory"
    exit 1
}

# Find signtool
Write-Host "Locating signtool..."
$SignToolPaths = @(
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe",
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.20348.0\x64\signtool.exe",
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe",
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.18362.0\x64\signtool.exe",
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.17763.0\x64\signtool.exe"
)

$SignTool = $null
foreach ($path in $SignToolPaths) {
    if (Test-Path $path) {
        $SignTool = $path
        Write-Host "Found signtool: $SignTool"
        break
    }
}

if (-not $SignTool) {
    Write-Host "ERROR: signtool.exe not found in any expected location"
    exit 1
}

# Find binaries to sign
Write-Host "Scanning for Windows binaries to sign..."
$BinariesToSign = @()

# Main Tauri executable
$MainExecutable = Join-Path $TargetDirectory "docuscope-ca-desktop.exe"
if (Test-Path $MainExecutable) {
    $BinariesToSign += $MainExecutable
    Write-Host "Found main executable: $MainExecutable"
}

# NSIS installer
$NSISDir = Join-Path $TargetDirectory "bundle\nsis"
if (Test-Path $NSISDir) {
    $NSISFiles = Get-ChildItem -Path $NSISDir -Filter "*.exe"
    foreach ($file in $NSISFiles) {
        $BinariesToSign += $file.FullName
        Write-Host "Found NSIS installer: $($file.FullName)"
    }
}

# MSI installer
$MSIDir = Join-Path $TargetDirectory "bundle\msi"
if (Test-Path $MSIDir) {
    $MSIFiles = Get-ChildItem -Path $MSIDir -Filter "*.msi"
    foreach ($file in $MSIFiles) {
        $BinariesToSign += $file.FullName
        Write-Host "Found MSI installer: $($file.FullName)"
    }
}

if ($BinariesToSign.Count -eq 0) {
    Write-Host "WARNING: No binaries found to sign"
    Write-Host "Contents of target directory:"
    Get-ChildItem -Path $TargetDirectory -Recurse -File | Select-Object FullName
    exit 0
}

Write-Host ""
Write-Host "Found $($BinariesToSign.Count) binaries to sign"
Write-Host ""

# Sign each binary
$SignedCount = 0
$FailedCount = 0

foreach ($binary in $BinariesToSign) {
    $fileName = Split-Path $binary -Leaf
    Write-Host "=== Signing: $fileName ==="
    
    # Get file size
    $fileSize = (Get-Item $binary).Length
    Write-Host "File size: $fileSize bytes"
    
    # Attempt signing with official Certum method
    Write-Host "Executing: signtool sign /sha1 `"***`" /tr http://time.certum.pl /td SHA256 /fd SHA256 /v `"$binary`""
    
    $signResult = & $SignTool sign /sha1 $CertificateSHA1 /tr "http://time.certum.pl" /td SHA256 /fd SHA256 /v $binary 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "SUCCESS: $fileName signed successfully"
        $SignedCount++
        
        # Verify the signature
        Write-Host "Verifying signature..."
        $verifyResult = & $SignTool verify /pa /v $binary 2>&1
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "VERIFIED: Signature verification successful"
        } else {
            Write-Host "WARNING: Signature verification failed"
            Write-Host "Verify output: $verifyResult"
        }
        
    } else {
        Write-Host "FAILED: Signing failed for $fileName"
        Write-Host "Error output: $signResult"
        $FailedCount++
        
        # Try fallback method without /td parameter
        Write-Host "Attempting fallback method..."
        $fallbackResult = & $SignTool sign /sha1 $CertificateSHA1 /tr "http://time.certum.pl" /fd SHA256 /v $binary 2>&1
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "SUCCESS: Fallback method worked for $fileName"
            $SignedCount++
            $FailedCount--
        } else {
            Write-Host "FAILED: Fallback method also failed"
            Write-Host "Fallback output: $fallbackResult"
        }
    }
    
    Write-Host ""
}

# Final summary
Write-Host "=== SIGNING SUMMARY ==="
Write-Host "Total binaries: $($BinariesToSign.Count)"
Write-Host "Successfully signed: $SignedCount"
Write-Host "Failed to sign: $FailedCount"

if ($FailedCount -eq 0) {
    Write-Host "ALL BINARIES SIGNED SUCCESSFULLY!"
    Write-Host "Windows Tauri application is ready for distribution"
    exit 0
} else {
    Write-Host "SOME BINARIES FAILED TO SIGN"
    Write-Host "Check the error messages above"
    exit 1
}
