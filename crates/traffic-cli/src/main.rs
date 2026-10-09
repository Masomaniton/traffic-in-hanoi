use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use traffic_core::{
    ApplyResult, Card, CardStatus, CoreError, Game, Layout, Operation, Owner, Square, Support,
};

struct Draft {
    rank_count: u16,
    squares: BTreeSet<Square>,
    starts: BTreeMap<Owner, Square>,
    finishes: BTreeMap<Owner, Square>,
}

impl Draft {
    fn standard(rank_count: u16) -> Self {
        let layout = Layout::standard(rank_count).expect("standard layout is valid");
        Self {
            rank_count,
            squares: layout.squares().collect(),
            starts: BTreeMap::from([
                (Owner::Heart, layout.start(Owner::Heart)),
                (Owner::Spade, layout.start(Owner::Spade)),
            ]),
            finishes: BTreeMap::from([
                (Owner::Heart, layout.finish(Owner::Heart)),
                (Owner::Spade, layout.finish(Owner::Spade)),
            ]),
        }
    }

    fn empty(rank_count: u16) -> Self {
        Self {
            rank_count,
            squares: BTreeSet::new(),
            starts: BTreeMap::new(),
            finishes: BTreeMap::new(),
        }
    }

    fn build(&self) -> Result<Layout, CoreError> {
        Layout::new(
            self.rank_count,
            self.squares.clone(),
            self.starts.clone(),
            self.finishes.clone(),
        )
    }

    fn print(&self) {
        println!("layout draft: {} ranks", self.rank_count);
        println!(
            "squares: {}",
            self.squares
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );
        for owner in [Owner::Heart, Owner::Spade] {
            println!(
                "{owner}: start {:?}, finish {:?}",
                self.starts.get(&owner),
                self.finishes.get(&owner)
            );
        }
    }
}

fn main() {
    println!("Traffic in Hanoi CLI — type `help` for commands.");
    let mut draft = Draft::standard(5);
    let mut game: Option<Game> = None;
    loop {
        print!("traffic> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let words: Vec<_> = line.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        if matches!(words[0].to_ascii_lowercase().as_str(), "quit" | "exit") {
            break;
        }
        let result = (|| -> Result<(), CoreError> {
            match words[0].to_ascii_lowercase().as_str() {
                "help" => {
                    help();
                    Ok(())
                }
                "standard" => {
                    let ranks = optional_u16(words.get(1)).unwrap_or(5);
                    draft = Draft::standard(ranks);
                    game = None;
                    println!("loaded standard {ranks}-rank layout");
                    Ok(())
                }
                "clear" => {
                    let ranks = required_u16(words.get(1), "clear <ranks>")?;
                    draft = Draft::empty(ranks);
                    game = None;
                    println!("created empty {ranks}-rank layout draft");
                    Ok(())
                }
                "square" => {
                    let square = parse_square(&words, 1, "square <column> <row>")?;
                    draft.squares.insert(square);
                    game = None;
                    Ok(())
                }
                "start" => {
                    set_special(&mut draft.starts, &words, "start <H|S> <column> <row>")?;
                    game = None;
                    Ok(())
                }
                "finish" => {
                    set_special(&mut draft.finishes, &words, "finish <H|S> <column> <row>")?;
                    game = None;
                    Ok(())
                }
                "layout" => {
                    draft.print();
                    Ok(())
                }
                "begin" => {
                    game = Some(Game::new(draft.build()?));
                    println!("game started; Heart acts first.");
                    Ok(())
                }
                "show" => {
                    show(game.as_ref().ok_or_else(|| err("run `begin` first"))?);
                    Ok(())
                }
                "history" => {
                    history(game.as_ref().ok_or_else(|| err("run `begin` first"))?);
                    Ok(())
                }
                "root" => {
                    let game = game.as_mut().ok_or_else(|| err("run `begin` first"))?;
                    let card = parse_card(words.get(1), game.layout().rank_count())?;
                    apply(game, Operation::Root(card));
                    Ok(())
                }
                "intend" | "intention" => {
                    let game = game.as_mut().ok_or_else(|| err("run `begin` first"))?;
                    let card = parse_card(words.get(1), game.layout().rank_count())?;
                    let target = parse_support(&words, 2, game.layout().rank_count())?;
                    apply(game, Operation::Intention { card, target });
                    Ok(())
                }
                "fulfil" | "fulfill" => {
                    let game = game.as_mut().ok_or_else(|| err("run `begin` first"))?;
                    let card = parse_card(words.get(1), game.layout().rank_count())?;
                    apply(game, Operation::Fulfilment(card));
                    Ok(())
                }
                "end" => {
                    let game = game.as_mut().ok_or_else(|| err("run `begin` first"))?;
                    apply(game, Operation::End);
                    Ok(())
                }
                "undo" => {
                    let game = game.as_mut().ok_or_else(|| err("run `begin` first"))?;
                    game.undo()?;
                    println!("undid final batch");
                    Ok(())
                }
                _ => Err(err("unknown command; type `help`")),
            }
        })();
        if let Err(error) = result {
            eprintln!("error: {error}");
        }
    }
}

fn apply(game: &mut Game, operation: Operation) {
    match game.apply(operation) {
        Ok(ApplyResult::Applied) => println!("ok"),
        Ok(ApplyResult::Ended(owner)) => println!("{owner} wins!"),
        Ok(ApplyResult::Blocked(violations)) => {
            println!("DEAD END:");
            for violation in violations {
                println!("  - {violation}");
            }
        }
        Err(error) => eprintln!("error: {error}"),
    }
}

fn show(game: &Game) {
    println!(
        "active player: {} | active card: {:?} | winner: {:?}",
        game.active_player(),
        game.active_card(),
        game.winner()
    );
    for square in game.layout().squares() {
        let cards_at_square: Vec<_> = game
            .board()
            .cards()
            .filter_map(|(card, _)| {
                (game.board().square_of(card).ok() == Some(square)).then_some(card)
            })
            .collect();
        let cards = if cards_at_square.is_empty() {
            "·".to_owned()
        } else {
            cards_at_square
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!("{square}: {cards}");
    }
    println!("statuses:");
    for (card, state) in game.board().cards() {
        let status = match &state.status {
            CardStatus::Idle => "idle".to_owned(),
            CardStatus::Evicted => "evicted".to_owned(),
            CardStatus::Intended(support) => format!("intended -> {support}"),
        };
        println!("  {card}: {status}");
    }
    if let Some(violations) = game.blocked() {
        println!(
            "DEAD END: {}",
            violations
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
}

fn history(game: &Game) {
    if game.history().is_empty() {
        println!("current turn history is empty");
    } else {
        for (index, entry) in game.history().iter().enumerate() {
            println!("{:>3}: {entry}", index + 1);
        }
    }
    if !game.records().is_empty() {
        println!("completed turns: {}", game.records().len());
    }
}

fn help() {
    println!(
        "\
Layout commands:
  standard [ranks]                 load the standard layout (default 5)
  clear <ranks>                    create an empty layout draft
  square <column> <row>            add a square to the draft
  start|finish <H|S> <column> <row>
  layout                           display the draft
  begin                            validate draft and start a new game

Game commands:
  root <H1|S1>                     root-evict a card
  intend <card> base <x> <y>       intend to a square base
  intend <card> card <H1|S1>       intend onto a card support
  fulfil <card>                    fulfil an intention
  end                              attempt to end the turn
  undo                             remove the latest batch
  show | history | help | quit"
    );
}

fn parse_owner(value: Option<&&str>) -> Result<Owner, CoreError> {
    match value.copied().map(str::to_ascii_uppercase).as_deref() {
        Some("H") | Some("HEART") => Ok(Owner::Heart),
        Some("S") | Some("SPADE") => Ok(Owner::Spade),
        _ => Err(err("owner must be H or S")),
    }
}

fn parse_card(value: Option<&&str>, rank_count: u16) -> Result<Card, CoreError> {
    let value = value.ok_or_else(|| err("missing card"))?;
    let mut chars = value.chars();
    let owner = match chars.next().map(|c| c.to_ascii_uppercase()) {
        Some('H') => Owner::Heart,
        Some('S') => Owner::Spade,
        _ => return Err(err("card must look like H1 or S2")),
    };
    let rank = chars
        .as_str()
        .parse::<u16>()
        .map_err(|_| err("card rank must be a positive integer"))?;
    Card::from_display(owner, rank, rank_count)
}

fn parse_support(words: &[&str], start: usize, rank_count: u16) -> Result<Support, CoreError> {
    match words
        .get(start)
        .copied()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("base") => Ok(Support::Base(parse_square(
            words,
            start + 1,
            "intend <card> base <column> <row>",
        )?)),
        Some("card") => Ok(Support::Card(parse_card(words.get(start + 1), rank_count)?)),
        _ => Err(err(
            "target must be `base <column> <row>` or `card <H1|S1>`",
        )),
    }
}

fn parse_square(words: &[&str], start: usize, usage: &str) -> Result<Square, CoreError> {
    Ok(Square::new(
        required_u16(words.get(start), usage)?,
        required_u16(words.get(start + 1), usage)?,
    ))
}

fn set_special(
    map: &mut BTreeMap<Owner, Square>,
    words: &[&str],
    usage: &str,
) -> Result<(), CoreError> {
    map.insert(parse_owner(words.get(1))?, parse_square(words, 2, usage)?);
    Ok(())
}
fn required_u16(value: Option<&&str>, usage: &str) -> Result<u16, CoreError> {
    value
        .ok_or_else(|| err(usage))?
        .parse()
        .map_err(|_| err(usage))
}
fn optional_u16(value: Option<&&str>) -> Option<u16> {
    value.and_then(|value| value.parse().ok())
}
fn err(message: impl Into<String>) -> CoreError {
    CoreError::new(message)
}
