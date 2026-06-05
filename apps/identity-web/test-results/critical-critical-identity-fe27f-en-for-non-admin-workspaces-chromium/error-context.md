# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: critical.spec.ts >> critical identity journeys >> service accounts admin surface is hidden for non admin workspaces
- Location: e2e/critical.spec.ts:51:3

# Error details

```
Error: locator.fill: Test ended.
Call log:
  - waiting for getByLabel('Email')
    - locator resolved to <div role="button" class="go1091339057" aria-label="Open match details for /verify-email">…</div>
    - fill("beta-owner+staging@example.com")
  - attempting fill action
    2 × waiting for element to be visible, enabled and editable
      - element is not visible
    - retrying fill action
    - waiting 20ms
    2 × waiting for element to be visible, enabled and editable
      - element is not visible
    - retrying fill action
      - waiting 100ms
    74 × waiting for element to be visible, enabled and editable
       - element is not visible
     - retrying fill action
       - waiting 500ms

```

```
Error: write EPIPE
```

```
Error: browserContext.close: Protocol error (Target.disposeBrowserContext): Failed to find context with id 38973706DCC0363E4214CEFEFBE61FA0
```