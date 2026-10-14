#!/usr/bin/env bash
# Local regression checks for BUG-0041. No SSH or Pi mutation.
set -euo pipefail

fixture_dir=$(mktemp -d /private/tmp/chillcenter-help-contract-test.XXXXXX)
cleanup() { rm -rf "$fixture_dir"; }
trap cleanup EXIT
expected_help=$'Chill Center : Usage : chill-server --config /chemin/config.json\n       chill-server --backup --config /chemin/config.json\n       chill-server --restore /chemin/sauvegarde --config /chemin/config.json --confirm-replace\n       chill-server --recover --config /chemin/config.json --confirm-replace\n       chill-server --export-cinefolio --source-db /chemin/cinefolio.sqlite --image-cache /chemin/images --output /chemin/nouveau.zip'

write_fixture() {
  path=$1
  status=$2
  output=$3
  printf '#!/usr/bin/env bash\nprintf %%s ' > "$path"
  printf '%q' "$output" >> "$path"
  printf ' >&2\nexit %s\n' "$status" >> "$path"
  chmod 0700 "$path"
}

verify_help_contract() {
  binary=$1
  if help_output=$("$binary" --help 2>&1); then
    help_status=0
  else
    help_status=$?
  fi
  test "$help_status" -eq 1 || return 1
  test "$help_output" = "$expected_help" || return 1
}

help_exit_one="$fixture_dir/help-exit-one"
write_fixture "$help_exit_one" 1 "$expected_help"
if "$help_exit_one" --help >/dev/null 2>&1; then
  echo 'Former simple help probe unexpectedly passed.' >&2
  exit 1
fi
verify_help_contract "$help_exit_one"

help_exit_zero="$fixture_dir/help-exit-zero"
write_fixture "$help_exit_zero" 0 "$expected_help"
if verify_help_contract "$help_exit_zero"; then
  echo 'Exit 0 unexpectedly accepted.' >&2
  exit 1
fi
help_exit_signal="$fixture_dir/help-exit-signal"
write_fixture "$help_exit_signal" 139 "$expected_help"
if verify_help_contract "$help_exit_signal"; then
  echo 'Signal-like exit unexpectedly accepted.' >&2
  exit 1
fi
help_wrong_usage="$fixture_dir/help-wrong-usage"
write_fixture "$help_wrong_usage" 1 'Chill Center : configuration inaccessible'
if verify_help_contract "$help_wrong_usage"; then
  echo 'Wrong usage unexpectedly accepted.' >&2
  exit 1
fi

echo 'BUG-0041 help contract regressions: PASS'
