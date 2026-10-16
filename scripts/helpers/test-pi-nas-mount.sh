#!/usr/bin/env bash
# Local regression checks for BUG-0040. No SSH or Pi mutation.
set -euo pipefail

validate_nas_records() {
  awk -v target='/srv/volumes/video' -v source='//192.0.2.92/Video' '
    $1 == target && $2 == source && $3 == "cifs" {
      matching_records++
      count = split($4, options, ",")
      for (option_index = 1; option_index <= count; option_index++) {
        if (options[option_index] == "ro") found_ro = 1
        if (options[option_index] == "rw") found_rw = 1
      }
    }
    END { exit matching_records == 1 && found_ro && !found_rw ? 0 : 1 }
  '
}

assert_accepts() {
  printf '%s\n' "$1" | validate_nas_records
}

assert_rejects() {
  if printf '%s\n' "$1" | validate_nas_records; then
    echo "Unexpectedly accepted fixture: $1" >&2
    exit 1
  fi
}

autofs_and_ro_first=$'/srv/volumes/video /dev/root autofs rw,relatime\n/srv/volumes/video //192.0.2.92/Video cifs ro,nosuid,nodev,noexec'
old_pattern='^/srv/volumes/video //192\.0\.2\.92/Video cifs .*(^|,)ro(,|$)'
if printf '%s\n' "$autofs_and_ro_first" | grep -Eq "$old_pattern"; then
  echo 'BUG-0040 regression fixture no longer demonstrates the former false rejection.' >&2
  exit 1
fi

assert_accepts "$autofs_and_ro_first"
assert_accepts '/srv/volumes/video //192.0.2.92/Video cifs nosuid,ro,nodev'
assert_rejects '/srv/volumes/video //192.0.2.92/Video cifs rw,relatime,nosuid'
assert_rejects '/srv/volumes/video //192.0.2.92/Video cifs rw,relatime,ro,nosuid'
assert_rejects '/srv/volumes/video //192.0.2.93/Video cifs ro,nosuid'
assert_rejects '/srv/volumes/video //192.0.2.92/Video nfs ro,nosuid'
assert_rejects '/srv/volumes/other //192.0.2.92/Video cifs ro,nosuid'
assert_rejects $'/srv/volumes/video //192.0.2.92/Video cifs ro,nosuid\n/srv/volumes/video //192.0.2.92/Video cifs rw,nosuid'
assert_rejects $'/srv/volumes/video //192.0.2.92/Video cifs ro,nosuid\n/srv/volumes/video //192.0.2.92/Video cifs ro,nodev'

echo 'BUG-0040 NAS mount guard regressions: PASS'
