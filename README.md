# ShipYard

ShipYard is an opinionated desktop shipping queue for local Git work and GitHub pull requests.

- **Local Work** contains checked-out work that has not become a pull request.
- **Pull Requests** contains open GitHub pull requests and their merge state.
- Merged and closed work leaves the queue.

Creating a pull request asks the configured coding agent to write the commit and pull-request metadata, commits local changes, checks compatibility with the latest default branch, pushes the branch, and opens the pull request. If Git finds a semantic merge conflict, ShipYard creates an isolated resolution worktree and asks the agent to resolve it before continuing.

## Integrations

ShipYard currently detects and supports:

- [Amp](https://ampcode.com/)
- [Codex](https://github.com/openai/codex)
- GitHub through an authenticated [GitHub CLI](https://cli.github.com/) installation

Choose one preferred coding agent in the global ShipYard Settings modal. Agent use is automatic after selection.

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
