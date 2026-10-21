#!/bin/bash
# Optional OS maintenance only.  Installed as root:root, mode 0750.
set -Eeuo pipefail
PATH=/usr/sbin:/usr/bin:/sbin:/bin
export PATH
DEBIAN_FRONTEND=noninteractive
export DEBIAN_FRONTEND
LC_ALL=C
export LC_ALL
umask 077

readonly APT_GET=/usr/bin/apt-get
readonly DPKG=/usr/bin/dpkg
readonly REBOOT=/usr/sbin/reboot
readonly LOCK_TIMEOUT=300

fail() {
  printf '%s\n' "os-package-maintenance: $*" >&2
  exit 1
}

interrupted() {
  printf '%s\n' 'os-package-maintenance: interrupted; reboot suppressed' >&2
  exit 143
}

trap interrupted HUP INT TERM

[[ ${EUID} -eq 0 ]] || fail 'must be run as root by chillcenter-os-package-maintenance.service'
[[ -x "$APT_GET" && -x "$DPKG" && -x "$REBOOT" ]] || fail 'required fixed-path command missing'

dpkg_audit() {
  local audit
  audit="$($DPKG --audit)" || fail "dpkg --audit failed ($1)"
  [[ -z "$audit" ]] || {
    printf '%s\n%s\n' "os-package-maintenance: dpkg audit is non-empty ($1); reboot suppressed:" "$audit" >&2
    exit 1
  }
}

dpkg_audit before
"$APT_GET" -o APT::Update::Error-Mode=any update || fail 'apt-get update failed; reboot suppressed'
"$APT_GET" \
  -o DPkg::Lock::Timeout="$LOCK_TIMEOUT" \
  -o Dpkg::Options::=--force-confold \
  --with-new-pkgs --no-remove -y upgrade || fail 'apt-get upgrade failed; reboot suppressed'
"$APT_GET" check || fail 'apt-get check failed; reboot suppressed'
dpkg_audit after

printf '%s\n' 'os-package-maintenance: completed successfully; rebooting now'
exec "$REBOOT"
