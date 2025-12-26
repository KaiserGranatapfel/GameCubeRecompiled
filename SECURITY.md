# Security Policy

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in GCRecomp, please **do not** open a public issue. Instead, please report it privately using one of the following methods:

### Preferred Method
Open a [GitHub Security Advisory](https://github.com/yourusername/GCRecomp/security/advisories/new) (if you have access) or email the maintainers directly.

### Response Time
We aim to:
- Acknowledge receipt of your report within **48 hours**
- Provide an initial assessment within **7 days**
- Keep you informed of our progress
- Release a fix within **30 days** (depending on severity)

### What to Include
Please include the following information in your report:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)
- Your contact information (optional, for follow-up questions)

### Disclosure Policy
We follow a **coordinated disclosure** process:
1. We will work with you to understand and resolve the issue
2. We will not disclose the vulnerability until a fix is available
3. We will credit you in the security advisory (unless you prefer to remain anonymous)
4. We will publish the fix and advisory together

## Security Best Practices

When using GCRecomp:
- Only process DOL files from legally owned physical discs
- Do not distribute recompiled binaries without proper authorization
- Keep your Rust toolchain and dependencies up to date
- Review generated code before execution
- Report suspicious behavior or potential vulnerabilities

## Scope

### In Scope
- Security vulnerabilities in GCRecomp's codebase
- Issues that could lead to code execution, data corruption, or unauthorized access
- Vulnerabilities in dependencies that affect GCRecomp

### Out of Scope
- Issues in recompiled game code (these are the user's responsibility)
- Issues requiring physical access to your machine
- Social engineering attacks
- Denial of service attacks that don't compromise security

## Recognition

We appreciate responsible disclosure and will acknowledge security researchers who help improve GCRecomp's security. Contributors will be credited in security advisories (unless they prefer to remain anonymous).

Thank you for helping keep GCRecomp secure!

