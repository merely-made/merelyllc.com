<#
.SYNOPSIS
  Run one Graphshell web scenario in a real, headed Chrome and collect its receipt.

.DESCRIPTION
  The page drives itself (ports/graphshell/src/web_scenario.rs, the genet-probe
  verb loop over the browser DOM); this script only serves the page, opens a
  browser at it, and waits for the receipt the page POSTs back. No synthetic OS
  input, no DevTools, no extension: a receipt is reproducible by anyone with
  Chrome and the built bundle.

  The rule's one stated exception (Mark's ruling F68, dynamics grammar plan,
  2026-10-05): -Cdp, used only by p4_tree_canvas_reader_cdp.scn, opens this
  run's Chrome with a DevTools port and, once the scenario is done, reads
  Chrome's own computed accessibility tree (Accessibility.getFullAXTree)
  through cdp-ax-check.mjs, read-only and never sending input. Every other
  receipt reads the page's own graphshellSemanticTree() and needs none.

  Output, under -Out (default testing\mere\scenarios\graphshell-web\<name>):
    scenario.done   RESULT ok|fail, then the step log
    result.json     scenario result, page errors, the host receipt, semantic tree
    <name>.png      every `capture`

.EXAMPLE
  .\run-graphshell-web-scenario.ps1 h3_boot
  .\run-graphshell-web-scenario.ps1 h3_boot -Build

.NOTES
  -Build compiles with CARGO_PROFILE_DEV_DEBUG=0 and runs wasm-bindgen with
  --no-demangle. The web manifest optimizes the physics crates (rapier and
  seiche) even in this dev build, so settle frames run at speed. Recorded in the browser WebRTC carrier plan (C4b.0, finding
  3): until buckram erased its measure closure (genet 577e2471e97) the
  module carried fourteen copies of Taffy, rust-lld crashed on its DWARF, and
  bindgen's demangled name section ran to 2 GB. Full debug info now links at
  ~974 MB, just under the browser's 1 GiB module limit; debug=0 gives ~70 MB
  and is kept for size. --no-demangle stays because demangled v0 names are
  still several times the module.
#>
param(
    [Parameter(Mandatory = $true)][string]$Scenario,
    [string]$Out,
    [int]$Port = 8960,
    # This lane gives every headed run its own port and Chrome profile (2026-10-05).
    [string]$ProfileName = ".chrome-profile-viewer-cone",
    [int]$TimeoutSeconds = 900,
    # Extra page URL parameters, e.g. "signal=http://127.0.0.1:8788" to join a
    # running c4_webrtc_host over WebRTC instead of mounting the in-process
    # fixture. The fixture is started separately (see the plan, C4b.1).
    [string]$Query = "",
    # The surface: index.html (the component fills the page) or embed.html
    # (the component in a box inside someone else's page).
    [string]$Page = "tree.html",
    # Start the C4 host fixture (c4_webrtc_host) for this run and stop it
    # after, and pass its signaling URL to the page. Every run then starts
    # from a fresh board at revision 1, which the live-board receipts assert.
    [switch]$Fixture,
    [string]$FixtureBind = "192.168.4.36",
    [int]$SignalPort = 8990,
    [switch]$Build,
    # F68's one exception to "no DevTools": read Chrome's computed
    # accessibility tree after the scenario, read-only (see the description).
    [switch]$Cdp,
    [int]$CdpPort = 8995,
    [int]$CdpExpect = 11,
    # The page directory to serve and build: repos\mere's by default, or a
    # worktree's ports\graphshell\web when a branch is run before it lands.
    [string]$Web
)
$ErrorActionPreference = "Stop"
# The viewer-cone lane's copy (2026-10-06) of the grammar-g9 lane's fourth-round copy: ports
# 8960-8999, its own profiles, the cone worktree, and a fixture built from the cone branch.
# The grammar-g9 lane's fourth-round copy (2026-10-05): a port and profile per run, a live-owner port check, and -Cdp: its own port, Chrome profile and
# receipt folder, a sink sweep limited to its own port, and the lane's worktree.
$web = if ($Web) {
    Resolve-Path $Web
} else {
    Resolve-Path "C:\Users\mark_\Code\worktrees\mere-viewer-cone\ports\graphshell\web"
}
$sinkScript = "C:\Users\mark_\Code\testing\mere\scripts\graphshell-web-sink.py"
$name = [IO.Path]::GetFileNameWithoutExtension($Scenario)
if (-not $Out) { $Out = Join-Path $PSScriptRoot "scenarios\$name" }
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = Resolve-Path $Out
$scenarioPath = "scenarios/$name.scn"
if (-not (Test-Path (Join-Path $web $scenarioPath))) { throw "no scenario at $web\$scenarioPath" }

if ($Build) {
    Push-Location $web
    try {
        $env:CARGO_PROFILE_DEV_DEBUG = "0"
        cargo build --target wasm32-unknown-unknown
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
        $targetDir = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory
        $wasm = Join-Path $targetDir 'wasm32-unknown-unknown\debug\graphshell_web.wasm'
        wasm-bindgen --target web --no-demangle --out-dir pkg $wasm
        if ($LASTEXITCODE -ne 0) { throw "wasm-bindgen failed" }
    } finally { Pop-Location; Remove-Item Env:CARGO_PROFILE_DEV_DEBUG -ErrorAction SilentlyContinue }
}

Remove-Item (Join-Path $Out "scenario.done"), (Join-Path $Out "result.json"), (Join-Path $Out "cdp.done"), (Join-Path $Out "ax-tree.json") -ErrorAction SilentlyContinue
Get-ChildItem $Out -Filter *.png -ErrorAction SilentlyContinue | Remove-Item

$host_proc = $null
if ($Fixture) {
    $exe = Resolve-Path "C:\Users\mark_\Code\worktrees\mere-genet-imgdec\target\debug\c4_webrtc_host.exe"
    $fixtureLog = Join-Path $Out "fixture.log"
    $host_proc = Start-Process $exe -PassThru -WindowStyle Hidden -RedirectStandardOutput $fixtureLog -ArgumentList @(
        "--bind", $FixtureBind, "--advertise", $FixtureBind, "--signal-port", "$SignalPort")
    $deadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $deadline -and -not ((Test-Path $fixtureLog) -and (Select-String -Path $fixtureLog -Pattern "^READY" -Quiet))) { Start-Sleep -Milliseconds 250 }
    if (-not (Select-String -Path $fixtureLog -Pattern "^READY" -Quiet)) { throw "the fixture did not report READY (see $fixtureLog)" }
    $Query = if ($Query) { "$Query&signal=http://127.0.0.1:$SignalPort" } else { "signal=http://127.0.0.1:$SignalPort" }
}

# Refuse occupied ports; a matching sink may belong to another active lane.
# Only a listener a live process owns holds the port: a stopped sink's socket
# can linger owned by pid 0 for minutes (this lane's round two, the pre.4
# lane's finding), and the next sink binds over it.
$held = @(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
    Where-Object { $_.OwningProcess -ne 0 -and (Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue) })
if ($held.Count -gt 0) {
    throw "port $Port is already listening"
}
if ($Cdp -and @(Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue | Where-Object { $_.OwningProcess -ne 0 }).Count -gt 0) {
    throw "CDP port $CdpPort is already listening"
}

$sink = Start-Process python -PassThru -WindowStyle Hidden -ArgumentList @(
    $sinkScript, "--web", "$web", "--out", "$Out", "--port", "$Port")
try {
    $deadline = (Get-Date).AddSeconds(10)
    do {
        try { $ok = (Invoke-WebRequest "http://127.0.0.1:$Port/index.html" -UseBasicParsing -TimeoutSec 2).StatusCode -eq 200 } catch { $ok = $false }
        if (-not $ok) { Start-Sleep -Milliseconds 250 }
    } until ($ok -or (Get-Date) -gt $deadline)
    if (-not $ok) { throw "the sink did not come up on port $Port" }

    $url = "http://127.0.0.1:$Port/$Page`?scenario=$scenarioPath&sink=http://127.0.0.1:$Port/scenario-receipt"
    if ($Query) { $url += "&$Query" }
    $chrome = "C:\Program Files\Google\Chrome\Application\chrome.exe"
    # A profile of its own, so this is a separate Chrome instance: the process
    # handle is the window, and stopping it closes exactly what was opened
    # (a page holds a ~70 MB module and a WebGPU device; left open, they add
    # up). The profile persists across runs so IndexedDB reopens as in a real
    # visit. The user's own Chrome is never touched.
    $profile = if ([IO.Path]::IsPathRooted($ProfileName)) { $ProfileName } else { Join-Path $PSScriptRoot $ProfileName }
    New-Item -ItemType Directory -Force $profile | Out-Null
    $profile = (Resolve-Path $profile).Path
    # WebRtcHideLocalIpsWithMdns is Chrome's default and replaces host
    # candidates with .local names the native answerer cannot resolve (the C1
    # receipt found this on the user profile and disabled the flag by hand);
    # a fresh profile needs it disabled on the command line.
    # This lane's runs found the window occluded (visibilityState hidden, no
    # frames); the two background flags keep an occluded window rendering.
    # 2026-10-02.
    $arguments = @(
        "--user-data-dir=$profile", "--no-first-run", "--no-default-browser-check",
        "--disable-features=WebRtcHideLocalIpsWithMdns,CalculateNativeWinOcclusion",
        "--disable-backgrounding-occluded-windows", "--disable-renderer-backgrounding",
        "--new-window", "--window-size=1400,900")
    if ($Cdp) { $arguments += "--remote-debugging-port=$CdpPort" }
    $arguments += $url
    $browser = Start-Process $chrome -PassThru -WindowStyle Hidden -ArgumentList $arguments
    try {
        $done = Join-Path $Out "scenario.done"
        $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
        while (-not (Test-Path $done) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
        if (-not (Test-Path $done)) { throw "no scenario.done after $TimeoutSeconds s (is the page erroring? open the URL by hand to look)" }
        Get-Content $done
        if ($Cdp) {
            # Read-only: the computed tree of the finished page, then nothing.
            $check = Join-Path $PSScriptRoot "cdp-ax-check.mjs"
            $report = Join-Path $Out "ax-tree.json"
            $cdpLine = & node $check $CdpPort $report $CdpExpect 2>&1 | Out-String
            Set-Content (Join-Path $Out "cdp.done") "EXIT $LASTEXITCODE`n$($cdpLine.Trim())"
            Write-Output "CDP $($cdpLine.Trim())"
        }
        Get-ChildItem $Out | Select-Object Name, Length | Format-Table -AutoSize
    } finally {
        # Stop every process of this profile's instance, not only the launcher
        # (Chrome forks renderers and a GPU process under the same user-data-dir).
        # Matched on the profile directory's name rather than its full path:
        # Chrome rewrites the path in its children's command lines.
        Get-CimInstance Win32_Process -Filter "Name = 'chrome.exe'" |
            Where-Object { $_.CommandLine -like "*$ProfileName*" } |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
        if ($browser -and -not $browser.HasExited) { Stop-Process -Id $browser.Id -Force -ErrorAction SilentlyContinue }
        Start-Sleep -Milliseconds 500
        $left = @(Get-CimInstance Win32_Process -Filter "Name = 'chrome.exe'" | Where-Object { $_.CommandLine -like "*$ProfileName*" }).Count
        if ($left -gt 0) { Write-Warning "$left chrome process(es) of the receipt profile survived" } else { Write-Output "browser closed" }
    }
} finally {
    if ($sink -and -not $sink.HasExited) { Stop-Process -Id $sink.Id -Force }
    if ($host_proc -and -not $host_proc.HasExited) { Stop-Process -Id $host_proc.Id -Force; Write-Output "fixture stopped" }
}
