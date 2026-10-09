#[cfg(target_arch = "wasm32")]
mod client {
    use gloo_net::http::Request;
    use leptos::prelude::*;
    use serde::Deserialize;
    use traffic_core::{
        Card, CardStatus, Game, GameDelta, HistoryCursor, Operation, Owner, Support,
    };
    use traffic_network::{ClientMessage, ServerMessage};
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{KeyboardEvent, MessageEvent, WebSocket};

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
        let state = RwSignal::new(None::<HistoryCursor>);
        let socket = RwSignal::new(None::<WebSocket>);
        let selected = RwSignal::new(None::<Card>);
        let connect = move |room: String| {
            let Some(host) = web_sys::window().and_then(|window| window.location().host().ok())
            else {
                return;
            };
            let Ok(ws) = WebSocket::new(&format!("ws://{host}/ws/{room}")) else {
                notice.set("WebSocket connection failed.".to_owned());
                return;
            };
            let (message_state, message_notice) = (state, notice);
            let handler = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
                let Some(text) = message.data().as_string() else {
                    return;
                };
                let Ok(message) = serde_json::from_str::<ServerMessage>(&text) else {
                    return;
                };
                match message {
                    ServerMessage::ReplayBootstrap { created, deltas } => {
                        let mut game = Game::from_created(created);
                        for delta in deltas {
                            if game.apply_delta(delta).is_err() {
                                message_notice.set("Replay failed.".to_owned());
                                return;
                            }
                        }
                        message_state.set(Some(HistoryCursor::from_game(game)));
                    }
                    ServerMessage::Events { events } => message_state.update(|cursor| {
                        if let Some(cursor) = cursor {
                            for event in events {
                                if !cursor.apply_event(event) {
                                    message_notice
                                        .set("Event stream is inconsistent; reconnect.".to_owned());
                                }
                            }
                        }
                    }),
                    ServerMessage::Rejected { reason, .. } => {
                        message_notice.set(format!("Rejected: {reason:?}"))
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
        view! {
            <main>
                <h1>"Traffic in Hanoi — local demo"</h1><p>{move || notice.get()}</p>
                <button on:click=move |_| { let code = code; let notice = notice; spawn_local(async move { match async { Request::post("/api/rooms").send().await?.json::<RoomResponse>().await }.await { Ok(room) => { code.set(room.code.clone()); notice.set(format!("Created {} as {:?}.", room.code, room.owner)); }, Err(_) => notice.set("Room creation failed.".to_owned()) } }); }>"Create room (Heart)"</button>
                <input placeholder="room code" prop:value=move || code.get() on:input=move |event| code.set(event_target_value(&event)) />
                <button on:click=move |_| { let room = code.get().to_uppercase(); let notice = notice; spawn_local(async move { match async { Request::post(&format!("/api/rooms/{room}/join")).send().await?.json::<RoomResponse>().await }.await { Ok(joined) => notice.set(format!("Joined {} as {:?}. Reload this page to connect.", joined.code, joined.owner)), Err(_) => notice.set("Room join failed.".to_owned()) } }); }>
                    "Join room (Spade)"
                </button>
                <button on:click=move |_| connect(code.get().to_uppercase())>"Connect"</button>
                <p>{move || state.get().map(|cursor| format!("Active: {}{}", cursor.game().active_player(), if cursor.is_live() { "" } else { " — reviewing history" })).unwrap_or_default()}</p>
                {move || state.get().map(|cursor| board(cursor, selected, state, socket, notice)).unwrap_or_else(|| view! { <p>"No game state."</p> }.into_any())}
                <button on:click=move |_| send(state, socket, notice, GameDelta::Operation(Operation::End))>"End turn"</button>
                <button on:click=move |_| send(state, socket, notice, GameDelta::Undo)>"Undo"</button>
                <button on:click=move |_| state.update(|cursor| if let Some(cursor) = cursor { cursor.step_back_batch(); })>"← Batch"</button>
                <button on:click=move |_| state.update(|cursor| if let Some(cursor) = cursor { cursor.step_forward_batch(); })>"Batch →"</button>
            </main>
        }
    }

    fn send(
        state: RwSignal<Option<HistoryCursor>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
        delta: GameDelta,
    ) {
        let (Some(cursor), Some(ws)) = (state.get(), socket.get()) else {
            notice.set("Join a room first.".to_owned());
            return;
        };
        if !cursor.is_live() {
            notice.set("Return to live history before playing.".to_owned());
            return;
        }
        if let Ok(text) = serde_json::to_string(&ClientMessage::SubmitDelta {
            known_sequence: cursor.game().sequence(),
            delta,
        }) {
            let _ = ws.send_with_str(&text);
        }
    }

    fn board(
        cursor: HistoryCursor,
        selected: RwSignal<Option<Card>>,
        state: RwSignal<Option<HistoryCursor>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
    ) -> AnyView {
        let game = cursor.game();
        let live = cursor.is_live();
        let cols = game
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
            let mut cards_at_square = game
                .board()
                .cards()
                .filter_map(|(card, _)| {
                    (game.board().square_of(card).ok() == Some(square)).then_some(card)
                })
                .collect::<Vec<_>>();
            cards_at_square.sort_by_key(|card| pile_depth(game, *card));
            let cards = cards_at_square.into_iter().enumerate().map(|(index, card)| {
                let status = game.board().card_state(card).map(|state| state.status.clone()).unwrap_or(CardStatus::Idle);
                let class = format!("playing-card {} {}{}", if card.owner == Owner::Heart { "heart" } else { "spade" }, status_name(&status), if selected.get() == Some(card) { " selected" } else { "" });
                view! { <button class=class style=format!("left:{}px;z-index:{}", index * 18, index + 1) on:click=move |event: web_sys::MouseEvent| { event.stop_propagation(); if !live { return; } if let Some(mover) = selected.get() { selected.set(None); send(state, socket, notice, GameDelta::Operation(Operation::Intention { card: mover, target: Support::Card(card) })); } else { match status { CardStatus::Idle => send(state, socket, notice, GameDelta::Operation(Operation::Root(card))), CardStatus::Evicted => selected.set(Some(card)), CardStatus::Intended(_) => send(state, socket, notice, GameDelta::Operation(Operation::Fulfilment(card))) } } }><span>{format!("{}{}", if card.owner == Owner::Heart { "♥" } else { "♠" }, card.rank.display())}</span></button> }
            }).collect_view();
            view! { <div class="board-square" style=format!("grid-column:{};grid-row:{}", square.column + 1, square.row + 1) on:click=move |_| { if live { if let Some(mover) = selected.get() { selected.set(None); send(state, socket, notice, GameDelta::Operation(Operation::Intention { card: mover, target: Support::Base(square) })); } } }><small>{format!("{},{}", square.column, square.row)}</small><div class="pile">{cards}</div></div> }
        }).collect_view();
        view! { <div class=if blocked { "board blocked" } else { "board" } tabindex="0" style=format!("grid-template-columns:repeat({},minmax(7rem,1fr));grid-template-rows:repeat({},7rem)", cols, rows) on:keydown=move |event: KeyboardEvent| { match event.key().as_str() { "Backspace" => { event.prevent_default(); send(state, socket, notice, GameDelta::Undo); }, "Enter" => { event.prevent_default(); send(state, socket, notice, GameDelta::Operation(Operation::End)); }, "ArrowLeft" => { event.prevent_default(); state.update(|cursor| if let Some(cursor) = cursor { cursor.step_back_batch(); }); selected.set(None); }, "ArrowRight" => { event.prevent_default(); state.update(|cursor| if let Some(cursor) = cursor { cursor.step_forward_batch(); }); selected.set(None); }, _ => {} } }>{squares}</div> }.into_any()
    }

    fn status_name(status: &CardStatus) -> &'static str {
        match status {
            CardStatus::Idle => "idle",
            CardStatus::Evicted => "evicted",
            CardStatus::Intended(_) => "intended",
        }
    }

    fn pile_depth(game: &Game, card: Card) -> usize {
        let mut depth = 0;
        let mut support = game.board().card_state(card).map(|state| state.support);
        while let Some(Support::Card(lower)) = support {
            depth += 1;
            support = game.board().card_state(lower).map(|state| state.support);
        }
        depth
    }
}
#[cfg(target_arch = "wasm32")]
fn main() {
    client::run();
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {}
