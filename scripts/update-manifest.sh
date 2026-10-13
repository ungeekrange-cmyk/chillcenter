#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
manifest_stage=$(mktemp "$PWD/.manifest.XXXXXX")
git ls-files -z | while IFS= read -r -d '' relative_path; do
  if [ "$relative_path" != "MANIFEST.sha256" ]; then
    shasum -a 256 -- "$relative_path"
  fi
done > "$manifest_stage"
mv "$manifest_stage" MANIFEST.sha256
