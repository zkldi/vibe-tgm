#!/usr/bin/env bash
set -euo pipefail

# Basenames (exact match or glob pattern) to suppress.
# Extend this list for any intentional hyphenated names.
WHITELIST=(
  ".dl-cache"
  "*.mdc"
  "commit-graph"
  "Justfile-*"
  "query-*.json"
  "rust-toolchain*"
  "docker-compose*"
  "vite-env.d.ts"
  "react-router.config.ts"
  "cpp_vendor/*"
  ".git-blame-ignore-revs"
  ".githooks/pre-push"
  "worker-configuration.d.ts"
  "deploy/kamal/.scripts/*"
  "deploy/kamal/.secrets/*"
  "rust_public/rg_formats/test_files/*"
  "rust_vendor/epaint/fonts/*"
  "LICENSE-APACHE"
  "LICENSE-MIT"
  "*.woff2"
  "*.ttf"
  "*.otf"
  "vendor/*"
  "wiki/*"
  "typescript/zenstyle_vscode_plugin/language-configuration.json"
  "typescript/web/app/routes/*"
)

is_whitelisted() {
  local entry="$1"          # full relative path
  local name                # basename only
  name="$(basename "$entry")"
  for pattern in "${WHITELIST[@]}"; do
    # Patterns are matched against BOTH the full relative path and the basename.
    # In bash `case`, * matches /, so:
    #   "vendor/*"   matches the full path of anything under vendor/
    #   "*.mdc"      matches any .mdc file regardless of depth
    #   "rust-toolchain*" matches that basename anywhere in the tree
    # shellcheck disable=SC2254
    case "$entry" in $pattern) return 0 ;; esac
    # shellcheck disable=SC2254
    case "$name"  in $pattern) return 0 ;; esac
  done
  return 1
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

errors=0

while IFS= read -r entry; do
  if is_whitelisted "$entry"; then
    continue
  fi
  echo "ERROR: hyphenated name: $entry"
  errors=$((errors + 1))
done < <(
  fd \
    --hidden \
    --exclude '.git' \
    --type f \
    --type d \
    -- '[-]'
)

echo ""
if ((errors > 0)); then
  echo "$errors hyphenated name(s) found."
  exit 1
else
  echo "No hyphenated names found."
fi
