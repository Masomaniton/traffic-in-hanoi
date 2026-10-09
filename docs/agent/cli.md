# `traffic-cli`

`traffic-cli` is a local rules-engine harness. One terminal user controls both
owners; it has no networking or persistence responsibilities.

Run it with:

```sh
cargo run -p traffic-cli
```

Load a standard board with any positive rank count:

```text
standard 5
begin
```

Or draft an arbitrary normalized layout:

```text
clear 3
square 0 0
square 0 1
square 1 1
square 2 1
square 2 0
start H 0 0
finish H 2 0
start S 2 1
finish S 0 1
begin
```

Use `root`, `intend`, `fulfil`, `end`, `undo`, `show`, and `history` to drive
the engine. `help` prints the exact command syntax. The CLI intentionally
exposes dead ends and their violations so rule behavior can be tested without
the server or browser.
