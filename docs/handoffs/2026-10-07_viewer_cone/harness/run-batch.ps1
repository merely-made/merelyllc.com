# The viewer-cone lane's headed batch (2026-10-06), adapted from the grammar-g9
# lane's run-batch-r4.ps1. One receipt at a time, each with its own port and
# Chrome profile unless it shares one on purpose (the saved-graph pair), and
# each failing on any page error. A control counts only if it fails for its
# planted reason. $Set picks the list: "default" is the union of the conatus
# session's three rounds plus the product and remote receipts; "viewer" is the
# tree subset that must survive the cone, plus the product and remote receipts
# that must be refused cleanly (RESULT fail, no page error). Writes -Log,
# refusing to overwrite.
param(
    [Parameter(Mandatory = $true)][string]$Log,
    [Parameter(Mandatory = $true)][ValidateSet("default", "viewer", "rerun")][string]$Set,
    [string]$Suffix = "r1",
    [int]$BasePort = 8960,
    # A comma-separated subset of scenario names to run; empty runs the whole set.
    [string]$Only = "",
    # The page directory to serve; the runner's default is the cone worktree's.
    [string]$Web = ""
)
$G = $PSScriptRoot
if (Test-Path $Log) { throw "refusing to overwrite $Log" }
Set-Content $Log "set $Set, suffix $Suffix, ports from $BasePort, one at a time"
$script:failed = 0
$script:index = 0
# $expect: "pass"; "fail:<text in the failing log>"; "gate:" (the receipt gate
# fails a planted page error); "refused" (the viewer refuses: RESULT fail, no
# page error); "cdp-fail:<text in cdp.done>".
function One($name, $page, $query, [string]$expect = "pass", [bool]$cdp = $false, [bool]$fixture = $false, [string]$profile = "", [int]$timeout = 600, [int]$fixedPort = 0) {
    if ($Only -and -not (($Only -split ",") -contains $name)) { return }
    # IndexedDB is per origin, port included: a pair that shares a store shares a port.
    $port = if ($fixedPort) { $fixedPort } else { $BasePort + ($script:index % 25) }
    $script:index++
    if (-not $profile) { $profile = ".chrome-profile-viewer-cone-$Set-$Suffix-p$($script:index)" }
    $tag = if ($query) { "-" + (($query -replace '[^A-Za-z0-9]+', '-').Trim('-')) } else { "" }
    $out = "$G\scenarios-$Set\$name-$Suffix$tag"
    $p = @{ Scenario = $name; Out = $out; TimeoutSeconds = $timeout; Page = $page; Port = $port; ProfileName = $profile }
    if ($query) { $p.Query = $query }
    if ($Web) { $p.Web = $Web }
    if ($cdp) { $p.Cdp = $true; $p.CdpPort = 8995; $p.CdpExpect = 11 }
    if ($fixture) { $p.Fixture = $true; $p.SignalPort = 8990 }
    $start = Get-Date
    try {
        $r = & "$G\run-scenario.ps1" @p 2>&1 | Out-String
        # Case-sensitive: the runner also lists a file named result.json, which -like "RESULT*" matches.
        $line = ($r -split "`n" | Where-Object { $_ -clike "RESULT *" } | Select-Object -First 1)
        if (-not $line) { $line = "RESULT none" }
    } catch { $line = "RESULT error: $($_.Exception.Message)" }
    $line = $line.Trim()
    $s = [int]((Get-Date) - $start).TotalSeconds
    $pageErrors = -1
    $gateFailures = -1
    $json = Join-Path $out "result.json"
    if (Test-Path $json) {
        $result = Get-Content $json -Raw | ConvertFrom-Json
        $pageErrors = @($result.scenario.errors).Count
        $gateFailures = @($result.scenario.gate_failures).Count
    }
    $doneText = if (Test-Path (Join-Path $out "scenario.done")) { Get-Content (Join-Path $out "scenario.done") -Raw } else { "" }
    $cdpText = if (Test-Path (Join-Path $out "cdp.done")) { (Get-Content (Join-Path $out "cdp.done") -Raw).Trim() -replace "`r?`n", " " } else { "" }
    $clean = $pageErrors -eq 0 -and $gateFailures -eq 0
    if ($expect -eq "pass") {
        $ok = $line -eq "RESULT ok" -and $clean -and (-not $cdp -or $cdpText -like "EXIT 0*")
        $verdict = if ($ok) { "PASS" } else { "FAIL" }
    } elseif ($expect -eq "refused") {
        $ok = $line -eq "RESULT fail" -and $clean
        $verdict = if ($ok) { "refused cleanly, as it should" } else { "NOT REFUSED CLEANLY" }
    } elseif ($expect -like "fail:*") {
        $why = $expect.Substring(5)
        $ok = $line -eq "RESULT fail" -and $clean -and $doneText.Contains($why)
        $verdict = if ($ok) { "control failed on '$why', as it should" } else { "CONTROL NOT FAILED AS PLANTED" }
    } elseif ($expect -like "gate:*") {
        $ok = $line -eq "RESULT fail" -and $gateFailures -ge 1 -and $doneText.Contains("receipt gate")
        $verdict = if ($ok) { "control failed by the receipt gate, as it should" } else { "CONTROL NOT FAILED BY THE GATE" }
    } else {
        $why = $expect.Substring(9)
        $ok = $line -eq "RESULT ok" -and $clean -and $cdpText -like "EXIT 1*" -and $cdpText.Contains($why)
        $verdict = if ($ok) { "control failed in Chrome's tree on '$why', as it should" } else { "CDP CONTROL NOT FAILED AS PLANTED" }
    }
    if (-not $ok) { $script:failed++ }
    Add-Content $Log "$verdict $(Split-Path $out -Leaf): $line; page errors $pageErrors, gate failures $gateFailures$(if ($cdpText) { "; $cdpText" }) ($s s) page=$page query='$query'"
    $deadline = (Get-Date).AddSeconds(20)
    do {
        $left = @(Get-CimInstance Win32_Process -Filter "Name = 'chrome.exe'" | Where-Object { $_.CommandLine -like "*$profile*" })
        $left | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
        if ($left.Count -gt 0) { Start-Sleep -Milliseconds 500 }
    } until ($left.Count -eq 0 -or (Get-Date) -gt $deadline)
    Start-Sleep -Seconds 2
}

if ($Set -eq "rerun") {
    # The first default round's three misses, rerun calm: the fast-speed
    # receipt with main's own query, its control, and the saved-graph pair.
    One "p6_tree_speed_fast" "tree.html" "nodes=300&seed=7&links=none&gpu=off&physics_speed=max"
    One "p6_tree_speed_fast_control" "tree.html" "nodes=24&seed=7&links=none&gpu=off&physics_speed=50"
    One "p6_tree_speed_slow" "tree.html" "physics_speed=0.2"
    One "p4_tree_canvas_reader_cdp" "tree.html" "plant_a11y=missing_action" "cdp-fail:Pin missing" $true
    One "p4_tree_saved_edit" "tree.html" "app=local" "pass" $false $false ".chrome-profile-viewer-cone-rerun-$Suffix-saved" 600 8989
    One "p4_tree_saved_reopen" "tree.html" "app=local" "pass" $false $false ".chrome-profile-viewer-cone-rerun-$Suffix-saved" 600 8989
    Add-Content $Log "DONE: $($script:index) receipts, $($script:failed) not as expected"
    return
}
$laws = "springs", "charge", "stress", "energy", "orbit", "kinds", "flock", "sync", "flow", "anneal", "still"
$speed300 = "nodes=300&seed=7&links=none&gpu=off"
$settle = "nodes=2000&seed=7&links=none"

# What both builds run: the tree page's physics, speed, reader, roles and keys.
foreach ($law in $laws + "profiles", "add", "drag", "density", "density_control", "framing_control") {
    One "p4_tree_physics_$law" "tree.html" "" "pass" $false $false "" 420
}
One "p6_tree_speed_slow" "tree.html" "physics_speed=0.2"
# Main's receipt asserts physics-speed max (the absolute >= 1x version).
One "p6_tree_speed_fast" "tree.html" "$speed300&physics_speed=max"
One "p6_tree_speed_fast_planted" "tree.html" "$speed300&physics_speed=max&physics_plant_stall_ms=10"
One "p6_tree_speed_fast_control" "tree.html" "nodes=24&seed=7&links=none&gpu=off&physics_speed=50"
One "p6_tree_speed_select" "tree.html" $speed300
One "p5_tree_cpu_settle_2000" "tree.html" "$settle&gpu=off" "pass" $false $false "" 1500
One "p4_tree_canvas_reader" "tree.html" ""
One "p4_tree_canvas_reader" "tree.html" "plant_a11y=missing_item" "fail:reader-items"
One "p4_tree_canvas_reader" "tree.html" "plant_a11y=missing_action" "fail:reader-buttons"
One "p4_tree_canvas_reader" "tree.html" "plant_a11y=dead_action" "fail:pinned"
One "p4_tree_canvas_reader_cdp" "tree.html" "" "pass" $true
One "p4_tree_canvas_reader_cdp" "tree.html" "plant_a11y=missing_action" "cdp-fail:Pin missing" $true
One "p4_tree_arrangement_roles" "tree.html" ""
One "p4_tree_role_controls" "tree.html" ""
One "p4_tree_physics_keys" "tree.html" ""
One "p4_tree_physics_framing_control" "tree.html" "plant_page_error=throw" "gate:"
One "p4_tree_physics_orbit" "tree.html" "plant_page_error=panic" "gate:"
if ($Set -eq "default") {
    One "p4_tree_remote_absent" "tree.html" ""
    # The GPU's lanes (pre.4's set), the main page, the product modes and the
    # remote board.
    foreach ($law in $laws) { One "p4_tree_physics_$law" "tree.html" "gpu_threshold=0" "pass" $false $false "" 420 }
    One "p5_tree_gpu_settle_2000" "tree.html" $settle "pass" $false $false "" 1500
    One "physics_speed_select" "index.html" $speed300
    One "physics_drag" "index.html" ""
    One "role_controls" "index.html" ""
    One "physics_orbit" "index.html" "plant_page_error=panic" "gate:"
    One "practice_encoded_axes" "practice.html" ""
    One "practice_workspace" "practice.html" ""
    One "p4_tree_item_role" "tree.html" "app=local"
    One "p4_tree_saved_edit" "tree.html" "app=local" "pass" $false $false ".chrome-profile-viewer-cone-default-$Suffix-saved" 600 8989
    One "p4_tree_saved_reopen" "tree.html" "app=local" "pass" $false $false ".chrome-profile-viewer-cone-default-$Suffix-saved" 600 8989
    One "p4_tree_remote_draft" "tree.html" "" "pass" $false $true
    One "p4_tree_physics_remote_board" "tree.html" "" "pass" $false $true
    One "physics_remote_board" "index.html" "" "pass" $false $true
    One "p4_tree_c4b1_live_board" "tree.html" "" "pass" $false $true
    One "p4_tree_c4b3_reconnect" "tree.html" "" "pass" $false $true
    One "c4b1_live_board" "index.html" "" "pass" $false $true
    One "c4b3_reconnect" "index.html" "" "pass" $false $true
} else {
    # What the viewer must refuse cleanly: the saved-graph route and a link.
    One "p4_tree_item_role" "tree.html" "app=local" "refused"
    One "p4_tree_remote_absent" "tree.html" "" "refused"
    One "p4_tree_saved_edit" "tree.html" "app=local" "refused"
    One "p4_tree_remote_draft" "tree.html" "" "refused" $false $true
    One "p4_tree_c4b1_live_board" "tree.html" "" "refused" $false $true
}
Add-Content $Log "DONE: $($script:index) receipts, $($script:failed) not as expected"
