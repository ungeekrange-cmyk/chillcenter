#!/usr/bin/env bash
# Mutation check of the operator script's --self-test: a validator that accepts everything must make it FAIL.
OP="$1"; M="$(mktemp -d)"
sed 's/^validate_archive_listing() {/validate_archive_listing() { return 0; }\nvalidate_archive_listing_orig() {/' "$OP" > "$M/m1.sh"
sed "s|^parse_backup_path() {|parse_backup_path() { printf \"/tmp/x\\n\"; return 0; }\nparse_backup_path_orig() {|" "$OP" > "$M/m2.sh"
for m in m1 m2; do /bin/bash "$M/$m.sh" --self-test >/dev/null 2>"$M/err"; rc=$?; echo "mutant $m : rc=$rc $( [ $rc -ne 0 ] && echo '(détecté ✓)' || echo '(NON DÉTECTÉ ✗)') $(head -c 120 "$M/err")"; done
/bin/bash "$OP" --self-test; echo "original : rc=$?"
rm -rf "$M"
