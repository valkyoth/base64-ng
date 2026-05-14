#!/usr/bin/env sh
set -eu

if [ ! -d fuzz ]; then
    echo "fuzz corpus: skipping; fuzz/ is not present"
    exit 0
fi

test -s docs/FUZZING.md

for target in decode in_place stream_chunks differential; do
    mkdir -p "fuzz/corpus/$target"
done

find fuzz/artifacts -type f ! -name .gitignore -print | while IFS= read -r artifact; do
    echo "fuzz corpus: artifact must not be committed or left for release gates: $artifact" >&2
    exit 1
done

find fuzz/corpus -type f ! -name .gitkeep -print | while IFS= read -r corpus_file; do
    case "$corpus_file" in
        fuzz/corpus/decode/* | fuzz/corpus/in_place/* | fuzz/corpus/stream_chunks/* | fuzz/corpus/differential/*)
            ;;
        *)
            echo "fuzz corpus: unknown corpus target for $corpus_file" >&2
            exit 1
            ;;
    esac

    size="$(wc -c <"$corpus_file" | tr -d '[:space:]')"
    if [ "$size" -gt 65536 ]; then
        echo "fuzz corpus: $corpus_file is larger than 65536 bytes" >&2
        exit 1
    fi
done

echo "fuzz corpus: ok"
