# Local demo

The checked-in local demo still implements the retired support-based model. It
must not be used as evidence for the pile-based rules or protocol described in
the current agent specifications.

After the rewrite, the demo remains intentionally in-memory: private rooms,
anonymous cookie sessions, WebSockets, a room-owned authoritative
`ServerMatch`, and no restart recovery. It will:

1. bootstrap browsers with `GameDesign` plus the ordered `ServerEvent` log;
2. let a browser retain a sequence position for catch-up after a temporary
   disconnect;
3. broadcast every accepted `Intend`, `Fulfil`, `EndTurn`, and open-turn
   `Undo` event; and
4. render client-local step-back/step-forward review without treating it as a
   server command.

Run instructions will be restored once the server and web rewrite exists. The
old controls and support-target terminology are intentionally undocumented.
