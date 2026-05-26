# Search for dlltool.exe in common MinGW/MSYS2 locations
$mingwPaths = @(
    "C:\mingw64\bin",
    "C:\msys64\mingw64\bin",
    "C:\msys64\ucrt64\bin",
    "C:\MinGW\bin",
    "C:\Program Files\mingw-w64\*\mingw64\bin",
    "C:\Program Files (x86)\mingw-w64\*\mingw64\bin",
    "$env:USERPROFILE\.rustup\toolchains\nightly-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained",
    "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained"
)

$foundPath = $null
foreach ($path in $mingwPaths) {
    $resolved = Resolve-Path $path -ErrorAction SilentlyContinue
    if ($resolved) {
        $resolvedPath = $resolved.Path
        if ($resolved -is [System.Array]) {
            $resolvedPath = $resolved[0].Path
        }
        $dlltool = Join-Path $resolvedPath "dlltool.exe"
        $as = Join-Path $resolvedPath "as.exe"
        if ((Test-Path $dlltool) -and (Test-Path $as)) {
            $foundPath = $resolvedPath
            break
        }
    }
}

if ($foundPath) {
    Write-Host "Found MinGW toolchain at: $foundPath" -ForegroundColor Green
    $env:PATH = "$foundPath;" + $env:PATH
    Write-Host "Added to session PATH." -ForegroundColor Green
} else {
    Write-Host "Warning: Could not automatically locate dlltool.exe. If build fails, ensure your MinGW bin folder is in your PATH." -ForegroundColor Yellow
}

# Ensure nightly override is set to use GNU compiler for building host dependencies of the bootloader
Write-Host "Configuring nightly toolchain to use GNU ABI target..." -ForegroundColor Cyan
rustup toolchain install nightly-x86_64-pc-windows-gnu
rustup component add llvm-tools-preview --toolchain nightly-x86_64-pc-windows-gnu
rustup override set nightly-x86_64-pc-windows-gnu

# Run bootimage command
cargo bootimage --manifest-path kernel/Cargo.toml --target x86_64-novaschool_os.json
