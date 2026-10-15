#[cfg(target_arch = "wasm32")]
mod client {
    use gloo_net::http::Request;
    use leptos::prelude::*;
    use serde::Deserialize;
    use traffic_core::{
        Adjacency, Direction, GameDesign, MatchSnapshot, PlayerId, ServerEvent, replay,
    };
    use traffic_network::{ClientMessage, LogSequence, ServerMessage};
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{MessageEvent, WebSocket};

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
        let connect = move |room: String| {
            let Some(host) = web_sys::window().and_then(|w| w.location().host().ok()) else {
                return;
            };
            let Ok(ws) = WebSocket::new(&format!("ws://{host}/ws/{room}")) else {
                notice.set("WebSocket connection failed.".to_owned());
                return;
            };
            let state = replica;
            let message_notice = notice;
            let handler = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                let Some(text) = event.data().as_string() else {
                    return;
                };
                let Ok(message) = serde_json::from_str::<ServerMessage>(&text) else {
                    return;
                };
                match message {
                    ServerMessage::ReplayBootstrap { design, events } => {
                        match replay(&design, &events) {
                            Ok(snapshot) => state.set(Some(Replica {
                                design,
                                log: events,
                                snapshot,
                            })),
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
                                        replica.snapshot = snapshot
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
                <h1>"Traffic in Hanoi"</h1>
                <p>{move || notice.get()}</p>
                <button on:click=move |_| {
                    let code = code;
                    let notice = notice;
                    spawn_local(async move {
                        match Request::post("/api/rooms").send().await {
                            Ok(response) => match response.json::<RoomResponse>().await {
                                Ok(room) => {
                                    code.set(room.code.clone());
                                    notice.set(format!("Created {} as player {}.", room.code, room.player.index()));
                                }
                                Err(_) => notice.set("Room creation failed.".to_owned()),
                            },
                            Err(_) => notice.set("Room creation failed.".to_owned()),
                        }
                    });
                }>"Create room"</button>
                <input placeholder="room code" prop:value=move || code.get()
                    on:input=move |event| code.set(event_target_value(&event)) />
                <button on:click=move |_| {
                    let room = code.get().to_uppercase();
                    let notice = notice;
                    spawn_local(async move {
                        match Request::post(&format!("/api/rooms/{room}/join")).send().await {
                            Ok(response) => match response.json::<RoomResponse>().await {
                                Ok(room) => notice.set(format!("Joined {} as player {}. Connect now.", room.code, room.player.index())),
                                Err(_) => notice.set("Join failed.".to_owned()),
                            },
                            Err(_) => notice.set("Join failed.".to_owned()),
                        }
                    });
                }>"Join room"</button>
                <button on:click=move |_| connect(code.get().to_uppercase())>"Connect"</button>
                {move || replica.get().map(|value| board(value, replica, socket, notice))
                    .unwrap_or_else(|| view! { <p>"No game state."</p> }.into_any())}
            </main>
        }
    }
    fn send(
        replica: RwSignal<Option<Replica>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
        event: ServerEvent,
    ) {
        let (Some(replica), Some(socket)) = (replica.get(), socket.get()) else {
            notice.set("Connect first.".to_owned());
            return;
        };
        let known = replica
            .log
            .len()
            .checked_sub(1)
            .map(|i| LogSequence::new(i as u64));
        if let Ok(text) = serde_json::to_string(&ClientMessage::SubmitEvent {
            known_through: known,
            event,
        }) {
            let _ = socket.send_with_str(&text);
        }
    }
    fn board(
        replica: Replica,
        state: RwSignal<Option<Replica>>,
        socket: RwSignal<Option<WebSocket>>,
        notice: RwSignal<String>,
    ) -> AnyView {
        let snapshot = replica.snapshot;
        let design = replica.design;
        let squares = snapshot.board().squares().map(|square| {
            let cards = snapshot.board().pile(square).iter().map(|card| format!("P{}-{}", card.owner.index(), card.rank.value())).collect::<Vec<_>>().join(" ");
            let moving = matches!(snapshot.board().state(square), traffic_core::SquareState::Moving(_));
            let up = Adjacency::new(square, Direction::Up).ok();
            let down = Adjacency::new(square, Direction::Down).ok();
            let left = Adjacency::new(square, Direction::Left).ok();
            let right = Adjacency::new(square, Direction::Right).ok();
            view! {
                <section>
                    <strong>{square.to_string()}</strong>
                    <span>{format!(" {cards}")}</span>
                    <span>{format!(" {:?} / {:?}", snapshot.board().state(square), snapshot.status(&design, square))}</span>
                    <button disabled=move || moving || up.is_none() on:click=move |_| if let Some(value) = up { send(state, socket, notice, ServerEvent::Intend(value)) }>"↑"</button>
                    <button disabled=move || moving || down.is_none() on:click=move |_| if let Some(value) = down { send(state, socket, notice, ServerEvent::Intend(value)) }>"↓"</button>
                    <button disabled=move || moving || left.is_none() on:click=move |_| if let Some(value) = left { send(state, socket, notice, ServerEvent::Intend(value)) }>"←"</button>
                    <button disabled=move || moving || right.is_none() on:click=move |_| if let Some(value) = right { send(state, socket, notice, ServerEvent::Intend(value)) }>"→"</button>
                    <button disabled=move || !moving on:click=move |_| send(state, socket, notice, ServerEvent::Fulfil(square))>"Fulfil"</button>
                </section>
            }
        }).collect_view();
        view!{<div><p>{format!("Current player: {} | active: {:?} | settled: {}",snapshot.current_player().index(),snapshot.active_card(),snapshot.is_settled(&design))}</p>{squares}<button on:click=move|_|send(state,socket,notice,ServerEvent::EndTurn)>"End turn"</button><button on:click=move|_|send(state,socket,notice,ServerEvent::Undo)>"Undo"</button></div>}.into_any()
    }
}
#[cfg(target_arch = "wasm32")]
fn main() {
    client::run()
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {}
