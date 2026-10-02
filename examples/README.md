# HyperKB Governance Examples

This directory contains reference examples demonstrating how HyperKB structures architectural records, standing policy directives, and proactive risk cards.

## Directory Structure

* **`directives/`**: Standing repository policy invariants enforced during AI agent workflows and pre-commit checks (`Rule of 5`).
* **`risks/`**: Proactive risk cards citing sensitive file patterns, known historical regression hotspots, or deployment hazards.
* **`decisions/`**: Architecture Decision Records (ADRs) tracking ratified technical designs and their trade-offs.

## Frontmatter Schema

All HyperKB governance documents use standard Markdown with a JSON frontmatter block delineated by `---hyperkb`:

```markdown
---hyperkb
{
  "id": "DIR-B090A56B3697",
  "kind": "directive",
  "title": "Zero Code Comments",
  "category": "behavior",
  "status": "active",
  "author": "Developer",
  "scope": [
    "src/**"
  ],
  "enforcement": "check_work"
}
---
# Document Title

Body text explaining the rationale, scope, and operational requirements.
```
