This file contains an AI-generated plan I adapted of the transition from support-based to pile-based rules. This is somewhat of a total overhall.

1. Treat docs/human/rules.md as normative and rewrite docs/agent/rules.md from it.
2. Redesign traffic-core around piles, moving squares, and derived status predicates, with table-driven tests for every action and derived eviction case.
3. Replace protocol deltas, server validation, CLI controls, and the Leptos interaction model together.
4. Preserve the useful architecture: authoritative server, replayable ordered log, stale-sequence rejection, and client replay. Those are still sound; the contents of the replay events simply change.
5. Update AGENTS.md to reflect the total redesign.

## Current status

Step 1 is complete. `docs/agent/rules.md` now specifies the pile-based model,
general square-lattice boards, arbitrary ordered players, arbitrary rank-legal
initial piles, positional active-card tracking, and the two board actions.

Steps 2 through 5 have not started. The existing core, protocol, server, CLI,
web UI, and `AGENTS.md` still describe and implement the support-based model.
