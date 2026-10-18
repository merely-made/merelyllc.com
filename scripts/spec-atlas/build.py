"""Build the Merely Spec Atlas page from `authority specs-json` and the catalog snapshot.

    cargo run --bin authority -- specs-json > /tmp/specs.json
    python3 scripts/spec-atlas/build.py /tmp/specs.json \
        content/spec-catalogs/browser-specs.json scripts/spec-atlas/template.html atlas.html
"""
import json, re, sys

REPORT, SNAPSHOT, TEMPLATE, OUT = sys.argv[1:5]
doc = json.load(open(REPORT))
cov, reg = doc["coverage"], doc["registry"]
catalog = json.load(open(SNAPSHOT))["spec"]
specs = {s["id"]: s for s in reg["spec"]}

GROUPS = [("Engines", ["genet", "vano", "boa-fork", "piccolo-fork"]),
          ("Rendering & embedding", ["netrender", "wgpu-weld", "wgpu-scry", "wgpu-graft"]),
          ("Workbench", ["mere", "dramatis"]),
          ("Apps", ["turnstone", "knot-editor", "woodshed", "isocosm", "cleromancy"]),
          ("Spec libraries", ["smolweb", "turquet", "wavicle", "emblem", "pipit", "mora"]),
          ("Transport", ["retinue"]), ("Site", ["mer3ly"]), ("Family-wide verdicts", ["family"])]
TIER = {**{r: "spec" for r in ["genet", "vano", "boa-fork", "piccolo-fork", "mere", "dramatis", "knot-editor", "woodshed",
                                "cleromancy", "smolweb", "turquet", "wavicle", "emblem", "retinue", "mer3ly"]},
        **{r: "dep" for r in ["netrender", "wgpu-weld", "wgpu-scry", "wgpu-graft", "turnstone"]},
        **{r: "thin" for r in ["isocosm", "pipit", "mora"]}}
LABEL = {"retinue": "murm", "boa-fork": "bole", "piccolo-fork": "didgeridoo", "mer3ly": "merelyllc.com"}
REL = {"implements": "impl", "via-dep": "dep", "should": "should", "adjacent": "adj"}
RANK = {"full": 0, "partial": 1, "stub": 2, "planned": 3, "none": 4}

claims_by_repo = {}
for c in reg["claim"]:
    claims_by_repo.setdefault(c["repo"], []).append(c)
listed = {r for _, rs in GROUPS for r in rs}
missing = sorted(set(claims_by_repo) - listed)
if missing:
    raise SystemExit(f"repos without an atlas group: {missing}")
repos = []
for group, ids in GROUPS:
    for rid in ids:
        rows = []
        for c in sorted(claims_by_repo.get(rid, []), key=lambda c: (RANK[c["depth"]], specs[c["spec"]]["name"].lower())):
            s = specs[c["spec"]]
            name = s["name"] + (f" ({c['crate']})" if c.get("crate") else "")
            note = (c.get("evidence") or "").strip()
            rows.append(dict(n=name, b=s["body"], r=REL[c["relation"]], d=c["depth"], v=c["verdict"],
                             x=note if len(note) <= 170 else note[:167].rstrip() + "…"))
        if rows:
            repos.append(dict(g=group, id=rid, label=LABEL.get(rid, rid), tier=TIER.get(rid, ""), specs=rows))

def fam(sid, title, org):
    s, t = sid.lower(), title.lower()
    if org == "Khronos Group" or s.startswith(("webgpu", "wgsl", "webgl")): return "GPU & graphics"
    if s.startswith(("css", "fill-stroke", "compositing", "filter-effects", "motion", "scroll-animations", "pointer-animations",
                     "animation-triggers", "web-animations", "svg", "mathml", "fxtf")) or "css" in s: return "CSS, SVG & rendering"
    if org == "IETF": return "IETF (HTTP, cookies, structured fields…)"
    if org == "Ecma International": return "ECMAScript & Intl"
    if org in ("Alliance for Open Media", "ISO/IEC", "CompuServe Incorporated"): return "Media formats"
    if re.search(r"csp|security|permission|fetch-metadata|mixed|referrer|integrity|trusted|cors|cookie|isolation|credential|webauthn|fido|payment|fedcm|private|privacy|storage-access|attribution|topics|fenced|shared-storage|turtledove|protected", s): return "Security, privacy & identity"
    if re.search(r"audio|video|media|webcodecs|webrtc|mediacapture|image-capture|encrypted|picture-in|speech|midi|remote-playback|mse|eme|webvtt|ttml|imsc|presentation|audio-output|screen-capture|autoplay", s): return "Media & real-time"
    if re.search(r"timing|performance|hr-time|reporting|observer|long-anim|event-timing|largest|paint-timing|layout-instability|compute-pressure|battery|js-self", s): return "Performance & observation"
    if re.search(r"sensor|accelerometer|gyroscope|magnetometer|orientation|geolocation|ambient|proximity|vibration|gamepad|webxr|webhid|webusb|serial|bluetooth|nfc|wake-lock|idle|contact|device|screen-|keyboard-lock|pointerlock|xr", s): return "Devices & sensors"
    if re.search(r"aria|accname|core-aam|html-aam|svg-aam|dpub|wai|a11y|accessib", s): return "Accessibility"
    if org == "WHATWG": return "WHATWG core"
    return "Other web APIs"

genet = {c["spec"]: c for c in reg["claim"] if c["repo"] == "genet" and not c.get("crate")}
rows = []
for s in catalog:
    m = (genet.get(s["shortname"]) or {}).get("measured") or {}
    rows.append(dict(s=s["shortname"], t=s["title"], o=s["organization"], f=fam(s["shortname"], s["title"], s["organization"]),
                     st=s["standing"], u=s["url"], m="yes" if m.get("wpt_total") else ("out-of-scope" if s["test_paths"] else "no-wpt"),
                     pf=m.get("files_all_pass", 0), fi=m.get("files", 0), sp=m.get("wpt_pass", 0), stt=m.get("wpt_total", 0),
                     ih=m.get("idl_named", 0), it=m.get("idl_total", 0), ch=m.get("css_props_named"), ct=m.get("css_props_total")))

t = open(TEMPLATE).read()
t = t.replace("Count of key specs per repo by publishing body, from the per-repo tables in §3. Darker cells hold more specs.",
              f"Claims per repo by publishing body, from the registry as reported on {cov['on']}. Darker cells hold more claims.")
t = t.replace("All 615 specs in the w3c/browser-specs catalog", f"All {len(catalog)} specs in the w3c/browser-specs {cov['catalog_version']} catalog")
t = t.replace("<li>Repo maps: read-only surveys of READMEs, manifests, design docs and source greps across 25 repos: the 23 code repos in merely-made, plus hocket and netfetcher. Nothing was built or run. Depth reflects each repo's own receipts and docs.</li>",
              f"<li>Built from <code>authority specs-json</code> on {cov['on']}: the registry in <code>merelyllc.com/content/specs.toml</code> ({cov['specs']} specs, {cov['claims']} claims) and the committed browser-specs {cov['catalog_version']} snapshot. The registry was seeded from read-only surveys of every repo, genet's ledger, mere's standards survey and its W3C review.</li>")
t = t.replace("__NCAT__", str(len(catalog)))
for a, b in [("__REPOS__", json.dumps(repos, ensure_ascii=False, separators=(",", ":"))),
             ("__ROWS__", json.dumps(rows, ensure_ascii=False, separators=(",", ":")))]:
    assert "</script" not in b
    t = t.replace(a, b)
open(OUT, "w").write(t)
print(len(repos), "repos;", sum(len(r["specs"]) for r in repos), "claims;", len(rows), "catalog rows")
