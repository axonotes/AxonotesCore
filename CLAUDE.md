# Claude Code Project Instructions

## Frontend Style Review

After writing or modifying frontend code (Svelte components, routes, styles), spawn a review agent to check compliance with UI/UX guidelines.

**Agent configuration:**

- Tool: `Task`
- Subagent type: `general-purpose`

**Agent prompt:**

```
Review the frontend code that was just written/modified against the Axonotes UI/UX guidelines.

Read the guidelines file: docs/ui-ux/GUIDELINES.md

Check the code for violations and report findings grouped by severity:

## Report Format

### [5/5 - Critical]
- Line X in `path/to/file.svelte`
  Description of violation.
  See: Guidelines section reference

### [4/5 - Important]
- ...

### [3/5 - Recommended]
- ...

### [2/5 - Nice-to-have]
- ...

### Summary
- Critical: N
- Important: N
- Recommended: N
- Nice-to-have: N

If no violations found, report: "No violations found."
```
