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

## Reviewable changes

When a task changes repository files, make the changes on a feature branch and
open a GitHub pull request for review rather than leaving them only in the
working tree or pushing directly to `main`. Before reporting the work complete:

1. Run the relevant validation for the changes.
2. Commit the changes and include the current `Amp-Thread-ID` trailer.
3. Push the feature branch.
4. Open a pull request and include the current Amp thread URL in its description
   when practical.

Do not merge the pull request unless the user explicitly asks for that action.
Answer-only tasks and changes the user explicitly requests to keep local do not
need a pull request.

## Efficient verification

Use the cheapest check that can disprove the change. Verification should build
confidence in product behavior, not repeatedly reconfirm the test harness.

### Validation ladder

During implementation, validate only the layer being changed:

- Pure TypeScript or Vue logic: run the relevant Vitest file(s), for example
  `npm test -- src/utils/titlebar.test.ts`.
- Frontend type changes: run `npm run typecheck`.
- CSS or layout-only changes: use the running Vite/Tauri session and inspect the
  affected state. Do not run Cargo tests.
- Rust changes: run the narrowest relevant Cargo test target first.
- Cross-layer or release-sensitive changes: run the full frontend build and
  applicable Rust checks once after the implementation is coherent.

Before committing, run one final validation batch appropriate to the files
changed. Do not rerun an unchanged passing suite merely because visual
automation, documentation, or PR metadata changed.

### Visual verification budget

Functional facts must be checked through state, DOM/accessibility properties,
geometry, or tests whenever possible. Use screenshots for visual judgment, not
to prove that a click emitted an event, an item is selected, or a value was
persisted.

Default budgets per coherent change:

- Ordinary UI: one functional interaction pass and one final screenshot.
- Responsive layout: up to three representative viewport screenshots.
- Animation: fixed start, midpoint, and end frames. Record video only when
  motion or timing itself is under review.
- Multiple variants: create one final contact sheet. Afterward, recapture only
  variants affected by later changes.
- Native integration: perform one final native smoke pass after cheaper checks
  succeed.

Do not capture screenshots of console commands, coordinate experiments,
loading states, or repeated unchanged states unless that is the behavior under
test. Keep intermediate/debug captures out of `.amp/in/artifacts`; save only
artifacts intended for user review there.

### Stop-loss rule for visual automation

After two failed interactions or captures, stop retrying coordinates or sleeps.
Diagnose the harness and switch to a deterministic selector, keyboard path,
fixture, direct state assertion, or focused test. If none is available, explain
the verification limitation rather than continuing screenshot-driven probing.

Use stable accessible names or DOM selectors instead of absolute `xdotool`
coordinates whenever the WebView can be inspected. Seed test state directly;
do not automate native file choosers just to construct a known project state.

Run `amp orb services ensure` at most once unless service status or logs show a
concrete failure. The capture scripts wait for the Shipyard window, including a
cold Rust build, so do not poll service status or repeatedly restart while they
wait. Restart only when there is evidence of stale code or a failed process;
ordinary frontend edits should use the existing development session.
