# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Ocean, please report it privately.

**Do not** open a public issue. Instead, email the maintainer directly or use GitHub's private vulnerability reporting feature.

You can expect an acknowledgement within 48 hours and a fix timeline depending on severity. Critical issues will have a patch within 7 days.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Scope

- Filesystem sandbox bypass
- Symlink escape attacks
- Remote code execution via document parsing
- Credential leakage
- Database injection via document content

## Out of Scope

- Denial of service via large/malformed documents
- Dependency CVEs (handled via Dependabot + cargo audit)
