I do not worry too much about public or private, nor modules, in this file. This is just to get the structure right.

```rs

struct Board {
    squares: BTreeMap<Square, SquareContents>
} 

// complete information about the configuration of a match
// the standard, two-player, five-card-per-player match is a particular value of the below type.
struct GameDesign {
    players: Vec<PlayerSetup>,
    squares: BTreeSet<Square>,
}

struct PlayerSetup {
    name: PlayerName,
    start: Square,
    finish: Square,
    initial_pile: Pile,
}

// contains just enough information to know everything needed to finish an ongoing match
struct MatchSnapshot {
    board: Board,
    current_player: PlayerId,
    active_card: Option<CardLocation>
}

struct CardLocation {
    square: Square,
    height: PileHeight,
}

// all the information needed for the client
struct ClientMatch {
    players: Vec<ClientPlayer>,
    past: Vec<BackwardRecord>,
    future: VecDeque<ForwardRecord>,
    viewed: MatchSnapshot,
    current: MatchSnapshot,
}

enum BackwardRecord {
    Intend(Square),
    Fulfil(Adjacency)
}

enum ForwardRecord {
    Intend(Adjacency),
    Fulfil(Square)
}

// all the information needed for the server
struct ServerMatch {
    design: GameDesign,
    log: Vec<ServerEvent>,
    current: MatchSnapshot,
}

enum ServerEvent {
    Intend(Adjacency),
    Fulfil(Square),
    EndTurn,
    Undo,
}

struct Adjacency {
    source: Square,
    direction: Direction,
}

```
