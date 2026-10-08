#[cfg(target_arch = "wasm32")]
mod client {
    use gloo_net::http::Request;
    use leptos::prelude::*;
    use serde::Deserialize;
    use traffic_core::{Card, CardStatus, Game, GameDelta, Operation, Owner, Support};
    use traffic_network::{ClientMessage, ServerMessage};
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{MessageEvent, WebSocket};

    #[derive(Clone, Deserialize)]
    struct RoomResponse {
        code: String,
        owner: Owner,
    }

    pub fn run() {
        mount_to_body(App);
    }

    #[component]
    fn App() -> impl IntoView {
        let code = RwSignal::new(String::new());
        let notice = RwSignal::new(
            "Create a room in Heart's profile, then join in Spade's profile.".to_owned(),
        );
        let state = RwSignal::new(None::<Game>);
        let socket = RwSignal::new(None::<WebSocket>);
        let selected = RwSignal::new(None::<Card>);
        let card = RwSignal::new("H1".to_owned());
        let target = RwSignal::new("base 0 1".to_owned());

        let connect = move |room: String| {
            let host = web_sys::window().and_then(|window| window.location().host().ok());
            let Some(host) = host else { return };
            let Ok(ws) = WebSocket::new(&format!("ws://{host}/ws/{room}")) else {
                notice.set("WebSocket connection failed.".to_owned());
                return;
            };
            let state_for_message = state;
            let notice_for_message = notice;
            let handler = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                let Some(text) = event.data().as_string() else {
                    return;
                };
                let Ok(message) = serde_json::from_str::<ServerMessage>(&text) else {
                    return;
                };
                match message {
                    ServerMessage::ReplayBootstrap { created, events } => {
                        let mut game = Game::from_created(created);
                        for event in events {
                            if game.apply_event(event).is_err() {
                                notice_for_message.set("Replay failed.".to_owned());
                                return;
                            }
                        }
                        state_for_message.set(Some(game));
                        notice_for_message.set("Connected.".to_owned());
                    }
                    ServerMessage::Events { events } => state_for_message.update(|game| {
                        if let Some(game) = game {
                            for event in events {
                                let _ = game.apply_event(event);
                            }
                        }
                    }),
                    ServerMessage::Rejected { reason, .. } => {
                        notice_for_message.set(format!("Rejected: {reason:?}"))
                    }
                }
            });
            ws.set_onmessage(Some(handler.as_ref().unchecked_ref()));
            handler.forget();
            let open_socket = ws.clone();
            let open = Closure::<dyn FnMut()>::new(move || {
                if let Ok(text) = serde_json::to_string(&ClientMessage::RequestReplayBootstrap) {
                    let _ = open_socket.send_with_str(&text);
                }
            });
            ws.set_onopen(Some(open.as_ref().unchecked_ref()));
            open.forget();
            socket.set(Some(ws));
        };

        let submit = move |delta: GameDelta| {
            let Some(game) = state.get() else {
                notice.set("Join a room first.".to_owned());
                return;
            };
            let Some(ws) = socket.get() else {
                notice.set("WebSocket is not connected.".to_owned());
                return;
            };
            let request = ClientMessage::SubmitDelta {
                known_sequence: game.sequence(),
                delta,
            };
            if let Ok(text) = serde_json::to_string(&request) {
                let _ = ws.send_with_str(&text);
            }
        };

        view! {
            <main>
                <h1>"Traffic in Hanoi — local demo"</h1>
                <p>{move || notice.get()}</p>
                <section>
                    <button on:click=move |_| {
                        let code = code; let notice = notice;
                        spawn_local(async move {
                            match async { Request::post("/api/rooms").send().await?.json::<RoomResponse>().await }.await {
                                Ok(room) => { code.set(room.code.clone()); notice.set(format!("Created {} as {:?}.", room.code, room.owner)); },
                                Err(_) => notice.set("Room creation failed.".to_owned()),
                            }
                        });
                    }>"Create room (Heart)"</button>
                    <input placeholder="room code" prop:value=move || code.get() on:input=move |event| code.set(event_target_value(&event)) />
                    <button on:click=move |_| {
                        let room = code.get().to_uppercase(); let notice = notice;
                        spawn_local(async move {
                            match async { Request::post(&format!("/api/rooms/{room}/join")).send().await?.json::<RoomResponse>().await }.await {
                                Ok(joined) => notice.set(format!("Joined {} as {:?}. Reload this page to connect.", joined.code, joined.owner)),
                                Err(_) => notice.set("Room join failed.".to_owned()),
                            }
                        });
                    }>"Join room (Spade)"</button>
                    <button on:click=move |_| connect(code.get().to_uppercase())>"Connect"</button>
                </section>
                <section>
                    <p>{move || selected.get().map(|card| format!("Selected {card}; click a square or card support.")).unwrap_or_else(|| "Click idle to root, evicted to select, intended to fulfil.".to_owned())}</p>
                    {move || state.get().map(|game| visual_board(game, selected, state, socket, notice)).unwrap_or_else(|| view! { <p>"No game state."</p> }.into_any())}
                </section>
                <section>
                    <input prop:value=move || card.get() on:input=move |event| card.set(event_target_value(&event)) />
                    <button on:click=move |_| match state.get().and_then(|game| parse_card(&card.get(), &game).ok()) { Some(value) => submit(GameDelta::Operation(Operation::Root(value))), None => notice.set("Invalid card.".to_owned()) }>"Root"</button>
                    <input prop:value=move || target.get() on:input=move |event| target.set(event_target_value(&event)) />
                    <button on:click=move |_| { let Some(game) = state.get() else { return }; if let (Ok(card), Ok(target)) = (parse_card(&card.get(), &game), parse_target(&target.get(), &game)) { submit(GameDelta::Operation(Operation::Intention { card, target })); } }>"Intend"</button>
                    <button on:click=move |_| match state.get().and_then(|game| parse_card(&card.get(), &game).ok()) { Some(value) => submit(GameDelta::Operation(Operation::Fulfilment(value))), None => () }>"Fulfil"</button>
                    <button on:click=move |_| submit(GameDelta::Operation(Operation::End))>"End"</button>
                    <button on:click=move |_| submit(GameDelta::Undo)>"Undo"</button>
                    <p>"Target format: base x y, or card H1."</p>
                </section>
            </main>
        }
    }

    fn send(
        state: RwSignal<Option<Game>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
        delta: GameDelta,
    ) {
        let Some(game) = state.get() else {
            notice.set("Join a room first.".to_owned());
            return;
        };
        let Some(ws) = socket.get() else {
            notice.set("WebSocket is not connected.".to_owned());
            return;
        };
        let request = ClientMessage::SubmitDelta {
            known_sequence: game.sequence(),
            delta,
        };
        if let Ok(text) = serde_json::to_string(&request) {
            let _ = ws.send_with_str(&text);
        }
    }

    fn visual_board(
        game: Game,
        selected: RwSignal<Option<Card>>,
        state: RwSignal<Option<Game>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
    ) -> AnyView {
        let columns = game
            .layout()
            .squares()
            .map(|square| square.column)
            .max()
            .unwrap_or(0)
            + 1;
        let rows = game
            .layout()
            .squares()
            .map(|square| square.row)
            .max()
            .unwrap_or(0)
            + 1;
        let blocked = game.blocked().is_some();
        let squares = game.layout().squares().map(|square| {
            let cards = game.board().pile(square).into_iter().enumerate().map(|(index, card)| {
                let status = game.board().card_state(card).map(|state| state.status.clone()).unwrap_or(CardStatus::Idle);
                let class = format!("playing-card {} {}{}", if card.owner == Owner::Heart { "heart" } else { "spade" }, status_name(&status), if selected.get() == Some(card) { " selected" } else { "" });
                view! { <button class=class style=format!("left:{}px;z-index:{}", index * 18, index + 1) on:click=move |event: web_sys::MouseEvent| {
                    event.stop_propagation();
                    if let Some(mover) = selected.get() {
                        selected.set(None);
                        send(state, socket, notice, GameDelta::Operation(Operation::Intention { card: mover, target: Support::Card(card) }));
                    } else {
                        match status {
                            CardStatus::Idle => send(state, socket, notice, GameDelta::Operation(Operation::Root(card))),
                            CardStatus::Evicted => selected.set(Some(card)),
                            CardStatus::Intended(_) => send(state, socket, notice, GameDelta::Operation(Operation::Fulfilment(card))),
                        }
                    }
                }><span>{format!("{}{}", if card.owner == Owner::Heart { "♥" } else { "♠" }, card.rank.display())}</span></button> }
            }).collect_view();
            view! { <div class="board-square" style=format!("grid-column:{};grid-row:{}", square.column + 1, square.row + 1) on:click=move |_| if let Some(mover) = selected.get() { selected.set(None); send(state, socket, notice, GameDelta::Operation(Operation::Intention { card: mover, target: Support::Base(square) })); }><small>{format!("{},{}", square.column, square.row)}</small><div class="pile">{cards}</div></div> }
        }).collect_view();
        view! { <div class=if blocked { "board blocked" } else { "board" } style=format!("grid-template-columns:repeat({},minmax(7rem,1fr));grid-template-rows:repeat({},7rem)", columns, rows)>{squares}</div> }.into_any()
    }

    fn status_name(status: &CardStatus) -> &'static str {
        match status {
            CardStatus::Idle => "idle",
            CardStatus::Evicted => "evicted",
            CardStatus::Intended(_) => "intended",
        }
    }

    fn parse_card(text: &str, game: &Game) -> Result<Card, ()> {
        let mut chars = text.chars();
        let owner = match chars.next().map(|value| value.to_ascii_uppercase()) {
            Some('H') => Owner::Heart,
            Some('S') => Owner::Spade,
            _ => return Err(()),
        };
        Card::from_display(
            owner,
            chars.as_str().parse().map_err(|_| ())?,
            game.layout().rank_count(),
        )
        .map_err(|_| ())
    }
    fn parse_target(text: &str, game: &Game) -> Result<Support, ()> {
        let parts: Vec<_> = text.split_whitespace().collect();
        match parts.as_slice() {
            ["base", x, y] => Ok(Support::Base(traffic_core::Square::new(
                x.parse().map_err(|_| ())?,
                y.parse().map_err(|_| ())?,
            ))),
            ["card", card] => Ok(Support::Card(parse_card(card, game)?)),
            _ => Err(()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    client::run();
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {}
