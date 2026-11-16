"""Reports contract enum fields that allow a value the database CHECK on the same-named column does not (drift, e.g. 'manual' vs 'approver'). Usage: python check_enum_drift.py (needs pyyaml). Exit 1 on findings.
"""
import pathlib
import re
import sys

import yaml

DOCS = pathlib.Path(__file__).resolve().parent.parent
api, db = [], []


def walk(node, file, path):
    if isinstance(node, dict):
        if isinstance(node.get("enum"), list):
            vals = frozenset(str(v) for v in node["enum"] if v is not None)
            if len(vals) > 1:
                api.append((vals, file, path))
        for key, value in node.items():
            walk(value, file, f"{path}/{key}")
    elif isinstance(node, list):
        for value in node:
            walk(value, file, path)


for f in sorted((DOCS / "api" / "openapi").glob("*.yaml")):
    walk(yaml.safe_load(f.read_text(encoding="utf-8")), f.name, "")
for f in sorted((DOCS / "database" / "migrations").glob("*.up.sql")):
    for m in re.finditer(r"(\w+)\s+IN\s*\(\s*('[^)]*')\s*\)", f.read_text(encoding="utf-8")):
        vals = frozenset(re.findall(r"'([^']*)'", m.group(2)))
        if len(vals) > 1:
            db.append((vals, f.name, m.group(1)))

# Real drift: a contract field whose values are not all allowed by any CHECK on a column of the same name.
# (Contract enums may be subsets, e.g. input fields that allow fewer values; those are fine.)
by_col = {}
for d, dfile, dcol in db:
    by_col.setdefault(dcol, []).append((d, dfile))
# Reviewed 2026-11-16: same field name, different concept (not drift). Key: (contract file, field name, first API value).
REVIEWED = {
    ("attendance.yaml", "kind"): "bulk waiver kind late/early, not org.list_entry.kind",
    ("organisation.yaml", "kind"): "financial-year kind (OR-01), not org.list_entry.kind",
    ("common.yaml", "level"): "approval level 1/2, not policy assignment level",
    ("identity.yaml", "level"): "approver rule level 1/2, not policy assignment level",
    ("leave.yaml", "level"): "approval level 1/2, not policy assignment level",
    ("common.yaml", "state"): "Job state; jobs are not a database table",
    ("leave.yaml", "decision"): "bulk command input approve/reject, not punch flag decision",
    ("scheduling.yaml", "source"): "response-only 'none' for a day with no schedule; not stored",
}
findings = set()
for a, afile, apath in api:
    parts = [p for p in apath.split("/") if p]
    name = parts[-1] if parts else ""
    if name not in by_col or (afile, name) in REVIEWED:
        continue
    if not any(a <= d for d, _ in by_col[name]):
        best = max(by_col[name], key=lambda x: len(a & x[0]))
        findings.add((afile, "/".join(parts[-3:]), best[1], name, tuple(sorted(a - best[0])), tuple(sorted(best[0] - a))))

for f in sorted(findings):
    print(f"{f[0]} {f[1]}  vs  {f[2]} {f[3]}:  only in API {list(f[4])}  only in DB {list(f[5])}")
print(f"{len(findings)} drifted enum fields")
sys.exit(1 if findings else 0)
