#!/bin/bash -p
# Inspectable web-assets-only installer. Download/verification is performed by
# chill-update; this wrapper never accepts a URL, path, tag, or repository.
set -euo pipefail
export PATH=/usr/sbin:/usr/bin:/sbin:/bin
unset BASH_ENV ENV CDPATH GLOBIGNORE
[[ ${EUID:-$(id -u)} -eq 0 ]] || { echo 'Run as root.' >&2; exit 2; }
[[ $# -eq 0 ]] || { echo 'Usage: install-web.sh' >&2; exit 2; }
readonly helper=/usr/lib/chillcenter/chill-update
readonly config=/etc/chillcenter/updates.json
readonly server=/usr/lib/chillcenter/server/current/payload
readonly server_config=/etc/chillcenter/server.json
readonly certificate=/var/lib/chillcenter/data/tls/certificate.pem
for file in "$helper" "$config" "$server" "$server_config" "$certificate"; do [[ -f "$file" && ! -L "$file" ]] || { echo "Missing regular file: $file" >&2; exit 1; }; done
before=$(sha256sum "$server" "$server_config" "$certificate")
"$helper" --config "$config" check web
if [[ -L /usr/lib/chillcenter/web/current ]]; then
  "$helper" --config "$config" apply web
else
  "$helper" --config "$config" install web
fi
after=$(sha256sum "$server" "$server_config" "$certificate")
[[ "$before" == "$after" ]] || { echo 'Refusing success: server/config/TLS changed.' >&2; exit 1; }
echo 'Web assets installed; server binary, configuration, TLS and NAS were not modified.'
