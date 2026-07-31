# Cross-Crate Semantic Corpus

`v1/cases.tsv` is the initial versioned behavior corpus shared by the core,
streaming, bytes, Tokio, serde, and sanitization surfaces. It records successful
bytes exactly. Failure rows retain each surface's real mutation and diagnostic
contract rather than pretending every destination is transactional.

Run the offline schema and implementation checks with:

```sh
scripts/check-semantic-corpus.sh
```

The runner is repository verification tooling, not a published crate.
