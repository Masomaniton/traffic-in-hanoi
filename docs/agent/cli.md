# `traffic-cli`

`traffic-cli` is a local pile-rules harness. One terminal user controls every
configured player; it has no networking, persistence, or authority role.

Run it with:

```sh
cargo run -p traffic-cli
```

It starts the standard two-player design. Commands are:

```text
intend <column> <row> <up|down|left|right>
fulfil <column> <row>
end-turn
undo
show
help
quit
```

`show` prints each pile, stored square state, active card, settled predicate,
and derived winner. The CLI intentionally exposes no root, support, or
eviction commands.
