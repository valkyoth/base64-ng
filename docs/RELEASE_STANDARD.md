# Standard Release Closeout

This is the default maintainer process. The executable closeout currently covers
the synchronized 2.1.0 family: all 13 Rust crates and the npm WASM loader.
The older detailed campaign gates remain available for explicitly requested
high-assurance work. Neither process is a military or whole-program certification.

## One Candidate, One Checklist

1. Finish source/tooling edits, review the changes, and freeze a clean commit.
2. Collect exact-candidate CI, packages, SBOMs and reproducibility records. Keep
   applicable campaigns with their original source IDs and documented limits.
3. Attach the candidate security review and complete one checklist outside Git
   history. Inspect actual logs/artifacts, not just `passed` labels.
4. The maintainer authorizes unsigned indexing, verifies/seals that inventory,
   then separately authorizes tagging and publication. No command here publishes.

Do not commit a final report that creates another candidate. The source's old
`security/pentest/v2.1.0.md` stays historical; standard readiness instead checks
`candidate/security-review.md` in the signed evidence collection.

There is no separate integration-commit approval, approved-candidate-record
digest, or approval of the approval document in standard mode. The maintained
release tooling, reviewed checklist and authorized evidence signature are the
trust boundary. A collector may prepare records but must not invent review or
maintainer approval. Use a trusted interpreter and an isolated release account.

## Verification By Change

| Changes | Required verification |
| --- | --- |
| Codec, unsafe code, dispatch, secret handling | Relevant correctness, fuzz, sanitizer, native and performance campaigns; security review |
| Compiler, dependencies, build configuration | Rebuild and rerun affected campaigns |
| CI, packaging, release/test tooling | Focused regression tests and affected CI/package checks; assess any effect on captured builds |
| Documentation or evidence summaries | Content, provenance and integrity checks; no automatic runtime campaign rerun |

Routine CI continues to run tests, Clippy, formatting, feature/platform and
dependency checks. The final 2.1 candidate still needs the complete full-release
Rust CI job inventory, including all four deferred jobs. Failed, cancelled,
skipped or incomplete required jobs never count as passes.

For this closeout, unchanged runtime/build/package inputs are compared against
the reviewed recapture source `516f785`. Unknown input paths fail closed. The
original two packets, seven review records and fourteen claim dispositions are
still verified, not replaced by a statement that everything passed at new HEAD.
Intervening documentation and Python/shell/PowerShell tooling paths are listed
in the checklist for explicit impact review. Tooling changes are not automatically
irrelevant: if a changed collector alters flags, tests or generated binaries,
recapture the affected properties and reassess the reuse policy. This bounded
2.1 route will not accept new runtime inputs simply by editing checklist text.

## Prepare The Collection

Use a private `0700` `target/release-evidence` directory in the candidate checkout.
Retain the existing `scoped/` packets/reviews unchanged. Generate the candidate
artifacts using the existing workspace package, WASM-loader, SBOM and
reproducibility collectors. The layout and report schemas are the same as the
[candidate artifact inventory](RELEASE_EVIDENCE.md#scoped-index-activation-contract).
Retain build logs, failures and their resolutions. Do not overwrite a sealed
collection, mix old manifests into it, or relabel a previous candidate's output.

Capture the complete final CI snapshot into `candidate/ci.json` using:

```sh
gh run view RUN_ID --json conclusion,event,headSha,jobs,url,workflowName
```

Confirm that run through authenticated GitHub access. It must be the exact frozen
commit, `Rust CI`, `workflow_dispatch`, and every required job must have passed.
The local JSON snapshot is evidence, not a GitHub signature.

Generate the pending checklist, without running campaigns or granting approval:

```sh
sh scripts/finalize-release-evidence.sh --standard --template
```

This creates `RELEASE-CHECKLIST.json` only if absent. Fill in the reviewer,
change-impact rationale, and the three checks after completing the actual review:

- `security-review`: the attached review covers the exact candidate.
- `artifact-inspection`: package contents, all-family build results, npm install
  smoke, provenance, SBOMs, reproducibility and authenticated CI were inspected.
- `retained-campaigns`: the listed changes do not invalidate the accepted scopes;
  exclusions and accepted performance limitations remain documented.

The external `candidate/security-review.md` must contain singleton fields:

```text
Status: PASS
Reviewed-Commit: <full frozen commit>
Reviewed-Tree: <full frozen tree>
Tester: <actual reviewer>
Scope: <actual reviewed scope and any retained review references>
```

Include the actual assessment below those fields. Do not turn a review of an
older source into an exact-candidate approval by changing its header. A review
may reference previous assessments plus the final delta; that scope must be clear.

## Check, Seal, Publish

Read-only closeout validation is the default release gate:

```sh
sh scripts/stable_release_gate.sh
# Equivalent: sh scripts/finalize-release-evidence.sh --standard --check
```

After the maintainer explicitly approves this candidate's complete checklist:

```sh
BASE64_NG_FINALIZE_RELEASE_COMMIT=<approved-full-commit> \
  sh scripts/finalize-release-evidence.sh --standard --write
```

This publishes `RELEASE-INDEX.json` and an unsigned `FINAL-MANIFEST.txt` with
atomic, no-overwrite publication of each file, not an atomic transaction across
both files. If interrupted after publishing only the index, rerun the same
command: it revalidates all inputs and resumes only when the existing index
matches exactly. Stale or altered indexes and existing final manifests or
signatures are rejected. A failed retry preserves a pre-existing index.
This is interruption recovery, not a guarantee of power-loss durability.
Both outputs retain `release_authorized=false`.
No separate candidate-record hash approval is needed. Verification remains:

```sh
sh scripts/finalize-release-evidence.sh --standard --verify
```

Use the existing isolated `seal-release-evidence.sh v2.1.0` procedure with the
dedicated evidence key, then `validate-release-readiness.sh v2.1.0`. Signing is
not performed during builds. Readiness verifies the authorized signature, all
indexed files, the exact candidate, CI, packages and review. Never weaken a
failed gate to make the checklist pass.

Only after explicit publication authorization, create the authorized signed
`v2.1.0` tag and follow the Rust-family and npm publishing commands in
[RELEASE.md](RELEASE.md). Publish the checked npm archive, not a different rebuild.
Leave the evidence/report attachments outside source history and attach them to
the release. The older `stable_release_gate.sh check|candidate|release` modes
remain explicit opt-ins to the high-assurance campaigns, not required reruns
after standard closeout.
