# Security policy

Metten is in early development and is not available for trading or public release. Security protections and release readiness have not been fully verified.

## Report a vulnerability

Please use [GitHub's private vulnerability reporting form](https://github.com/moultonlavah/metten/security/advisories/new) to contact the maintainers. Include a description, affected version or commit, and safe reproduction steps. Do not include real credentials, account data, or private trading records. Please do not disclose exploitable details in a public issue or pull request.

Maintainers can use the private report to coordinate investigation and disclosure. Please allow time for a fix before publishing details.

## Store reports privately

Keep confidential reports and investigation details in GitHub's private report or draft advisory. Keep this policy in Git; do not commit confidential reports. If a local working copy is necessary, use the ignored root `private-security/` folder, with one subfolder per report and only the redacted evidence needed for investigation. Do not store secrets or private account records there.

The ignore rule prevents ordinary accidental additions; it is not encryption or access control, does not remove existing Git history, and can be bypassed by force-adding files. Never force-add reports. Keep any local copies access-restricted and remove them when no longer needed. Maintainers should enable GitHub notifications for security alerts on this repository; delivery depends on personal notification settings.
