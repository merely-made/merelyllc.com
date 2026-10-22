#!/usr/bin/env python3
"""Fetch the spec registry's results files into content/spec-results/.

    cargo run --locked --bin authority -- spec-sources | python3 scripts/refresh-spec-results.py

Each stdin line names a repository, its GitHub slug, a branch and a path. The
file is fetched at the latest commit on that branch that touched it, and
sources.json records that commit and its date, so every count the site shows
names where and when it came from. GITHUB_TOKEN or GH_TOKEN is used when set.
"""

import datetime
import json
import os
import pathlib
import shutil
import sys
import tempfile
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
RESULTS = ROOT / "content" / "spec-results"


def get(url):
    headers = {"User-Agent": "merelyllc.com spec-results refresh"}
    if "api.github.com" in url:
        headers["Accept"] = "application/vnd.github+json"
        token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
        if token:
            headers["Authorization"] = "Bearer " + token
    with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=30) as response:
        return response.read()


def main():
    sources = [json.loads(line) for line in sys.stdin if line.strip()]
    staging = pathlib.Path(tempfile.mkdtemp(prefix="spec-results-"))
    fetched = []
    for source in sources:
        query = urllib.parse.urlencode({"sha": source["ref"], "path": source["path"], "per_page": 1})
        commits = json.loads(get(f"https://api.github.com/repos/{source['slug']}/commits?{query}"))
        if not commits:
            sys.exit(f"{source['slug']}@{source['ref']} has no commit touching {source['path']}")
        commit = commits[0]["sha"]
        body = get(f"https://raw.githubusercontent.com/{source['slug']}/{commit}/{source['path']}")
        target = staging / source["repo"] / source["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)
        fetched.append(dict(source, commit=commit, committed_on=commits[0]["commit"]["committer"]["date"][:10]))
        print(f"{source['slug']} {source['path']} at {commit[:10]}", file=sys.stderr)
    manifest = {
        "fetched_on": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d"),
        "source": sorted(fetched, key=lambda s: (s["repo"], s["slug"], s["ref"], s["path"])),
    }
    (staging / "sources.json").write_text(json.dumps(manifest, indent=1) + "\n")
    shutil.rmtree(RESULTS, ignore_errors=True)
    shutil.copytree(staging, RESULTS)
    shutil.rmtree(staging)


if __name__ == "__main__":
    main()
