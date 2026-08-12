# Shipyard

Shipyard is an opinionated desktop shipping queue for local Git work and GitHub pull requests.

- **Local Work** contains checked-out work that has not become a pull request.
- **Pull Requests** contains open GitHub pull requests, their merge state, and any local work not yet pushed to them.
- Merged and closed work leaves the queue.

Creating a pull request asks the configured coding agent to write the commit and pull-request metadata, commits local changes, checks compatibility with the latest default branch, pushes the branch, and opens the pull request. If Git finds a semantic merge conflict, Shipyard creates an isolated resolution worktree and asks the agent to resolve it before continuing.

When more work is added locally after a pull request exists, Shipyard changes the primary action to update or reconcile the pull request. A pull request cannot be merged while its local branch contains uncommitted or unpushed work.

## Integrations

Shipyard currently detects and supports:

- [Amp](https://ampcode.com/)
- [Codex](https://github.com/openai/codex)
- GitHub through an authenticated [GitHub CLI](https://cli.github.com/) installation

Choose one preferred coding agent in the global Shipyard Settings modal. Agent use is automatic after selection.

## Development

```sh
npm install
npm run tauri dev
```

Validation:

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

### Visual validation in Amp orbs

Fresh Amp orbs include a virtual Linux desktop for running and reviewing the real Tauri app. Start
the supervised app and noVNC portal with:

```sh
amp orb services ensure
```

The command prints the **Shipyard Native App** portal URL. The portal is interactive and displays
the maximized Tauri window running on display `:99`. On smaller screens, use **Actual size** for a
zoomed, pannable view and **Fit screen** to see the whole app. Agents can automate the app with
`DISPLAY=:99 xdotool` and capture app-only review artifacts with:

```sh
.agents/capture-screenshot
.agents/capture-recording 10
```

Both capture commands print the generated path under `.amp/in/artifacts/`. This Linux environment
can validate the shared Tauri UI and backend behavior, but actions explicitly implemented only for
macOS still require validation on macOS.
