# Traffic in Hanoi

Traffic in Hanoi is a board game based on the [Tower of Hanoi](https://en.wikipedia.org/wiki/Tower_of_Hanoi) puzzle. All you need is a deck of cards to play. This repo seeks to digitize this game.

# AI Workflow and Project Structure

This section describes the AI workflow used in this project. The point is that there is a clear delination between human-written and agent-generated text. This [README.md](README.md) is written, while [AGENTS.md](AGENTS.md) is generated.

All code is in the `crates` directory. Within the `docs` directory, there is `agent` for generated documentation and `human` for written documentation. The `human` directory is broken down further into directories corresponding to the writer. Note, documentation within the `human` directory does not necessarily accurately reflect the current state of the project, and is rather a place to store the specifications each human expects the agents to follow. The `agent` directory has `intended` for planned features and fixes and `fulfilled` for a complete and current documentation of the codebase. Each time the changes described in an `intended` file are completed, the file is deleted and `fulfilled` is adjusted accordingly. The point of `notes` is to be a place where we do not have to worry about the workflow invariants.

The following invariants must hold at every commit:
- There are no contradictory specifications inside the `human` directory.
- Every specification in the `human` directory appears as a currently in place or planned addition in `agent` under `fulfilled` or `intended`, respectively.
- The `agent` directory contains a complete and up-to-date description of the entire codebase
