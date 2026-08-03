#!/usr/bin/env sh
set -eu

if [ ! -d fuzz ]; then
    echo "fuzz corpus: skipping; fuzz/ is not present"
    exit 0
fi

test -s docs/FUZZING.md

targets="
decode
in_place
stream_chunks
differential
profiles
x86_encode
x86_decode
neon
mime_body
pem_document
multibase_family
imap_payload
password_records
openpgp_armor
v2_runtime_codec
v2_incremental
v2_async
v2_assurance
"

for target in $targets; do
    mkdir -p "fuzz/corpus/$target"
done

find fuzz/artifacts -type f ! -name .gitignore -print | while IFS= read -r artifact; do
    echo "fuzz corpus: artifact must not be committed or left for release gates: $artifact" >&2
    exit 1
done

find fuzz/corpus -type f ! -name .gitkeep -print | while IFS= read -r corpus_file; do
    target="${corpus_file#fuzz/corpus/}"
    target="${target%%/*}"
    if ! printf '%s\n' "$targets" | grep -F -x -q "$target"; then
        echo "fuzz corpus: unknown corpus target for $corpus_file" >&2
        exit 1
    fi

    size="$(wc -c <"$corpus_file" | tr -d '[:space:]')"
    if [ "$size" -gt 65536 ]; then
        echo "fuzz corpus: $corpus_file is larger than 65536 bytes" >&2
        exit 1
    fi
done

echo "fuzz corpus: ok"
