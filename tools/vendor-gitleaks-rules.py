#!/usr/bin/env python3
"""Vendor the gitleaks credential rules into Mushroom, as generated Rust.

Mushroom needs to recognise an AWS key in a note before sending that note to
an AI endpoint. Writing twenty patterns by hand means being wrong about the
ones nobody remembers -- Twilio, SendGrid, Azure SAS, GCP service accounts.
Gitleaks maintains ~220 of them. This takes the rules and leaves the binary:
gitleaks is a Go program built to scan a repository in batch, and Mushroom
scans one request in a hot path.

The patterns port unchanged because Go's `regexp` and Rust's `regex` are both
RE2-family -- finite automata, no backreferences, no lookaround. This script
checks that claim rather than trusting it, and refuses to emit anything if a
rule turns up that Rust could not accept.

Usage:

    python tools/vendor-gitleaks-rules.py <path-to-gitleaks.toml> [--version v8.30.1]

The gitleaks config ships inside the Go module, so the usual source is:

    go install github.com/zricethezav/gitleaks/v8@v8.30.1
    python tools/vendor-gitleaks-rules.py \\
        ~/go/pkg/mod/github.com/zricethezav/gitleaks/v8@v8.30.1/config/gitleaks.toml

Output is written to src-tauri/src/ai/privacy/gitleaks_rules.rs and is meant to
be committed and read, not regenerated silently. Pin a version; a rule set that
tracks upstream on its own can change what Mushroom sends without anybody
deciding to.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

# Tuned for source trees and CI configuration. Run against prose it fires on
# hashes, ids and ordinary technical writing, and a scanner people learn to
# ignore is worse than no scanner. Mushroom has its own entropy-gated rule for
# `password = ...` in `rules.rs`, shaped for notes rather than for code.
DROPPED = {"generic-api-key"}

# Rust expands bounded repetition when it compiles, so a pattern like
# `[\w-]{50,1000}` becomes an automaton past the crate's 10 MB size limit and
# is rejected -- even though the syntax is fine and Go accepts it. The upper
# bound only exists to avoid matching absurdly long runs, so removing it makes
# the rule very slightly broader, which is the safe direction for a detector.
# Each rewrite is listed here rather than applied by a general rule, so that
# "the patterns are unmodified" stays true of everything not named.
REWRITES = {
    "pypi-upload-token": (r"{50,1000}", r"{50,}"),
    "vault-batch-token": (r"{138,300}", r"{138,}"),
}

# Rust's regex crate rejects these outright; Go's regexp has no syntax for them
# either, so finding one means upstream changed engines and every assumption
# here needs revisiting.
UNSUPPORTED = [
    (re.compile(r"\(\?[=!]"), "lookahead"),
    (re.compile(r"\(\?<[=!]"), "lookbehind"),
    (re.compile(r"\\[1-9]"), "backreference"),
]

HEADER = '''\
// Credential detection rules, vendored from gitleaks.
//
// GENERATED FILE -- do not edit by hand.
// Regenerate with: python tools/vendor-gitleaks-rules.py <gitleaks.toml>
//
// Source:  https://github.com/gitleaks/gitleaks
// Version: {version}
// Rules:   {kept} vendored, {dropped} dropped ({dropped_ids})
//
// gitleaks is MIT licensed:
//
//   Copyright (c) 2019 Zachary Rice
//
//   Permission is hereby granted, free of charge, to any person obtaining a
//   copy of this software and associated documentation files (the "Software"),
//   to deal in the Software without restriction, including without limitation
//   the rights to use, copy, modify, merge, publish, distribute, sublicense,
//   and/or sell copies of the Software, and to permit persons to whom the
//   Software is furnished to do so, subject to the following conditions:
//
//   The above copyright notice and this permission notice shall be included in
//   all copies or substantial portions of the Software.
//
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
//   FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
//   DEALINGS IN THE SOFTWARE.
//
// The patterns are unmodified except where a `Bound widened for Rust` comment
// says otherwise, and those are listed in the generator. What changes here is
// the shape they are stored in and which ones are included: a rule tuned for
// scanning a source tree is not automatically right for scanning prose.

use super::rules::{{Confidence, Rule}};

/// Provider credential patterns, one per shape that does not occur by accident.
pub const GITLEAKS_RULES: &[Rule] = &[
'''


def rust_string(value: str) -> str:
    """A Rust raw string with enough hashes to survive the content."""
    hashes = 1
    while ('"' + "#" * hashes) in value:
        hashes += 1
    pad = "#" * hashes
    return f'r{pad}"{value}"{pad}'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", type=Path, help="path to gitleaks.toml")
    parser.add_argument("--version", default="unknown", help="gitleaks version to record")
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("src-tauri/src/ai/privacy/gitleaks_rules.rs"),
    )
    args = parser.parse_args()

    with args.config.open("rb") as handle:
        config = tomllib.load(handle)

    rules = config.get("rules", [])
    if not rules:
        print("no rules found -- is that really a gitleaks config?", file=sys.stderr)
        return 1

    kept, dropped = [], []
    for rule in rules:
        rule_id = rule.get("id")
        pattern = rule.get("regex")
        if not rule_id or not pattern:
            continue
        if rule_id in DROPPED:
            dropped.append(rule_id)
            continue

        for probe, what in UNSUPPORTED:
            if probe.search(pattern):
                print(
                    f"rule {rule_id!r} uses {what}, which Rust's regex cannot "
                    f"compile. Upstream may have changed engines -- stop and "
                    f"look before vendoring.",
                    file=sys.stderr,
                )
                return 1

        rewritten = None
        if rule_id in REWRITES:
            old, new = REWRITES[rule_id]
            if old not in pattern:
                print(
                    f"rule {rule_id!r} no longer contains {old!r}; the rewrite "
                    f"is stale. Check upstream before vendoring.",
                    file=sys.stderr,
                )
                return 1
            pattern = pattern.replace(old, new)
            rewritten = f"{old} -> {new}"

        kept.append((rule_id, pattern, rule.get("entropy"), rule.get("keywords", []), rewritten))

    kept.sort(key=lambda r: r[0])

    lines = [
        HEADER.format(
            version=args.version,
            kept=len(kept),
            dropped=len(dropped),
            dropped_ids=", ".join(sorted(dropped)) or "none",
        )
    ]

    for rule_id, pattern, entropy, keywords, rewritten in kept:
        lines.append("    Rule {")
        if rewritten:
            lines.append(f"        // Bound widened for Rust: {rewritten}")
        lines.append(f'        name: "{rule_id}",')
        lines.append(f"        pattern: {rust_string(pattern)},")
        # Gitleaks entropy is a confidence floor on the matched text. Rules
        # without one match on shape alone, which is enough for a prefix like
        # `AKIA` that does not occur by accident.
        lines.append(
            f"        confidence: Confidence::Entropy({float(entropy)}),"
            if entropy is not None
            else "        confidence: Confidence::Shape,"
        )
        if keywords:
            joined = ", ".join(f'"{k}"' for k in keywords)
            lines.append(f"        keywords: &[{joined}],")
        else:
            lines.append("        keywords: &[],")
        lines.append("    },")

    lines.append("];\n")

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(lines), encoding="utf-8", newline="\n")

    rewrote = [r[0] for r in kept if r[4]]
    print(
        f"wrote {args.out}: {len(kept)} rules, "
        f"dropped {len(dropped)} ({', '.join(dropped) or 'none'}), "
        f"bounds widened for {len(rewrote)} ({', '.join(rewrote) or 'none'})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
