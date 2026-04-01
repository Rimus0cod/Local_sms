# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | ✅ Yes    |
| < 1.0   | ❌ No     |

## Reporting a Vulnerability

**Please do NOT report security vulnerabilities via public GitHub Issues.**

Instead, use **GitHub Security Advisories**:
👉 https://github.com/Rimus0cod/Local_sms/security/advisories/new

### What to include in your report

- A description of the vulnerability and its impact
- Steps to reproduce or a proof of concept
- The affected version(s)
- Any suggested mitigations

### Response timeline

- We aim to acknowledge reports within **72 hours**
- We aim to provide a fix or mitigation plan within **14 days** for critical issues
- We will credit reporters in the release notes unless you request otherwise

## Scope

This security policy covers:

- The Tauri desktop application (`apps/tauri-client`)
- The QUIC relay server (`apps/localmessenger_server`)
- The cryptographic library crates (`crates/crypto`, `crates/messaging`, `crates/transport`)
- The storage layer (`crates/storage`)

## Out of Scope

- The browser fallback frontend (development only, not a production deployment)
- Vulnerabilities in third-party dependencies (please report those upstream)
- Attacks requiring full OS compromise (out of threat model scope)

## Disclosure Policy

We follow **coordinated disclosure**. We ask that you give us a reasonable time to fix
the issue before publishing details publicly.

## Acknowledgements

We thank all security researchers who responsibly disclose vulnerabilities to us.