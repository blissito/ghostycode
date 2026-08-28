---
name: security-review
description: Review trust boundaries, auth/authz, injection, secrets, filesystem/network exposure, dependencies, and exploitability without pretending a shallow lint is an audit.
invocation: model+user
---

# Security Review

## When to use
Use when looking for vulnerabilities, auth holes, secret leaks, or unsafe trust boundaries.

## Non-goals
- This is not a formal audit certificate.
- Do not claim exploitability without evidence.

## Workflow
1. Map trust boundaries and entry points.
2. Check auth/authz, injection, secrets, FS/network exposure, deps.
3. Rank findings by exploitability and impact.
4. Recommend fixes and verification steps.
