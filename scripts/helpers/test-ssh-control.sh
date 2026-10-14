#!/usr/bin/env bash
# Local regression checks for SSH control-socket transport. No SSH connection.
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
socket_dir=$(mktemp -d /private/tmp/chillcenter-ssh-control-test.XXXXXX)
cleanup() { rm -rf "$socket_dir"; }
trap cleanup EXIT
private_dir="$socket_dir/private"
mkdir -m 0700 "$private_dir"
socket_path="$private_dir/control"

python3 - "$socket_path" <<'PY'
import socket, sys
sock = socket.socket(socket.AF_UNIX)
sock.bind(sys.argv[1])
sock.close()
PY
chmod 0600 "$socket_path"
python3 "$script_dir/validate-ssh-control.py" "$socket_path"

if python3 "$script_dir/validate-ssh-control.py" "$socket_dir/missing"; then
  echo 'Missing socket unexpectedly accepted.' >&2
  exit 1
fi
chmod 0660 "$socket_path"
if python3 "$script_dir/validate-ssh-control.py" "$socket_path"; then
  echo 'Group-accessible socket unexpectedly accepted.' >&2
  exit 1
fi
chmod 0600 "$socket_path"
chmod 0770 "$private_dir"
if python3 "$script_dir/validate-ssh-control.py" "$socket_path"; then
  echo 'Mode 0770 parent unexpectedly accepted.' >&2
  exit 1
fi
chmod 0777 "$private_dir"
if python3 "$script_dir/validate-ssh-control.py" "$socket_path"; then
  echo 'Mode 0777 parent unexpectedly accepted.' >&2
  exit 1
fi
chmod 0700 "$private_dir"
ln -s "$private_dir" "$socket_dir/parent-link"
if python3 "$script_dir/validate-ssh-control.py" "$socket_dir/parent-link/control"; then
  echo 'Symlink parent unexpectedly accepted.' >&2
  exit 1
fi
if python3 "$script_dir/validate-ssh-control.py" "$private_dir/control-%h"; then
  echo 'SSH expansion token unexpectedly accepted.' >&2
  exit 1
fi
if python3 "$script_dir/validate-ssh-control.py" "$private_dir/control-\${USER}"; then
  echo 'SSH dollar expansion unexpectedly accepted.' >&2
  exit 1
fi

echo 'SSH control socket regressions: PASS'
