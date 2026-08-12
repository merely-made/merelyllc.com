[CmdletBinding()]
param(
    [string]$OutputPath = "content/github-metadata.json"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$mer3lyRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if ([IO.Path]::IsPathRooted($OutputPath) -or $OutputPath -match "(^|[\\/])\.\.([\\/]|$)") {
    throw "OutputPath must be relative to the Mer3ly repository"
}
$outputFull = [IO.Path]::GetFullPath((Join-Path $mer3lyRoot $OutputPath))
$contentRoot = [IO.Path]::GetFullPath((Join-Path $mer3lyRoot "content"))
$contentPrefix = $contentRoot.TrimEnd(
    [IO.Path]::DirectorySeparatorChar,
    [IO.Path]::AltDirectorySeparatorChar
) + [IO.Path]::DirectorySeparatorChar
if (-not $outputFull.StartsWith($contentPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputPath must remain below content"
}

& gh auth status *> $null
if ($LASTEXITCODE -ne 0) {
    throw "GitHub CLI is not authenticated; existing metadata cache retained"
}

$authorityRaw = & cargo run --quiet --manifest-path (Join-Path $mer3lyRoot "Cargo.toml") `
    --bin authority -- public-repositories 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "authority validation failed; existing metadata cache retained"
}
$authority = $authorityRaw | ConvertFrom-Json

$organization = "merely-made"
$editorialIdBySlug = @{}
foreach ($repository in @($authority.repository | Where-Object { $_.public })) {
    $editorialIdBySlug[[string]$repository.github_slug] = [string]$repository.id
}

$organizationRaw = & gh repo list $organization --limit 200 --json `
    "name,nameWithOwner,description,homepageUrl,isArchived,isFork,licenseInfo,primaryLanguage,pushedAt,repositoryTopics,stargazerCount,updatedAt,visibility" `
    2>$null
if ($LASTEXITCODE -ne 0) {
    throw "GitHub organization refresh failed for $organization; existing cache retained"
}
$organizationRepositories = @($organizationRaw | ConvertFrom-Json) |
    Where-Object { $_.visibility -eq "PUBLIC" } |
    Sort-Object name
if ($organizationRepositories.Count -eq 0) {
    throw "GitHub organization refresh returned no public repositories; existing cache retained"
}

$records = [System.Collections.Generic.List[object]]::new()
foreach ($metadata in $organizationRepositories) {
    $slug = [string]$metadata.nameWithOwner
    $id = $editorialIdBySlug[$slug]
    if ([string]::IsNullOrWhiteSpace($id)) {
        $id = ([string]$metadata.name).ToLowerInvariant() -replace "[^a-z0-9]+", "-"
        $id = $id.Trim("-")
    }
    $topics = @(
        $metadata.repositoryTopics |
            ForEach-Object { $_.name } |
            Sort-Object -Unique
    )
    $records.Add([ordered]@{
        id = $id
        github_slug = $slug
        name = [string]$metadata.name
        description = if ($null -eq $metadata.description) { "" } else { [string]$metadata.description }
        homepage = if (
            $null -ne $metadata.homepageUrl -and
            ([string]$metadata.homepageUrl).StartsWith("https://", [StringComparison]::OrdinalIgnoreCase)
        ) { [string]$metadata.homepageUrl } else { $null }
        license = if ($null -eq $metadata.licenseInfo) { $null } else { [string]$metadata.licenseInfo.key }
        updated_at = $metadata.updatedAt
        pushed_at = $metadata.pushedAt
        primary_language = if ($null -eq $metadata.primaryLanguage) {
            $null
        } else {
            [string]$metadata.primaryLanguage.name
        }
        stargazer_count = [uint64]$metadata.stargazerCount
        archived = [bool]$metadata.isArchived
        fork = [bool]$metadata.isFork
        topics = $topics
    })
}

$liveSlugs = @{}
foreach ($record in $records) {
    $liveSlugs[[string]$record.github_slug] = $true
}
$eventCandidates = [System.Collections.Generic.List[object]]::new()
$eventsRaw = & gh api "orgs/$organization/events?per_page=100" 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "GitHub public event refresh failed for $organization; existing cache retained"
}
foreach ($event in @($eventsRaw | ConvertFrom-Json)) {
    $repository = [string]$event.repo.name
    if (-not [bool]$event.public -or -not $liveSlugs.ContainsKey($repository)) {
        continue
    }
    $eventCandidates.Add([ordered]@{
        id = [string]$event.id
        kind = [string]$event.type
        repository = $repository
        created_at = ([DateTime]$event.created_at).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    })
}
$events = @(
    $eventCandidates |
        Sort-Object -Property @{
            Expression = { [string]$_["created_at"] }
            Descending = $true
        } |
        Select-Object -First 40
)

$cache = [ordered]@{
    schema = "mer3ly.github-organization/v2"
    organization = $organization
    generated_at_utc = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")
    repository = @($records)
    event = @($events)
}
$json = $cache | ConvertTo-Json -Depth 8
if ($json -match "(?i)([A-Z]:\\\\|file://|/Users/|\\\\Users\\\\|viewerPermission|sshUrl|token)") {
    throw "public metadata sanitization failed; existing cache retained"
}

$temporaryDirectory = Join-Path $mer3lyRoot ".tmp"
New-Item -ItemType Directory -Force -Path $temporaryDirectory | Out-Null
$temporaryPath = Join-Path $temporaryDirectory "github-metadata.$PID.json"

try {
    Set-Content -LiteralPath $temporaryPath -Value $json -Encoding utf8
    $validation = & cargo run --quiet --manifest-path (Join-Path $mer3lyRoot "Cargo.toml") `
        --bin authority -- validate-metadata $mer3lyRoot $temporaryPath 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Rust validation rejected refreshed metadata: $($validation -join '; ')"
    }
    Move-Item -LiteralPath $temporaryPath -Destination $outputFull -Force
} catch {
    if (Test-Path -LiteralPath $temporaryPath) {
        [IO.File]::Delete($temporaryPath)
    }
    throw "metadata refresh failed; existing cache retained: $($_.Exception.Message)"
}

Write-Output "wrote $OutputPath"
Write-Output "metadata: $($records.Count) public repositories and $($events.Count) public events refreshed atomically"
