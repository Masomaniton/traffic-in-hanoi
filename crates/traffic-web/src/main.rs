#[cfg(target_arch = "wasm32")]
mod client {
    use gloo_net::http::Request;
    use leptos::prelude::*;
    use serde::Deserialize;
    use traffic_core::{
        Adjacency, Card, Direction, GameDesign, MatchSnapshot, PlayerId, ServerEvent, Square,
        SquareState, SquareStatus, replay,
    };
    use traffic_network::{ClientMessage, LogSequence, ServerMessage};
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{HtmlInputElement, KeyboardEvent, MessageEvent, WebSocket};

    const SANDWICH_CARDS: usize = 2;
    const FAN_DELAY_MS: i32 = 350;

    #[derive(Clone, Deserialize)]
    struct RoomResponse {
        code: String,
        player: PlayerId,
    }

    #[derive(Clone)]
    struct Replica {
        design: GameDesign,
        log: Vec<ServerEvent>,
        snapshot: MatchSnapshot,
    }

    pub fn run() {
        mount_to_body(App)
    }

    #[component]
    fn App() -> impl IntoView {
        let code = RwSignal::new(String::new());
        let notice =
            RwSignal::new("Create a room, join it in another profile, then connect.".to_owned());
        let replica = RwSignal::new(None::<Replica>);
        let socket = RwSignal::new(None::<WebSocket>);
        let selected = RwSignal::new(None::<Square>);
        let fanned = RwSignal::new(None::<Square>);
        let pressing = RwSignal::new(None::<Square>);
        let suppress_click = RwSignal::new(false);
        // None means the newest authoritative snapshot. A value is a prefix length
        // in the immutable event log and therefore a read-only local review view.
        let review = RwSignal::new(None::<usize>);

        let connect = move |room: String| {
            let Some(host) = web_sys::window().and_then(|window| window.location().host().ok())
            else {
                return;
            };
            let Ok(ws) = WebSocket::new(&format!("ws://{host}/ws/{room}")) else {
                notice.set("WebSocket connection failed.".to_owned());
                return;
            };
            let state = replica;
            let message_notice = notice;
            let message_review = review;
            let handler = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                let Some(text) = event.data().as_string() else {
                    return;
                };
                let Ok(message) = serde_json::from_str::<ServerMessage>(&text) else {
                    message_notice.set("Received an invalid server message.".to_owned());
                    return;
                };
                match message {
                    ServerMessage::ReplayBootstrap { design, events } => {
                        match replay(&design, &events) {
                            Ok(snapshot) => {
                                state.set(Some(Replica {
                                    design,
                                    log: events,
                                    snapshot,
                                }));
                                message_review.set(None);
                                message_notice.set("Connected. Select a pile to begin.".to_owned());
                            }
                            Err(error) => message_notice.set(format!("Replay failed: {error}")),
                        }
                    }
                    ServerMessage::Events { events } => state.update(|state| {
                        if let Some(replica) = state {
                            for received in events {
                                if received.sequence.value() != replica.log.len() as u64 {
                                    message_notice
                                        .set("Event stream has a gap; reconnect.".to_owned());
                                    return;
                                }
                                let mut log = replica.log.clone();
                                log.push(received.event);
                                match replay(&replica.design, &log) {
                                    Ok(snapshot) => {
                                        replica.log = log;
                                        replica.snapshot = snapshot;
                                    }
                                    Err(error) => {
                                        message_notice
                                            .set(format!("Invalid event stream: {error}"));
                                        return;
                                    }
                                }
                            }
                        }
                    }),
                    ServerMessage::Rejected { reason, .. } => {
                        message_notice.set(format!("Rejected by the server: {reason:?}."));
                    }
                }
            });
            ws.set_onmessage(Some(handler.as_ref().unchecked_ref()));
            handler.forget();
            let opener = ws.clone();
            let open = Closure::<dyn FnMut()>::new(move || {
                if let Ok(text) = serde_json::to_string(&ClientMessage::RequestReplayBootstrap) {
                    let _ = opener.send_with_str(&text);
                }
            });
            ws.set_onopen(Some(open.as_ref().unchecked_ref()));
            open.forget();
            socket.set(Some(ws));
        };

        let key_replica = replica;
        let key_socket = socket;
        let key_notice = notice;
        view! {
            <main tabindex="0" on:keydown=move |event: KeyboardEvent| {
                if event.target().and_then(|target| target.dyn_into::<HtmlInputElement>().ok()).is_some() {
                    return;
                }
                let Some(value) = key_replica.get() else { return; };
                match event.key().as_str() {
                    "Backspace" if review.get().is_none() => {
                        event.prevent_default();
                        send(key_replica, key_socket, key_notice, ServerEvent::Undo);
                    }
                    "Enter" if review.get().is_none() => {
                        event.prevent_default();
                        send(key_replica, key_socket, key_notice, ServerEvent::EndTurn);
                    }
                    "ArrowLeft" => { event.prevent_default(); step_action(review, value.log.len(), false); }
                    "ArrowRight" => { event.prevent_default(); step_action(review, value.log.len(), true); }
                    "ArrowUp" => { event.prevent_default(); step_turn(review, &value.log, false); }
                    "ArrowDown" => { event.prevent_default(); step_turn(review, &value.log, true); }
                    "Escape" => { fanned.set(None); selected.set(None); }
                    _ => {}
                }
            }>
                <h1>"Traffic in Hanoi"</h1>
                <p class="notice">{move || notice.get()}</p>
                <section class="panel connection" aria-label="Room connection">
                    <button on:click=move |_| {
                        let code = code;
                        let notice = notice;
                        spawn_local(async move {
                            match Request::post("/api/rooms").send().await {
                                Ok(response) => match response.json::<RoomResponse>().await {
                                    Ok(room) => {
                                        code.set(room.code.clone());
                                        notice.set(format!("Created {} as player {}.", room.code, room.player.index() + 1));
                                    }
                                    Err(_) => notice.set("Room creation failed.".to_owned()),
                                },
                                Err(_) => notice.set("Room creation failed.".to_owned()),
                            }
                        });
                    }>
                        "Create room"
                    </button>
                    <input aria-label="Room code" placeholder="room code" prop:value=move || code.get()
                        on:input=move |event| code.set(event_target_value(&event)) />
                    <button on:click=move |_| {
                        let room = code.get().to_uppercase();
                        let notice = notice;
                        spawn_local(async move {
                            match Request::post(&format!("/api/rooms/{room}/join")).send().await {
                                Ok(response) => match response.json::<RoomResponse>().await {
                                    Ok(room) => notice.set(format!("Joined {} as player {}. Connect now.", room.code, room.player.index() + 1)),
                                    Err(_) => notice.set("Join failed.".to_owned()),
                                },
                                Err(_) => notice.set("Join failed.".to_owned()),
                            }
                        });
                    }>
                        "Join room"
                    </button>
                    <button on:click=move |_| connect(code.get().to_uppercase())>"Connect"</button>
                </section>
                {move || replica.get().map(|value| board(
                    value, replica, socket, notice, selected, fanned, pressing, suppress_click, review,
                )).unwrap_or_else(|| view! { <p class="panel">"No game state yet."</p> }.into_any())}
            </main>
        }
    }

    fn send(
        state: RwSignal<Option<Replica>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
        event: ServerEvent,
    ) {
        let (Some(replica), Some(socket)) = (state.get(), socket.get()) else {
            notice.set("Connect first.".to_owned());
            return;
        };
        let known_through = replica
            .log
            .len()
            .checked_sub(1)
            .map(|index| LogSequence::new(index as u64));
        match serde_json::to_string(&ClientMessage::SubmitEvent {
            known_through,
            event,
        }) {
            Ok(text) => match socket.send_with_str(&text) {
                Ok(()) => notice.set("Waiting for server confirmation…".to_owned()),
                Err(_) => notice.set("Connection is not open.".to_owned()),
            },
            Err(_) => notice.set("Could not encode the action.".to_owned()),
        }
    }

    fn step_action(review: RwSignal<Option<usize>>, log_len: usize, forward: bool) {
        let position = review.get().unwrap_or(log_len);
        let next = if forward {
            position.saturating_add(1).min(log_len)
        } else {
            position.saturating_sub(1)
        };
        review.set((next != log_len).then_some(next));
    }

    fn step_turn(review: RwSignal<Option<usize>>, log: &[ServerEvent], forward: bool) {
        let position = review.get().unwrap_or(log.len());
        let mut starts = vec![0];
        starts.extend(log.iter().enumerate().filter_map(|(index, event)| {
            matches!(event, ServerEvent::EndTurn).then_some(index + 1)
        }));
        let next = if forward {
            starts
                .into_iter()
                .find(|start| *start > position)
                .unwrap_or(log.len())
        } else {
            starts
                .into_iter()
                .rev()
                .find(|start| *start < position)
                .unwrap_or(0)
        };
        review.set((next != log.len()).then_some(next));
    }

    fn direction_between(source: Square, target: Square) -> Option<Direction> {
        match (
            target.column as i32 - source.column as i32,
            target.row as i32 - source.row as i32,
        ) {
            (0, 1) => Some(Direction::Up),
            (0, -1) => Some(Direction::Down),
            (1, 0) => Some(Direction::Right),
            (-1, 0) => Some(Direction::Left),
            _ => None,
        }
    }

    fn schedule_fan(
        square: Square,
        pressing: RwSignal<Option<Square>>,
        fanned: RwSignal<Option<Square>>,
        suppress_click: RwSignal<bool>,
    ) {
        let callback = Closure::<dyn FnMut()>::new(move || {
            if pressing.get() == Some(square) && fanned.get().is_none() {
                fanned.set(Some(square));
                suppress_click.set(true);
            }
        });
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                FAN_DELAY_MS,
            );
            callback.forget();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn board(
        replica: Replica,
        state: RwSignal<Option<Replica>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
        selected: RwSignal<Option<Square>>,
        fanned: RwSignal<Option<Square>>,
        pressing: RwSignal<Option<Square>>,
        suppress_click: RwSignal<bool>,
        review: RwSignal<Option<usize>>,
    ) -> AnyView {
        let current = replica.snapshot.clone();
        let position = review.get().unwrap_or(replica.log.len());
        let snapshot = replay(&replica.design, &replica.log[..position]).unwrap_or(current.clone());
        let design = replica.design.clone();
        let squares: Vec<_> = snapshot.board().squares().collect();
        let max_column = squares
            .iter()
            .map(|square| square.column)
            .max()
            .unwrap_or(0);
        let max_row = squares.iter().map(|square| square.row).max().unwrap_or(0);
        let reviewing = position != replica.log.len();
        let title = design
            .player(snapshot.current_player())
            .name()
            .as_str()
            .to_owned();
        let winner = snapshot
            .winner(&design)
            .map(|player| design.player(player).name().as_str().to_owned());
        let square_views = squares.into_iter().map(|square| {
            let cards = snapshot.board().pile(square).to_vec();
            let has_cards = !cards.is_empty();
            let state_value = snapshot.board().state(square);
            let moving = matches!(state_value, SquareState::Moving(_));
            let evicted = snapshot.status(&design, square) == SquareStatus::Evicted;
            let selected_here = selected.get() == Some(square);
            let target = selected.get().and_then(|source| direction_between(source, square)).is_some();
            let is_fanned = fanned.get() == Some(square);
            let class = format!(
                "board-square{}{}{}{}",
                moving.then_some(" moving").unwrap_or(""),
                evicted.then_some(" evicted").unwrap_or(""),
                selected_here.then_some(" selected").unwrap_or(""),
                (target && !selected_here).then_some(" target").unwrap_or(""),
            );
            let square_label = format!(
                "Square {square}; {}{}; {} card{}",
                if moving { "moving" } else { "idling" },
                if evicted { ", evicted" } else { "" },
                cards.len(), if cards.len() == 1 { "" } else { "s" },
            );
            let grid_style = format!(
                "grid-column:{};grid-row:{};",
                square.column + 1,
                max_row - square.row + 1,
            );
            let pile_summary = cards.iter().map(|card| card_name(&design, *card)).collect::<Vec<_>>().join(", ");
            let card_views = displayed_cards(&cards, is_fanned).into_iter().enumerate().map(|(index, (card, left))| {
                card_view(&design, card, left, index)
            }).collect_view();
            let click_state = state;
            let click_socket = socket;
            let click_notice = notice;
            view! {
                <button class=class style=grid_style aria-label=square_label
                    on:mousedown=move |_| {
                        if has_cards && fanned.get().is_none() {
                            pressing.set(Some(square));
                            schedule_fan(square, pressing, fanned, suppress_click);
                        }
                    }
                    on:mouseup=move |_| pressing.set(None)
                    on:mouseleave=move |_| pressing.set(None)
                    on:keydown=move |event: KeyboardEvent| match event.key().as_str() {
                        "i" | "I" => { event.prevent_default(); fanned.set(Some(square)); }
                        "Escape" => { event.prevent_default(); fanned.set(None); }
                        _ => {}
                    }
                    on:click=move |_| {
                        if suppress_click.get() {
                            suppress_click.set(false);
                            return;
                        }
                        if fanned.get().is_some() {
                            fanned.set(None);
                            return;
                        }
                        if reviewing { return; }
                        if moving {
                            selected.set(None);
                            send(click_state, click_socket, click_notice, ServerEvent::Fulfil(square));
                        } else if let Some(source) = selected.get() {
                            if let Some(direction) = direction_between(source, square) {
                                selected.set(None);
                                match Adjacency::new(source, direction) {
                                    Ok(adjacency) => send(click_state, click_socket, click_notice, ServerEvent::Intend(adjacency)),
                                    Err(_) => notice.set("That target is not adjacent.".to_owned()),
                                }
                            } else if !has_cards {
                                notice.set("Choose a non-empty source pile.".to_owned());
                            } else {
                                selected.set(Some(square));
                            }
                        } else if !has_cards {
                            notice.set("Choose a non-empty source pile.".to_owned());
                        } else {
                            selected.set(Some(square));
                        }
                    }>
                    <span class="pile" class:fanned=move || is_fanned>
                        <span class="pile-summary">{pile_summary}</span>
                        {card_views}
                    </span>
                    <span class="square-label">{square.to_string()}</span>
                </button>
            }
        }).collect_view();
        let board_style = format!(
            "grid-template-columns:repeat({},minmax(0,1fr));grid-template-rows:repeat({},minmax(0,1fr));",
            max_column + 1,
            max_row + 1
        );
        view! {
            <section class="panel game-layout">
                <div class="board-wrap" on:click=move |_| {
                    if fanned.get().is_some() { fanned.set(None); }
                }>
                    <div class="board" style=board_style>{square_views}</div>
                </div>
                <aside class="game-meta" aria-live="polite">
                    <h2>"Match status"</h2>
                    <dl>
                        <dt>"Current player"</dt><dd>{title}</dd>
                        <dt>"Active card"</dt><dd>{match snapshot.active_card() { Some(location) => format!("At {}", location.square()), None => "Choose an opening card".to_owned() }}</dd>
                        <dt>"Turn"</dt><dd>{if snapshot.is_settled(&design) { "Settled" } else { "In progress" }}</dd>
                        <dt>"Winner"</dt><dd>{winner.unwrap_or_else(|| "—".to_owned())}</dd>
                    </dl>
                    <div class="controls">
                        <button disabled=move || reviewing on:click=move |_| send(state, socket, notice, ServerEvent::Undo)>"Undo"</button>
                        <button disabled=move || reviewing on:click=move |_| send(state, socket, notice, ServerEvent::EndTurn)>"End turn"</button>
                    </div>
                    {reviewing.then(|| view! { <p class="reviewing">"Reviewing history. Return to the newest position to play."</p> })}
                </aside>
            </section>
        }.into_any()
    }

    fn displayed_cards(cards: &[Card], fanned: bool) -> Vec<(Card, f64)> {
        let visible = if fanned {
            cards
        } else {
            let count = cards.len().min(SANDWICH_CARDS + 2);
            &cards[cards.len().saturating_sub(count)..]
        };
        let last = visible.len().saturating_sub(1);
        visible
            .iter()
            .enumerate()
            .map(|(index, card)| {
                let left = if fanned {
                    6.0 + 31.0 * index as f64
                } else if last == 0 {
                    36.0
                } else {
                    6.0 + 30.0 * index as f64 / last as f64
                };
                (*card, left)
            })
            .collect()
    }

    fn card_name(design: &GameDesign, card: Card) -> String {
        format!(
            "{} {}",
            design.player(card.owner).name().as_str(),
            card.rank.value()
        )
    }

    fn card_view(design: &GameDesign, card: Card, left: f64, index: usize) -> AnyView {
        let owner = design.player(card.owner).name().as_str().to_owned();
        let initial = owner.chars().next().unwrap_or('?');
        let marking = format!("{}{}", card.rank.value(), initial);
        let class = if card.owner.index() == 0 {
            "playing-card heart"
        } else {
            "playing-card spade"
        };
        view! {
            <span class=class style=format!("left:{left:.2}%;z-index:{index};") role="img" aria-label=card_name(design, card)>
                <span>{marking.clone()}</span><span class="inverted">{marking}</span>
            </span>
        }.into_any()
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    client::run()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
