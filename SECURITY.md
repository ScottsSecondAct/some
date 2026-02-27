# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Instead, report vulnerabilities privately using one of the following methods:

1. **GitHub Private Vulnerability Reporting:** Use the [Security Advisories](https://github.com/ScottsSecondAct/some/security/advisories/new) page to submit a private report directly on GitHub.
2. **Email:** Send details to **scott@ScottsSecondAct.com** *(replace with your preferred contact)*.

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Affected version(s)
- Potential impact

### What to Expect

- **Acknowledgment** within 72 hours of your report
- **Status update** within 7 days with an initial assessment
- **Resolution timeline** communicated once the issue is confirmed
- Credit in the release notes (unless you prefer to remain anonymous)

### Scope

As a terminal pager, `some` processes file content and user input in a local terminal environment. Relevant security concerns include:

- Memory safety issues (buffer overflows, use-after-free)
- Unsafe handling of malicious file content
- Escape sequence injection through displayed content
- Denial of service via crafted input files

### Out of Scope

- Issues requiring physical access to the machine
- Social engineering
- Vulnerabilities in dependencies with existing upstream fixes (please check first)
