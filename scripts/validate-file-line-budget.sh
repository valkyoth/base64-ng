#!/usr/bin/env sh
set -eu

max_lines=500

over_budget="$(
    find src crates -type f -name '*.rs' \
    ! -path '*/target/*' \
    ! -path '*/fuzz/*' \
    ! -path '*/perf/*' \
    | sort \
    | while IFS= read -r file; do
        lines="$(wc -l < "$file" | tr -d ' ')"
        if [ "$lines" -gt "$max_lines" ]; then
            printf '%s has %s lines; limit is %s\n' "$file" "$lines" "$max_lines"
        fi
    done
)"

if [ -n "$over_budget" ]; then
    printf '%s\n' "$over_budget" >&2
    exit 1
fi

echo "line budget: ok"
