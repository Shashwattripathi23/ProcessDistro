# ProcessDistro Build Script
# Builds all components of the ProcessDistro system

param(
    [switch]$Release,
    [switch]$Clean,
    [switch]$WasmOnly
)

Write-Host "ProcessDistro Build Script" -ForegroundColor Green
Write-Host "=========================" -ForegroundColor Green

# Set build configuration
$BuildConfig = if ($Release) { "release" } else { "debug" }
$BuildFlag = if ($Release) { "--release" } else { "" }

Write-Host "Build configuration: $BuildConfig" -ForegroundColor Yellow

# Clean previous builds if requested
if ($Clean) {
    Write-Host "Cleaning previous builds..." -ForegroundColor Yellow
    cargo clean
}

# Function to build a component
function Build-Component {
    param($Name, $Path)
    
    Write-Host "Building $Name..." -ForegroundColor Cyan
    Push-Location $Path
    
    try {
        $result = cargo build $BuildFlag
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ $Name built successfully" -ForegroundColor Green
        }
        else {
            Write-Host "✗ $Name build failed" -ForegroundColor Red
            return $false
        }
    }
    catch {
        Write-Host "✗ $Name build failed: $_" -ForegroundColor Red
        return $false
    }
    finally {
        Pop-Location
    }
    
    return $true
}

# Function to build WASM tasks
function Build-WasmTask {
    param($Name, $Path)
    
    Write-Host "Building WASM task: $Name..." -ForegroundColor Cyan
    Push-Location $Path
    
    try {
        # Build for WASM target
        cargo build --target wasm32-unknown-unknown $BuildFlag
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ WASM task $Name built successfully" -ForegroundColor Green
        }
        else {
            Write-Host "✗ WASM task $Name build failed" -ForegroundColor Red
            return $false
        }
    }
    catch {
        Write-Host "✗ WASM task $Name build failed: $_" -ForegroundColor Red
        return $false
    }
    finally {
        Pop-Location
    }
    
    return $true
}

# Ensure WASM target is installed
Write-Host "Checking WASM target..." -ForegroundColor Yellow
rustup target add wasm32-unknown-unknown

$success = $true

if (-not $WasmOnly) {
    # Build core components
    $success = $success -and (Build-Component "Common" "common")
    $success = $success -and (Build-Component "Controller" "controller")
    $success = $success -and (Build-Component "Edge Node" "edge_node")
}

# Build WASM tasks
Write-Host "`nBuilding WASM tasks..." -ForegroundColor Yellow
$success = $success -and (Build-WasmTask "Password Hash" "wasm_tasks/password_hash")
$success = $success -and (Build-WasmTask "Matrix Multiplication" "wasm_tasks/matrix_mul")
$success = $success -and (Build-WasmTask "Mandelbrot" "wasm_tasks/mandelbrot")

# Copy WASM files to output directory
if ($success) {
    Write-Host "`nCopying WASM files..." -ForegroundColor Yellow
    $wasmDir = "target/wasm"
    New-Item -ItemType Directory -Force -Path $wasmDir | Out-Null
    
    $targetDir = if ($Release) { "target/wasm32-unknown-unknown/release" } else { "target/wasm32-unknown-unknown/debug" }
    
    Copy-Item "$targetDir/password_hash.wasm" "$wasmDir/" -ErrorAction SilentlyContinue
    Copy-Item "$targetDir/matrix_mul.wasm" "$wasmDir/" -ErrorAction SilentlyContinue
    Copy-Item "$targetDir/mandelbrot.wasm" "$wasmDir/" -ErrorAction SilentlyContinue
    
    Write-Host "✓ WASM files copied to $wasmDir" -ForegroundColor Green
}

# Build summary
Write-Host "`nBuild Summary:" -ForegroundColor Yellow
Write-Host "==============" -ForegroundColor Yellow

if ($success) {
    Write-Host "✓ All components built successfully!" -ForegroundColor Green
    Write-Host "`nTo run ProcessDistro:" -ForegroundColor Cyan
    Write-Host "  Controller: cargo run --bin controller -- start" -ForegroundColor White
    Write-Host "  Edge Node:  cargo run --bin edge_node -- --controller localhost:30000" -ForegroundColor White
}
else {
    Write-Host "✗ Some components failed to build" -ForegroundColor Red
    exit 1
}

Write-Host "`nBuild completed." -ForegroundColor Green