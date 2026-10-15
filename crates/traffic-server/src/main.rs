use std::{collections::HashMap, net::SocketAddr, sync::Arc};
mod session;
use axum::{
    Json, Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use session::SessionId;
use tokio::sync::{Mutex, mpsc, oneshot};
use tower_http::services::ServeDir;
use traffic_core::{GameDesign, MatchSnapshot, PlayerId, ServerEvent, replay};
use traffic_network::{ClientMessage, LogSequence, RejectionReason, SequencedEvent, ServerMessage};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    rooms: Arc<Mutex<HashMap<String, RoomHandle>>>,
    trace: bool,
}
#[derive(Clone)]
struct RoomHandle {
    sender: mpsc::Sender<RoomCommand>,
}
enum RoomCommand {
    Join {
        session: SessionId,
        reply: oneshot::Sender<Result<PlayerId, String>>,
    },
    Subscribe {
        session: SessionId,
        outgoing: mpsc::UnboundedSender<ServerMessage>,
    },
    Message {
        session: SessionId,
        message: ClientMessage,
        outgoing: mpsc::UnboundedSender<ServerMessage>,
    },
}
#[derive(Serialize)]
struct RoomResponse {
    code: String,
    player: PlayerId,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        rooms: Arc::new(Mutex::new(HashMap::new())),
        trace: std::env::args().any(|a| a == "--trace-protocol"),
    };
    let app = Router::new()
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/{code}/join", post(join_room))
        .route("/ws/{code}", get(websocket))
        .route("/health", get(|| async { "ok" }))
        .fallback_service(ServeDir::new("crates/traffic-server/static"))
        .with_state(state);
    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Traffic demo server: http://{address}");
    axum::serve(tokio::net::TcpListener::bind(address).await.unwrap(), app)
        .await
        .unwrap();
}
async fn create_room(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (HeaderMap, Json<RoomResponse>) {
    let (session, cookie) = session::establish(&headers);
    let code = room_code();
    let handle = spawn_room(session, state.trace);
    state.rooms.lock().await.insert(code.clone(), handle);
    (
        session::cookie_header(cookie),
        Json(RoomResponse {
            code,
            player: GameDesign::standard().player_id_at(0).unwrap(),
        }),
    )
}
async fn join_room(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<RoomResponse>), (StatusCode, String)> {
    let (session, cookie) = session::establish(&headers);
    let handle = state
        .rooms
        .lock()
        .await
        .get(&code)
        .cloned()
        .ok_or((StatusCode::NOT_FOUND, "unknown room".to_owned()))?;
    let (tx, rx) = oneshot::channel();
    handle
        .sender
        .send(RoomCommand::Join { session, reply: tx })
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "room stopped".to_owned()))?;
    let player = rx
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "room stopped".to_owned()))?
        .map_err(|e| (StatusCode::CONFLICT, e))?;
    Ok((
        session::cookie_header(cookie),
        Json(RoomResponse { code, player }),
    ))
}
async fn websocket(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode> {
    let session = session::existing(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let handle = state
        .rooms
        .lock()
        .await
        .get(&code)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(upgrade.on_upgrade(move |socket| serve_socket(socket, handle, session)))
}
async fn serve_socket(socket: WebSocket, handle: RoomHandle, session: SessionId) {
    let (mut writer, mut reader) = socket.split();
    let (outgoing, mut incoming) = mpsc::unbounded_channel();
    if handle
        .sender
        .send(RoomCommand::Subscribe {
            session: session.clone(),
            outgoing: outgoing.clone(),
        })
        .await
        .is_err()
    {
        return;
    }
    loop {
        tokio::select! {
            Some(message) = incoming.recv() => {
                if let Ok(text) = serde_json::to_string(&message)
                    && writer.send(Message::Text(text.into())).await.is_err()
                {
                    break;
                }
            }
            message = reader.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(message) = serde_json::from_str(&text) {
                        let _ = handle.sender.send(RoomCommand::Message {
                            session: session.clone(),
                            message,
                            outgoing: outgoing.clone(),
                        }).await;
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
}
fn spawn_room(first: SessionId, trace: bool) -> RoomHandle {
    let (sender, mut receiver) = mpsc::channel(64);
    tokio::spawn(async move {
        let design = GameDesign::standard();
        let mut current = MatchSnapshot::new(&design);
        let mut log = Vec::<ServerEvent>::new();
        let mut second = None;
        let mut subscribers = Vec::<mpsc::UnboundedSender<ServerMessage>>::new();
        while let Some(command) = receiver.recv().await {
            let player = |session: &SessionId| {
                if session == &first {
                    design.player_id_at(0)
                } else if second.as_ref() == Some(session) {
                    design.player_id_at(1)
                } else {
                    None
                }
            };
            match command {
                RoomCommand::Join { session, reply } => {
                    let result = if session == first {
                        Ok(design.player_id_at(0).unwrap())
                    } else if second.as_ref() == Some(&session) {
                        Ok(design.player_id_at(1).unwrap())
                    } else if second.is_none() {
                        second = Some(session);
                        Ok(design.player_id_at(1).unwrap())
                    } else {
                        Err("room already has two players".to_owned())
                    };
                    let _ = reply.send(result);
                }
                RoomCommand::Subscribe { session, outgoing } => {
                    if player(&session).is_some() {
                        subscribers.push(outgoing);
                    } else {
                        let _ = outgoing.send(rejected(&log, RejectionReason::Unauthorized));
                    }
                }
                RoomCommand::Message {
                    session,
                    message,
                    outgoing,
                } => match message {
                    ClientMessage::RequestReplayBootstrap => {
                        if player(&session).is_some() {
                            let _ = outgoing.send(ServerMessage::ReplayBootstrap {
                                design: design.clone(),
                                events: log.clone(),
                            });
                        } else {
                            let _ = outgoing.send(rejected(&log, RejectionReason::Unauthorized));
                        }
                    }
                    ClientMessage::CatchUpAfter { sequence } => {
                        if player(&session).is_none() {
                            let _ = outgoing.send(rejected(&log, RejectionReason::Unauthorized));
                            continue;
                        }
                        let start =
                            sequence.map_or(0, |value| value.value().saturating_add(1) as usize);
                        let events = log
                            .iter()
                            .enumerate()
                            .skip(start)
                            .map(|(index, event)| SequencedEvent {
                                sequence: LogSequence::new(index as u64),
                                event: event.clone(),
                            })
                            .collect();
                        let _ = outgoing.send(ServerMessage::Events { events });
                    }
                    ClientMessage::SubmitEvent {
                        known_through,
                        event,
                    } => {
                        let Some(acting) = player(&session) else {
                            let _ = outgoing.send(rejected(&log, RejectionReason::Unauthorized));
                            continue;
                        };
                        if known_through != through(&log) {
                            let _ = outgoing.send(rejected(&log, RejectionReason::StaleSequence));
                            continue;
                        }
                        if acting != current.current_player() {
                            let _ = outgoing.send(rejected(&log, RejectionReason::Unauthorized));
                            continue;
                        }
                        let mut candidate = log.clone();
                        candidate.push(event);
                        match replay(&design, &candidate) {
                            Ok(next) => {
                                current = next;
                                log = candidate;
                                let index = log.len() - 1;
                                let update = ServerMessage::Events {
                                    events: vec![SequencedEvent {
                                        sequence: LogSequence::new(index as u64),
                                        event: log[index].clone(),
                                    }],
                                };
                                subscribers
                                    .retain(|subscriber| subscriber.send(update.clone()).is_ok());
                                if trace {
                                    eprintln!("accepted event {index}");
                                }
                            }
                            Err(_) => {
                                let _ =
                                    outgoing.send(rejected(&log, RejectionReason::InvalidEvent));
                            }
                        }
                    }
                },
            }
        }
    });
    RoomHandle { sender }
}
fn through(log: &[ServerEvent]) -> Option<LogSequence> {
    log.len().checked_sub(1).map(|i| LogSequence::new(i as u64))
}
fn rejected(log: &[ServerEvent], reason: RejectionReason) -> ServerMessage {
    ServerMessage::Rejected {
        current_through: through(log),
        reason,
    }
}
fn room_code() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}
