# Security Policy

## Supported Versions

We provide security updates for the following versions of `latex-rs`:

| Version | Supported          |
| ------- | ------------------ |
| 1.4.x   | :white_check_mark: |
| < 1.4.0 | :x:                |

## Reporting a Vulnerability

We take the security of `latex-rs` seriously. If you discover a security vulnerability, please do not open a public issue. Instead, report it privately following these steps:

1.  **Email**: Send a detailed report to `andrezz1997@gmail.com`.
2.  **Details**: Include a description of the vulnerability, steps to reproduce, and any potential impact.
3.  **Acknowledgement**: We will acknowledge receipt of your report within 48 hours.
4.  **Fix**: We will work on a fix and coordinate a disclosure timeline with you.

## Security Architecture

`latex-rs` implements several measures to protect users:

- **LaTeX Sandbox**: pdflatex is executed with `-no-shell-escape` to prevent arbitrary command execution via `\write18`.
- **Path Sanitization**: All error messages are sanitized to prevent leaking sensitive information about your local file system structure.
- **Process Timeouts**: External processes (pdflatex, biber, etc.) have strict timeouts to prevent resource exhaustion (DoS).
- **WebKit Sandbox**: WebKit process sandboxing is enabled by default, except in specific environments where it is known to be incompatible (e.g., WSL, certain container runtimes).
- **CSP Headers**: The preview pane uses strict Content Security Policy headers to prevent script execution.
