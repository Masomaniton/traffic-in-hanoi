use std::io::{self, Write};
use traffic_core::{Adjacency, Direction, GameDesign, ServerEvent, Square, replay};

fn main() {
    let design = GameDesign::standard();
    let mut log = Vec::new();
    println!("Traffic in Hanoi pile CLI — help for commands.");
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
        match words[0] {
            "quit" | "exit" => break,
            "help" => println!(
                "intend <x> <y> <up|down|left|right> | fulfil <x> <y> | end-turn | undo | show | quit"
            ),
            "show" => show(&design, &log),
            "intend" => match adjacency(&words) {
                Ok(a) => submit(&design, &mut log, ServerEvent::Intend(a)),
                Err(e) => eprintln!("error: {e}"),
            },
            "fulfil" => match square(&words, 1) {
                Ok(s) => submit(&design, &mut log, ServerEvent::Fulfil(s)),
                Err(e) => eprintln!("error: {e}"),
            },
            "end-turn" => submit(&design, &mut log, ServerEvent::EndTurn),
            "undo" => submit(&design, &mut log, ServerEvent::Undo),
            _ => eprintln!("unknown command"),
        }
    }
}
fn submit(d: &GameDesign, log: &mut Vec<ServerEvent>, event: ServerEvent) {
    let mut candidate = log.clone();
    candidate.push(event);
    match replay(d, &candidate) {
        Ok(_) => {
            *log = candidate;
            println!("ok")
        }
        Err(e) => eprintln!("rejected: {e}"),
    }
}
fn show(d: &GameDesign, log: &[ServerEvent]) {
    match replay(d, log) {
        Ok(snapshot) => {
            println!(
                "player {:?}; active {:?}; settled {}; winner {:?}",
                snapshot.current_player(),
                snapshot.active_card(),
                snapshot.is_settled(d),
                snapshot.winner(d)
            );
            for square in snapshot.board().squares() {
                let cards = snapshot
                    .board()
                    .pile(square)
                    .iter()
                    .map(|c| format!("p{}:{}", c.owner.index(), c.rank.value()))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{square}: {:?} [{}]", snapshot.board().state(square), cards)
            }
        }
        Err(e) => eprintln!("replay failed: {e}"),
    }
}
fn square(words: &[&str], offset: usize) -> Result<Square, String> {
    Ok(Square::new(
        words
            .get(offset)
            .ok_or("missing column")?
            .parse()
            .map_err(|_| "bad column")?,
        words
            .get(offset + 1)
            .ok_or("missing row")?
            .parse()
            .map_err(|_| "bad row")?,
    ))
}
fn adjacency(words: &[&str]) -> Result<Adjacency, String> {
    let direction = match words.get(3).copied() {
        Some("up") => Direction::Up,
        Some("down") => Direction::Down,
        Some("left") => Direction::Left,
        Some("right") => Direction::Right,
        _ => return Err("direction must be up, down, left, or right".to_owned()),
    };
    Adjacency::new(square(words, 1)?, direction).map_err(|e| e.to_string())
}
