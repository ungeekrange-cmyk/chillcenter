# Security

[Français](SECURITY.md) | **English**

Chill Center is intended for a trusted home LAN. It currently provides no
authentication: a profile is not a security boundary between users. Do not
expose the server to the Internet or set up public port forwarding.

HTTPS encrypts traffic. Accepting a certificate on first connection requires
checking that you are contacting your own server. Never include private keys
or passwords in a report.

Certificates, passwords, tokens and real configuration must never be committed,
even to a private repository. The server generates TLS material in its data
directory; TMDB keys are configured at runtime. Keep real addresses in an
ignored local file (for example, `server.local.json`), never in distributed
examples. Only the port and safe loopback listening are provided by default.

Before publication, scan Git history and objects, not just the current working
directory. A secret removed from a file remains in earlier commits: if genuine,
revoke it, then explicitly agree on history cleanup. Identify and review
synthetic test values; do not hide them behind a blanket detection exclusion.

## Reporting a vulnerability

[Dependency controls and tracked advisories](docs/security/DEPENDENCIES.en.md)
distinguish the global lockfile from actual shipping components. Real alerts
remain visible; updates are never automatically merged.

If GitHub offers “Report a vulnerability” for this repository, use that private
channel. Otherwise, ask the maintainer for a private channel without publicly
posting exploitation instructions or attaching secrets. No long-term security
support or remediation deadline is promised at this stage.

Include the version, preconditions, impact and an anonymized reproduction.
Player dependencies and their licences remain separate from Chill Center code.
