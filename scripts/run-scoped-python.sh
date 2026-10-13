#!/bin/sh
# The interpreter and this launcher must be authenticated before execution.
set -eu
case "${1:-}" in
    scoped_index.py|scoped_finalizer.py) program="$1"; shift ;;
    *) echo 'scoped Python: unsupported entry point' >&2; exit 2 ;;
esac
scripts_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)"
cache_dir="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-scoped-pycache.XXXXXX")"
child=
trap 'rm -rf -- "$cache_dir"' EXIT
stop() {
    trap '' HUP INT TERM
    if [ -n "$child" ]; then
        kill -s "$1" "$child" 2>/dev/null || true
        wait "$child" 2>/dev/null || true
    fi
    exit "$2"
}
trap 'stop HUP 129' HUP
trap 'stop INT 130' INT
trap 'stop TERM 143' TERM

# -B alone would still READ poisoned bytecode. A fresh cache prefix also
# redirects reads, while -I excludes inherited Python paths and user sites.
python3 -I -X "pycache_prefix=$cache_dir" -c '
import signal
import sys
signal.signal(signal.SIGINT, signal.default_int_handler)
import importlib.machinery
import runpy
scripts_dir, program, *arguments = sys.argv[1:]
# A cache prefix does not exclude legacy module.pyc or native extensions.
sys.path_importer_cache[scripts_dir] = importlib.machinery.FileFinder(
    scripts_dir,
    (importlib.machinery.SourceFileLoader, importlib.machinery.SOURCE_SUFFIXES),
)
sys.path.insert(0, scripts_dir)
sys.argv = [program, *arguments]
runpy.run_path(program, run_name="__main__")
' "$scripts_dir" "$scripts_dir/$program" "$@" &
child=$!
status=0
wait "$child" || status=$?
exit "$status"
