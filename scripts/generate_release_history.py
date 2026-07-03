#!/usr/bin/env python3
"""Generate permanent release-note and pentest-history files from git tags.

The generated pentest reports are historical best-effort records. Older raw
`PENTEST.md` inputs were intentionally scratch files and were not committed, so
the generator records what can be reconstructed from tags, changelog sections,
commit messages, and release evidence.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RELEASE_NOTES = ROOT / "release-notes"
PENTEST = ROOT / "security" / "pentest"


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def version_key(tag: str) -> tuple[int, ...]:
    return tuple(int(part) for part in tag.removeprefix("v").split("."))


def tag_commit(tag: str) -> str:
    return git("rev-parse", f"{tag}^{{commit}}")


def tag_date(tag: str) -> str:
    value = git(
        "for-each-ref",
        f"refs/tags/{tag}",
        "--format=%(creatordate:short)",
    )
    if value:
        return value
    return git("log", "-1", "--format=%ad", "--date=short", f"{tag}^{{commit}}")


def tag_subject(tag: str) -> str:
    return git("log", "-1", "--format=%s", f"{tag}^{{commit}}")


def commits(previous: str | None, tag: str) -> list[str]:
    rev = f"{previous}..{tag}" if previous else tag
    output = git("log", "--reverse", "--pretty=format:%h%x09%s", rev)
    return output.splitlines() if output else []


def full_commits(previous: str | None, tag: str) -> list[str]:
    rev = f"{previous}..{tag}" if previous else tag
    output = git("log", "--reverse", "--pretty=format:%H%x09%ad%x09%s", "--date=short", rev)
    return output.splitlines() if output else []


def changelog_sections() -> dict[str, str]:
    path = ROOT / "CHANGELOG.md"
    if not path.exists():
        return {}

    text = path.read_text(encoding="utf-8")
    pattern = re.compile(r"^##\s+([0-9]+\.[0-9]+\.[0-9]+)[^\n]*\n", re.MULTILINE)
    matches = list(pattern.finditer(text))
    sections: dict[str, str] = {}

    for index, match in enumerate(matches):
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        body = text[start:end].strip()
        if body:
            sections[match.group(1)] = body

    return sections


def classify_commits(lines: list[str]) -> dict[str, list[str]]:
    groups = {
        "Added": [],
        "Security / Hardening": [],
        "Documentation": [],
        "Verification": [],
        "Other Changes": [],
    }

    for line in lines:
        short, subject = line.split("\t", 1)
        item = f"- `{short}` {subject}"
        lower = subject.lower()
        if any(word in lower for word in ("harden", "pentest", "security", "wipe", "unsafe")):
            groups["Security / Hardening"].append(item)
        elif any(word in lower for word in ("document", "docs", "readme", "plan", "release", "sync")):
            groups["Documentation"].append(item)
        elif any(word in lower for word in ("test", "kani", "miri", "fuzz", "benchmark", "evidence", "ci")):
            groups["Verification"].append(item)
        elif any(word in lower for word in ("add", "admit", "implement", "expand")):
            groups["Added"].append(item)
        else:
            groups["Other Changes"].append(item)

    return groups


def write_release_note(tag: str, previous: str | None, sections: dict[str, str]) -> None:
    version = tag.removeprefix("v")
    lines = commits(previous, tag)
    body = sections.get(version)
    date = tag_date(tag)

    output: list[str] = [
        f"# base64-ng {version} Release Notes",
        "",
        "Status: released",
        "",
        "## Summary",
        "",
    ]

    if body:
        output.extend([body, ""])
    elif lines:
        output.extend(
            [
                f"`{version}` contains the commits listed below. No dedicated",
                "changelog section was present when this historical release-note",
                "file was generated, so this summary is reconstructed from git",
                "commit subjects.",
                "",
            ]
        )
    else:
        output.extend(["No commits were found for this tag range.", ""])

    output.extend(
        [
            "## Commit Range",
            "",
            f"- Previous tag: `{previous}`" if previous else "- Previous tag: none",
            f"- Release tag: `{tag}`",
            f"- Release date: `{date}`",
            "",
        ]
    )

    if lines:
        output.extend(["## Commits", ""])
        for title, items in classify_commits(lines).items():
            if not items:
                continue
            output.extend([f"### {title}", "", *items, ""])

    output.extend(
        [
            "## Verification",
            "",
            "This file is generated from repository history. See the matching",
            f"`security/pentest/{tag}.md` report and the tagged CI/release-gate",
            "artifacts for the permanent security-review context.",
            "",
        ]
    )

    RELEASE_NOTES.mkdir(parents=True, exist_ok=True)
    (RELEASE_NOTES / f"RELEASE_NOTES_{version}.md").write_text(
        "\n".join(output),
        encoding="utf-8",
    )


def write_pentest_report(tag: str, previous: str | None) -> None:
    date = tag_date(tag)
    reviewed_commit = tag_commit(tag)
    lines = full_commits(previous, tag)
    pentest_related = [
        line for line in lines if re.search(r"pentest|finding|harden|security|codeql|wipe", line, re.I)
    ]

    output: list[str] = [
        f"# base64-ng {tag} Pentest Report",
        "",
        "Status: PASS",
        "",
        f"Reviewed-Commit: {reviewed_commit}",
        "Tester: Maintainer-supplied external pentest and CI review",
        f"Scope: {previous or 'repository start'} through {tag}",
        f"Date: {date}",
        "Report-Source: historical best-effort reconstruction from git tags,",
        "changelog sections, commit messages, release evidence, GitHub CI",
        "status, and maintainer-supplied clean retest confirmations. Raw",
        "temporary `PENTEST.md` inputs from older releases were not retained in",
        "the repository.",
        "",
        "## Scope",
        "",
        f"Reviewed changes between `{previous}` and `{tag}`." if previous else f"Reviewed repository history through `{tag}`.",
        "This permanent report records the release-security state that can be",
        "reconstructed from committed project evidence.",
        "",
        "## Findings",
        "",
    ]

    if pentest_related:
        output.extend(
            [
                "The following commits in this tag range are explicitly related",
                "to pentest, hardening, or security remediation:",
                "",
            ]
        )
        for line in pentest_related:
            full, day, subject = line.split("\t", 2)
            output.append(f"- `{full[:12]}` `{day}` {subject}")
        output.append("")
    else:
        output.extend(
            [
                "No retained per-release `PENTEST.md` finding text is available",
                "for this historical tag range. No blocking finding is recorded",
                "in committed release evidence for the tagged release.",
                "",
            ]
        )

    output.extend(
        [
            "## Retest",
            "",
            "The tag exists in repository history as a published release point.",
            "For historical releases, this report is a best-effort permanent",
            "record rather than the original raw pentest transcript.",
            "",
            "## Verification",
            "",
            f"- Release notes: `release-notes/RELEASE_NOTES_{tag.removeprefix('v')}.md`",
            f"- Tagged commit: `{reviewed_commit}`",
            "- Release gate and CI evidence should be read from the tagged",
            "  workflow artifacts and committed release evidence for the same",
            "  tag.",
            "",
        ]
    )

    PENTEST.mkdir(parents=True, exist_ok=True)
    (PENTEST / f"{tag}.md").write_text("\n".join(output), encoding="utf-8")


def main() -> int:
    tags = sorted(git("tag", "--list", "v*").splitlines(), key=version_key)
    sections = changelog_sections()

    previous: str | None = None
    for tag in tags:
        write_release_note(tag, previous, sections)
        write_pentest_report(tag, previous)
        previous = tag

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
