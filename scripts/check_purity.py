#!/usr/bin/env python3
"""Static domain-kernel purity checker.

Flags the most common kernel-purity violations in a Rust crate WITHOUT needing a
Rust toolchain: default features that pull I/O/format crates, format-crate types
in public signatures, paths in public signatures, and direct I/O in non-test
code. Heuristic by design — a grep, not a compiler — so review findings rather
than trusting them blindly. Test modules can still produce false positives.

Usage:
    python check_purity.py <path-to-crate-root>

Exit code: 0 if no HARD findings, 1 otherwise (so CI can gate on it).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Concrete format crates (the `serde` trait itself is fine), runtimes, transport,
# storage. The `serde` derive trait is intentionally absent.
BANNED_CRATES = [
    "serde_yaml", "serde_yaml_bw", "tokio", "async_std", "async-std",
    "reqwest", "hyper", "tonic", "axum", "rusqlite", "sqlx",
]

HARD, WARN = "HARD", "WARN"


def parse_cargo(cargo: Path):
    """Return (deps, features) where deps maps name->is_optional, features is the
    [features] table as name->list. Uses tomllib if available, else a minimal
    fallback parser good enough for the two tables we need."""
    text = cargo.read_text(encoding="utf-8", errors="replace")
    try:
        import tomllib
        data = tomllib.loads(text)
        deps = {}
        for name, spec in (data.get("dependencies") or {}).items():
            deps[name] = bool(spec.get("optional")) if isinstance(spec, dict) else False
        feats = {k: list(v) for k, v in (data.get("features") or {}).items()}
        return deps, feats
    except Exception:
        return _fallback_parse(text)


def _fallback_parse(text: str):
    deps, feats, section = {}, {}, None
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("[") and s.endswith("]"):
            section = s.strip("[]")
            continue
        if not s or s.startswith("#") or "=" not in s:
            continue
        key, _, val = s.partition("=")
        key, val = key.strip().strip('"'), val.strip()
        if section == "dependencies":
            deps[key] = "optional = true" in val or "optional=true" in val
        elif section == "features":
            items = re.findall(r'"([^"]+)"', val)
            feats[key] = items
    return deps, feats


def cargo_findings(cargo: Path):
    out = []
    deps, feats = parse_cargo(cargo)

    # Non-optional banned deps are ALWAYS in the graph -> hard.
    for name, optional in deps.items():
        base = name.replace("-", "_")
        if base in BANNED_CRATES or name in BANNED_CRATES:
            if not optional:
                out.append((HARD, "Cargo.toml",
                            f"banned crate '{name}' is a non-optional dependency "
                            f"(make it `optional = true` behind a feature)"))

    # Default features that turn the convenience stack on.
    default = feats.get("default", [])
    if default:
        # Resolve which banned optional deps `default` reaches (one level + dep:).
        reachable = set()

        def walk(feat, seen=None):
            seen = seen or set()
            if feat in seen:
                return
            seen.add(feat)
            for item in feats.get(feat, []):
                if item.startswith("dep:"):
                    reachable.add(item[4:])
                elif item in feats:
                    walk(item, seen)
                else:
                    reachable.add(item.split("/")[0])
        for f in default:
            walk(f)
        hit = sorted(d for d in reachable
                     if d in BANNED_CRATES or d.replace("-", "_") in BANNED_CRATES)
        if hit:
            out.append((HARD, "Cargo.toml",
                        f"default features enable banned crate(s) {hit} — a kernel "
                        f"should be pure by default (set `default = []`, add a "
                        f"`full` umbrella for examples/tests)"))
        else:
            out.append((WARN, "Cargo.toml",
                        f"default features are non-empty ({default}); confirm none "
                        f"pull I/O — prefer `default = []` for a kernel"))
    return out


PUB_PATH = re.compile(r"\bpub\s+fn\b.*\b(Path|PathBuf)\b")
PUB_FMT_ERR = re.compile(
    r"->\s*Result<[^>]*\b(" + "|".join(re.escape(c) for c in BANNED_CRATES) + r")::Error")
VARIANT_FMT_ERR = re.compile(
    r"\((" + "|".join(re.escape(c) for c in BANNED_CRATES) + r")::Error\)")
DIRECT_IO = re.compile(
    r"\bstd::fs::|\bstd::net::|\bstd::env::var|\bstd::process::Command|"
    r"\bSystemTime::now\b|\breqwest::|\btokio::|\bhyper::")


def source_findings(src: Path):
    out = []
    for f in sorted(src.rglob("*.rs")):
        parts = set(f.parts)
        if "tests" in parts or "benches" in parts or "examples" in parts:
            continue
        in_test_mod = False
        for i, line in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            code = line.split("//", 1)[0]
            if "#[cfg(test)]" in line:
                in_test_mod = True  # crude: suppress the immediately following block
            loc = f"{f}:{i}"
            if PUB_FMT_ERR.search(code) or VARIANT_FMT_ERR.search(code):
                out.append((HARD, loc, "format crate named in a public signature "
                                       "(use an opaque kernel error; convert at the seam)"))
            if PUB_PATH.search(code):
                out.append((WARN, loc, "path in a public fn signature "
                                       "(take bytes/&str; let an adapter own the filesystem)"))
            if DIRECT_IO.search(code) and not in_test_mod:
                out.append((WARN, loc, "direct I/O / non-determinism in non-test code "
                                       "(move to an adapter; inject time/randomness)"))
    return out


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    root = Path(sys.argv[1])
    cargo = root / "Cargo.toml"
    src = root / "src"
    if not cargo.exists():
        print(f"no Cargo.toml at {root}", file=sys.stderr)
        sys.exit(2)

    findings = cargo_findings(cargo) + (source_findings(src) if src.exists() else [])
    hard = [f for f in findings if f[0] == HARD]
    warn = [f for f in findings if f[0] == WARN]

    print(f"\nDomain-kernel purity report for {root}\n" + "=" * 52)
    if not findings:
        print("No purity violations detected. (Still run the build-level checks: "
              "`cargo check --no-default-features` and the cargo tree assertion.)")
        sys.exit(0)

    for sev, label, items in (("HARD (fix first)", HARD, hard), ("WARN (review)", WARN, warn)):
        if items:
            print(f"\n{sev}: {len(items)}")
            for _, loc, msg in items:
                print(f"  {loc}\n      {msg}")

    print(f"\n{len(hard)} hard, {len(warn)} warn. "
          "Heuristic results — confirm against the source and run the build checks.")
    sys.exit(1 if hard else 0)


if __name__ == "__main__":
    main()
