# Ocean Governance

## Maintainer

Ocean is currently maintained by a single person (@harunjeylan).

## Decision-Making

For v0.x (pre-1.0), Ocean uses a BDFL (Benevolent Dictator for Life) model:
- The maintainer makes final decisions on features, direction, and releases
- Community input is sought via issues and discussions
- Major decisions are announced as GitHub discussions

## Contributor Roles

- **Maintainer**: Repository owner, makes final decisions, manages releases
- **Committer**: Proven contributors granted write access (added as needed)
- **Contributor**: Anyone who submits a PR, files an issue, or improves documentation

## Pull Request Process

See CONTRIBUTING.md for the full PR workflow.

## Community Guidelines

- Be respectful and constructive
- Assume good faith
- Focus on what is best for the project
- Follow the Code of Conduct

## Release Process

Releases are cut from `main` by tagging `v*` (e.g., `v0.1.0`). The CI release workflow builds binaries for Windows, Linux, and macOS, uploads them to the GitHub release, and generates release notes. Crates.io publishing is a manual step.

## Sub-teams

No sub-teams currently. If the project grows, this document will be updated to define areas of ownership.
