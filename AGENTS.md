# Shipyard agent guidance

## Preserve the Amp thread link

When Amp performs work in this repository, preserve the URL of the current Amp
thread in the commits that represent that work. Use the exact Git trailer below
and do not invent or substitute a different thread URL:

```text
Amp-Thread-ID: https://ampcode.com/threads/T-<current-thread-id>
```

Amp normally adds this trailer automatically. If you create or amend a commit
manually, include it yourself. When creating or updating a pull request, also
include the same URL in the pull-request description when practical. Shipyard
uses the commit trailer as the durable source of truth and exposes a matching
**Open Amp thread** action for the work item.

Only use the URL for the thread that actually performed the work. If there is
no current Amp thread, leave the trailer out rather than guessing.
