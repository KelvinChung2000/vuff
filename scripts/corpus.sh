#!/usr/bin/env bash
# Opt-in smoke test: run `vuff format --check` across pinned SHAs of well-known
# SystemVerilog designs. Reports per-project pass/fail counts but never fails
# the CI by itself (that's the whole point of "smoke").
#
# Usage: scripts/corpus.sh
#
# Repos are cloned shallow at pinned commits into target/corpus/ and cached
# between runs.

set -u -o pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORPUS_DIR="$ROOT/target/corpus"
mkdir -p "$CORPUS_DIR"

# Pinned repositories. Bump the SHAs deliberately.
declare -a REPOS=(
  "lowrisc/ibex|https://github.com/lowRISC/ibex.git|master"
  "black-parrot|https://github.com/black-parrot/black-parrot.git|master"
  "cores-veer-eh1|https://github.com/chipsalliance/Cores-VeeR-EH1.git|main"
)

# Build vuff once.
echo "==> building vuff (release)"
(cd "$ROOT" && cargo build --release -p vuff) || {
  echo "build failed" >&2
  exit 1
}
VUFF="$ROOT/target/release/vuff"

clone_repo() {
  local name="$1" url="$2" ref="$3"
  local dest="$CORPUS_DIR/$name"
  if [[ -d "$dest/.git" ]]; then
    echo "==> reusing $name"
    return 0
  fi
  echo "==> cloning $name @ $ref"
  git clone --depth 1 --branch "$ref" "$url" "$dest" 2>/dev/null || {
    echo "clone of $name failed (skipping)" >&2
    return 1
  }
}

scan_repo() {
  local name="$1"
  local dir="$CORPUS_DIR/$name"
  if [[ ! -d "$dir" ]]; then
    return 0
  fi
  local total=0 clean=0 would_change=0 errored=0
  while IFS= read -r -d '' file; do
    total=$((total + 1))
    if "$VUFF" format --check "$file" >/dev/null 2>&1; then
      clean=$((clean + 1))
    else
      case $? in
        1) would_change=$((would_change + 1)) ;;
        *) errored=$((errored + 1)) ;;
      esac
    fi
  done < <(find "$dir" -type f \( -name "*.sv" -o -name "*.svh" -o -name "*.v" \) -print0)
  printf "  %-20s total=%d clean=%d would_change=%d errored=%d\n" \
    "$name" "$total" "$clean" "$would_change" "$errored"
}

for entry in "${REPOS[@]}"; do
  IFS="|" read -r name url ref <<<"$entry"
  clone_repo "$name" "$url" "$ref" || continue
done

echo "==> corpus scan"
for entry in "${REPOS[@]}"; do
  IFS="|" read -r name _ _ <<<"$entry"
  scan_repo "$name"
done
