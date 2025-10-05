# ProcessDistro Test Script
# Runs all tests for the ProcessDistro system

param(
    [switch]$Unit,
    [switch]$Integration,
    [switch]$Coverage
)

Write-Host "ProcessDistro Test Script" -ForegroundColor Green
Write-Host "========================" -ForegroundColor Green

# Function to run tests for a component
function Test-Component {
    param($Name, $Path)
    
    Write-Host "Testing $Name..." -ForegroundColor Cyan
    Push-Location $Path
    
    try {
        if ($Coverage) {
            # Run with coverage (requires cargo-tarpaulin)
            cargo tarpaulin --out Html --output-dir ../target/coverage
        }
        else {
            cargo test
        }
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ $Name tests passed" -ForegroundColor Green
        }
        else {
            Write-Host "✗ $Name tests failed" -ForegroundColor Red
            return $false
        }
    }
    catch {
        Write-Host "✗ $Name tests failed: $_" -ForegroundColor Red
        return $false
    }
    finally {
        Pop-Location
    }
    
    return $true
}

$success = $true

# Unit tests
if ($Unit -or (-not $Integration)) {
    Write-Host "`nRunning unit tests..." -ForegroundColor Yellow
    
    $success = $success -and (Test-Component "Common" "common")
    $success = $success -and (Test-Component "Controller" "controller")
    $success = $success -and (Test-Component "Edge Node" "edge_node")
    
    # Test WASM tasks
    $success = $success -and (Test-Component "Password Hash WASM" "wasm_tasks/password_hash")
    $success = $success -and (Test-Component "Matrix Mul WASM" "wasm_tasks/matrix_mul")
    $success = $success -and (Test-Component "Mandelbrot WASM" "wasm_tasks/mandelbrot")
}

# Integration tests
if ($Integration) {
    Write-Host "`nRunning integration tests..." -ForegroundColor Yellow
    
    # Build everything first
    cargo build --release
    
    if ($LASTEXITCODE -eq 0) {
        # Start controller in background
        Write-Host "Starting controller for integration tests..." -ForegroundColor Cyan
        $controller = Start-Process -FilePath "cargo" -ArgumentList "run --bin controller -- start --port 30001" -PassThru -NoNewWindow
        
        Start-Sleep 3  # Wait for controller to start
        
        try {
            # Run integration test suite
            cargo test --test integration_tests
            
            if ($LASTEXITCODE -eq 0) {
                Write-Host "✓ Integration tests passed" -ForegroundColor Green
            }
            else {
                Write-Host "✗ Integration tests failed" -ForegroundColor Red
                $success = $false
            }
        }
        finally {
            # Stop controller
            if (-not $controller.HasExited) {
                Stop-Process -Id $controller.Id -Force
                Write-Host "Controller stopped" -ForegroundColor Yellow
            }
        }
    }
    else {
        Write-Host "✗ Build failed, skipping integration tests" -ForegroundColor Red
        $success = $false
    }
}

# Benchmark tests
Write-Host "`nRunning benchmark tests..." -ForegroundColor Yellow
python tools/benchmark.py

# Test summary
Write-Host "`nTest Summary:" -ForegroundColor Yellow
Write-Host "=============" -ForegroundColor Yellow

if ($success) {
    Write-Host "✓ All tests passed!" -ForegroundColor Green
    
    if ($Coverage) {
        Write-Host "✓ Coverage report generated in target/coverage/" -ForegroundColor Cyan
    }
}
else {
    Write-Host "✗ Some tests failed" -ForegroundColor Red
    exit 1
}

Write-Host "`nTesting completed." -ForegroundColor Green