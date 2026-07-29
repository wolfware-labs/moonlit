# Security Policy

## Reporting a Vulnerability

Please **do not** report security vulnerabilities through public GitHub issues.

Report them privately via GitHub's private vulnerability reporting: open the
repository's **Security** tab and click **Report a vulnerability**
(<https://github.com/wolfware-labs/moonlit/security/advisories/new>).

Please include:

- a description of the vulnerability and its impact,
- steps to reproduce (a minimal pipeline or plugin, where applicable),
- the affected version(s) or commit.

We aim to acknowledge reports within 3 business days and to share a remediation
timeline after triage. Please give us a reasonable opportunity to address the
issue before any public disclosure.

## Scope

Moonlit executes third-party plugins as sandboxed WebAssembly components under a
deny-by-default capability model (`network`, `exec`, `filesystem`, and `env`
granted per plugin). Reports about sandbox escapes, capability-enforcement
bypasses, or supply-chain integrity of the plugin/registry publish flow are
especially valuable.

## Supported Versions

Moonlit is in early development; security fixes are applied to the latest
release.
