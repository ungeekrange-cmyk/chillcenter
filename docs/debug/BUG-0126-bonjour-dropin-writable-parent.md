# BUG0126 — Bonjour operator must refuse writable drop-in parents

Date2026-10-21, author/root, independently reported by security review.
Status **FIXED_SOURCE / focused regression PASS**, no Pi change before review.

The root operator checked directory type/root ownership but allowed a preexisting
root-owned0775/0777 parent. Another allowed local writer could replace service
drop-ins before reload. This is a compromised-host precondition, not a new LAN
attack, but the installer must fail closed rather than trust it.

Reject group/other-writable parent modes without changing the existing directory.
Test both0775 and0777: no drop-in created, no daemon reload, effective families
unchanged. Journal/snapshots stay private and recoverable; no chmod of user-owned
or preexisting directories is performed.
