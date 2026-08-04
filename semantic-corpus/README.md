# Cross-Crate Semantic Corpus

`v1/cases.tsv` is the initial versioned behavior corpus shared by the 2.0 core
one-shot and heapless incremental APIs, synchronous streams, bytes, Tokio,
serde, and sanitization surfaces. It records successful bytes exactly. Failure
rows retain each surface's real mutation and diagnostic contract rather than
pretending every destination is transactional. The 2.0 one-shot runner also
asserts complete-destination transactionality, while incremental decode checks
that only the row's declared committed prefix can escape before rejection.

Run the offline schema and implementation checks with:

```sh
scripts/check-semantic-corpus.sh
```

The runner is repository verification tooling, not a published crate.
