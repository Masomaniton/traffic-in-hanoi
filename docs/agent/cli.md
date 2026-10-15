# `traffic-cli`

`traffic-cli` is a local pile-rules harness. One terminal user controls every
configured player; it has no networking, persistence, or authority role.

The current checked-in CLI still targets the retired support-based rules and
will be replaced with the core rewrite. Its redesigned command surface should
mirror the replayable core events:

```text
intend <source-column> <source-row> <direction>
fulfil <source-column> <source-row>
end-turn
undo
show
```

It should also support creation of the standard design and inspection of the
derived pile, moving-state, planted/evicted-status, current-player, and
active-card data. It must not expose card-support, root, or eviction commands.
