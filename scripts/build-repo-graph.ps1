param(
    [switch]$KeepTarget
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "crates/repo-graph/Cargo.toml"
$target = Join-Path $root "target/repo-graph"
$bindgen = Join-Path $root ".tmp/repo-graph-bindgen"
$assets = Join-Path $root "assets"
$originalEncodedRustflags = $env:CARGO_ENCODED_RUSTFLAGS
$profileRoot = [Environment]::GetFolderPath("UserProfile")
$rustflagSeparator = [char]0x1f
$pathRemap = "--remap-path-prefix=$profileRoot=/source"

if (Test-Path -LiteralPath $bindgen) {
    $resolved = (Resolve-Path -LiteralPath $bindgen).Path
    $expectedRoot = (Resolve-Path -LiteralPath $root).Path
    if (-not $resolved.StartsWith($expectedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing to remove a bindgen directory outside the repository"
    }
    [System.IO.Directory]::Delete($resolved, $true)
}

New-Item -ItemType Directory -Force -Path $bindgen | Out-Null
try {
    $env:MER3LY_GRAPH_TARGET = $target
    if ([string]::IsNullOrEmpty($originalEncodedRustflags)) {
        $env:CARGO_ENCODED_RUSTFLAGS = $pathRemap
    }
    else {
        $env:CARGO_ENCODED_RUSTFLAGS = "$originalEncodedRustflags$rustflagSeparator$pathRemap"
    }
    cargo build `
        --manifest-path $manifest `
        --target wasm32-unknown-unknown `
        --target-dir $env:MER3LY_GRAPH_TARGET `
        --release
    if ($LASTEXITCODE -ne 0) {
        throw "repository graph Wasm build failed"
    }

    $wasm = Join-Path $target "wasm32-unknown-unknown/release/mer3ly_repo_graph.wasm"
    wasm-bindgen `
        --target web `
        --no-typescript `
        --out-dir $bindgen `
        $wasm
    if ($LASTEXITCODE -ne 0) {
        throw "wasm-bindgen failed"
    }

    Copy-Item `
        -LiteralPath (Join-Path $bindgen "mer3ly_repo_graph.js") `
        -Destination (Join-Path $assets "mer3ly_repo_graph.js") `
        -Force
    Copy-Item `
        -LiteralPath (Join-Path $bindgen "mer3ly_repo_graph_bg.wasm") `
        -Destination (Join-Path $assets "mer3ly_repo_graph_bg.wasm") `
        -Force
}
finally {
    if (Test-Path -LiteralPath $bindgen) {
        [System.IO.Directory]::Delete((Resolve-Path -LiteralPath $bindgen).Path, $true)
    }
    Remove-Item Env:MER3LY_GRAPH_TARGET -ErrorAction SilentlyContinue
    if ([string]::IsNullOrEmpty($originalEncodedRustflags)) {
        Remove-Item Env:CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
    }
    else {
        $env:CARGO_ENCODED_RUSTFLAGS = $originalEncodedRustflags
    }
}

if (-not $KeepTarget -and (Test-Path -LiteralPath $target)) {
    $resolvedTarget = (Resolve-Path -LiteralPath $target).Path
    $expectedRoot = (Resolve-Path -LiteralPath $root).Path
    if (-not $resolvedTarget.StartsWith($expectedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing to remove a graph target directory outside the repository"
    }
    [System.IO.Directory]::Delete($resolvedTarget, $true)
}

Write-Output "repository graph Wasm assets rebuilt"
