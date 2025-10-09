# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in bru, please report it responsibly:

### Where to Report
- **Email**: [Create a private security advisory](https://github.com/nijaru/kombrucha/security/advisories/new) on GitHub
- **Do NOT** open public issues for security vulnerabilities

### What to Include
1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if you have one)

### Response Timeline
- **Initial response**: Within 48 hours
- **Status update**: Within 7 days
- **Fix timeline**: Depends on severity
  - Critical: 1-7 days
  - High: 7-14 days
  - Medium: 14-30 days
  - Low: Best effort

### Disclosure Policy
- We will work with you to understand and validate the issue
- Once fixed, we'll coordinate disclosure timing
- You will be credited in the security advisory (if desired)

## Security Considerations

### What bru Does
- Downloads bottles from GitHub Container Registry (ghcr.io)
- Extracts archives to /opt/homebrew/Cellar
- Creates symlinks in /opt/homebrew/bin
- Executes `install_name_tool` for bottle relocation
- Runs with user permissions (no sudo required)

### What bru Does NOT Do
- Does not execute arbitrary code from formulae
- Does not require elevated privileges
- Does not modify system files outside /opt/homebrew
- Does not send data to third parties
- Does not store credentials

### Known Limitations
- Trusts Homebrew API responses (formulae.brew.sh)
- Trusts bottle checksums from API
- No signature verification beyond SHA256 checksums
- Relies on HTTPS for security

### Potential Attack Vectors
1. **Compromised Homebrew API**: If formulae.brew.sh is compromised, malicious bottles could be served
   - *Mitigation*: Homebrew maintains the API, same risk as `brew`
2. **Man-in-the-middle**: HTTPS downgrade attacks
   - *Mitigation*: reqwest enforces HTTPS
3. **Bottle tampering**: Modified bottles with correct checksums
   - *Mitigation*: SHA256 verification, same as `brew`
4. **Path traversal**: Malicious bottles with path traversal in filenames
   - *Mitigation*: tar crate handles this, extraction limited to Cellar

### Security Best Practices
- Always verify what packages you're installing
- Use official Homebrew formulae
- Review `bru info <formula>` before installation
- Keep bru updated

## Security Updates
Security fixes will be released as patch versions and announced in:
- GitHub Security Advisories
- Release notes
- README.md

## Questions?
For security questions that aren't vulnerabilities, open a GitHub issue with the "security" label.
