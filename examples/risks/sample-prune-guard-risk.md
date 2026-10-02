---hyperkb
{
  "id": "01a0f539-7334-73bf-9c04-598056ada8d8",
  "kind": "risk",
  "status": "acknowledged",
  "owner": "Developer",
  "issue": null,
  "paths": [
    "src/core/scanner.rs",
    "src/storage/**"
  ],
  "versions": [],
  "environments": [],
  "supersedes": null
}
---
# Prune Guard Failure on Partial Mount

Partial disk mounts must never cause unmounted documents to be pruned as missing.

## Risk Acknowledgment
- **Acknowledged by**: Developer
- **Date**: 2026-10-01T02:32:22.915432+00:00
- **Rationale**: Scanner maintains transactional consistency and skips non-existent mount paths
