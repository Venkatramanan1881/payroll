"""Contract checks for docs/api/openapi (run in CI).

1. Each OpenAPI file is valid OpenAPI 3.1, with cross-file $refs resolved.
2. Every operation declares x-permission, x-module, x-scope, x-audit, x-events, x-idempotent (API_CONVENTIONS §2).
3. Commands that declare x-idempotent: key take the Idempotency-Key header.
4. No request body schema has an org_id property (TB-01).
5. Every PERMISSION_MATRIX operation code maps to at least one operation, or is listed in EXEMPT with a reason.
6. No path or query parameter is named org_id (TB-01).
7. Every x-events value is in the domain event list in docs/architecture/SERVICE_BOUNDARIES.md §4.

Usage: python check_contracts.py   (needs openapi-spec-validator, pyyaml)
"""
import pathlib
import re
import sys

import yaml
from openapi_spec_validator import validate
from openapi_spec_validator.readers import read_from_filename

HERE = pathlib.Path(__file__).parent
SPEC_DIR = HERE / "openapi"
MATRIX = HERE.parent / "domain" / "PERMISSION_MATRIX.md"
BOUNDARIES = HERE.parent / "architecture" / "SERVICE_BOUNDARIES.md"
_events_section = BOUNDARIES.read_text(encoding="utf-8").split("### Domain events")[1].split("\n## ")[0]
KNOWN_EVENTS = set(re.findall(r"`([A-Z][A-Za-z]+)`", _events_section))
REQUIRED_X = ["x-permission", "x-module", "x-scope", "x-audit", "x-events", "x-idempotent"]
METHODS = {"get", "post", "put", "patch", "delete"}

# Matrix codes with no HTTP operation of their own, with the reason. Filled from the context files' x-exempt-permissions.
EXEMPT: dict[str, str] = {}

failures: list[str] = []
ops_by_code: dict[str, list[str]] = {}
op_count = 0


def deref(spec, node, base):
    """Resolve a local or common.yaml $ref one level (enough for parameters)."""
    if isinstance(node, dict) and "$ref" in node:
        ref = node["$ref"]
        file, _, pointer = ref.partition("#")
        doc = spec if not file else yaml.safe_load((base / file).read_text(encoding="utf-8"))
        for part in pointer.strip("/").split("/"):
            doc = doc[part]
        return doc
    return node


def has_org_id(schema, spec, base, depth=0):
    if depth > 6 or not isinstance(schema, dict):
        return False
    schema = deref(spec, schema, base)
    if "org_id" in (schema.get("properties") or {}):
        return True
    return any(has_org_id(s, spec, base, depth + 1) for key in ("allOf", "oneOf", "anyOf") for s in schema.get(key, []))


files = sorted(p for p in SPEC_DIR.glob("*.yaml") if p.name != "common.yaml")
for path in [SPEC_DIR / "common.yaml", *files]:
    spec_dict, base_uri = read_from_filename(str(path))
    try:
        validate(spec_dict, base_uri=base_uri)
    except Exception as exc:  # noqa: BLE001 - report every invalid file
        failures.append(f"{path.name}: invalid OpenAPI: {str(exc).splitlines()[0][:300]}")
        continue
    for code, reason in (spec_dict.get("x-exempt-permissions") or {}).items():
        EXEMPT[code] = f"{path.name}: {reason}"
    for route, item in (spec_dict.get("paths") or {}).items():
        for method, op in item.items():
            if method not in METHODS:
                continue
            op_count += 1
            where = f"{path.name} {method.upper()} {route}"
            missing = [x for x in REQUIRED_X if x not in op]
            if missing:
                failures.append(f"{where}: missing {missing}")
                continue
            perm = op["x-permission"]
            for code in (perm if isinstance(perm, list) else [perm]):
                ops_by_code.setdefault(code, []).append(where)
            for p in op.get("parameters", []) + item.get("parameters", []):
                if deref(spec_dict, p, path.parent).get("name") == "org_id":
                    failures.append(f"{where}: parameter org_id (TB-01)")
            for event in op["x-events"] or []:
                if event not in KNOWN_EVENTS:
                    failures.append(f"{where}: event {event} not in SERVICE_BOUNDARIES §4")
            if op["x-idempotent"] == "key":
                params = [deref(spec_dict, p, path.parent) for p in op.get("parameters", []) + item.get("parameters", [])]
                if not any(p.get("name") == "Idempotency-Key" for p in params):
                    failures.append(f"{where}: x-idempotent key but no Idempotency-Key header")
            body = (((op.get("requestBody") or {}).get("content") or {}).get("application/json") or {}).get("schema")
            if body and has_org_id(body, spec_dict, path.parent):
                failures.append(f"{where}: request body contains org_id (TB-01)")

matrix_codes = set(re.findall(r"^\| ([A-Z]{3}-[0-9]{2}[a-z]?) \|", MATRIX.read_text(encoding="utf-8"), re.M))
unknown = sorted(c for c in ops_by_code if c not in matrix_codes and c not in {"SELF-AUTH", "SYSTEM", "AUTHENTICATED"})
unmapped = sorted(c for c in matrix_codes if c not in ops_by_code and c not in EXEMPT)
for c in unknown:
    failures.append(f"x-permission {c} is not a PERMISSION_MATRIX code ({ops_by_code[c][0]})")
for c in unmapped:
    failures.append(f"PERMISSION_MATRIX {c} has no operation and no exemption")

print(f"files: {len(files) + 1}, operations: {op_count}, matrix codes: {len(matrix_codes)}, "
      f"mapped: {len(matrix_codes & set(ops_by_code))}, exempt: {len(EXEMPT)}")
if failures:
    print(f"FAIL ({len(failures)})")
    print("\n".join(failures))
    sys.exit(1)
print("PASS")
