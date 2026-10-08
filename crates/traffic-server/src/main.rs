use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    Json, Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::{Mutex, mpsc, oneshot};
use tower_http::services::ServeDir;
use traffic_core::{Game, GameCreated, Layout, Owner};
use traffic_network::{ClientMessage, RejectionReason, ServerMessage};
use uuid::Uuid;

type SessionId = String;

#[derive(Clone)]
struct AppState {
    rooms: Arc<Mutex<HashMap<String, RoomHandle>>>,
}

#[derive(Clone)]
struct RoomHandle {
    sender: mpsc::Sender<RoomCommand>,
}

enum RoomCommand {
    Join {
        session: SessionId,
        reply: oneshot::Sender<Result<Owner, String>>,
    },
    Subscribe {
        session: SessionId,
        outgoing: mpsc::UnboundedSender<ServerMessage>,
    },
    ClientMessage {
        session: SessionId,
        message: ClientMessage,
        outgoing: mpsc::UnboundedSender<ServerMessage>,
    },
}

#[derive(Serialize)]
struct RoomResponse {
    code: String,
    owner: Owner,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        rooms: Arc::new(Mutex::new(HashMap::new())),
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
    let (session, cookie) = session(&headers);
    let code = room_code();
    let handle = spawn_room(session);
    state.rooms.lock().await.insert(code.clone(), handle);
    (
        cookie_header(cookie),
        Json(RoomResponse {
            code,
            owner: Owner::Heart,
        }),
    )
}

async fn join_room(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<RoomResponse>), (StatusCode, String)> {
    let (session, cookie) = session(&headers);
    let handle = state
        .rooms
        .lock()
        .await
        .get(&code)
        .cloned()
        .ok_or((StatusCode::NOT_FOUND, "unknown room".to_owned()))?;
    let (reply, response) = oneshot::channel();
    handle
        .sender
        .send(RoomCommand::Join { session, reply })
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "room stopped".to_owned()))?;
    let owner = response
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "room stopped".to_owned()))?
        .map_err(|message| (StatusCode::CONFLICT, message))?;
    Ok((cookie_header(cookie), Json(RoomResponse { code, owner })))
}

async fn websocket(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(session) = existing_session(&headers) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
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
                let Ok(text) = serde_json::to_string(&message) else { continue };
                if writer.send(Message::Text(text.into())).await.is_err() { break; }
            }
            message = reader.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(message) = serde_json::from_str(&text) {
                        let _ = handle.sender.send(RoomCommand::ClientMessage { session: session.clone(), message, outgoing: outgoing.clone() }).await;
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
        }
    }
}

fn spawn_room(heart: SessionId) -> RoomHandle {
    let (sender, mut receiver) = mpsc::channel(64);
    tokio::spawn(async move {
        let created = GameCreated::new(Layout::standard(5).expect("standard layout is valid"));
        let mut game = Game::from_created(created.clone());
        let mut spade = None;
        let mut subscribers: Vec<mpsc::UnboundedSender<ServerMessage>> = Vec::new();
        while let Some(command) = receiver.recv().await {
            match command {
                RoomCommand::Join { session, reply } => {
                    let result = if session == heart {
                        Ok(Owner::Heart)
                    } else if spade.as_ref() == Some(&session) {
                        Ok(Owner::Spade)
                    } else if spade.is_none() {
                        spade = Some(session);
                        Ok(Owner::Spade)
                    } else {
                        Err("room already has two players".to_owned())
                    };
                    let _ = reply.send(result);
                }
                RoomCommand::Subscribe { session, outgoing } => {
                    if session == heart || spade.as_ref() == Some(&session) {
                        subscribers.push(outgoing);
                    } else {
                        let _ = outgoing.send(ServerMessage::Rejected {
                            current_sequence: game.sequence(),
                            reason: RejectionReason::Unauthorized,
                        });
                    }
                }
                RoomCommand::ClientMessage {
                    session,
                    message,
                    outgoing,
                } => match message {
                    ClientMessage::RequestReplayBootstrap => {
                        if session == heart || spade.as_ref() == Some(&session) {
                            let _ = outgoing.send(ServerMessage::ReplayBootstrap {
                                created: created.clone(),
                                events: game.events().to_vec(),
                            });
                        } else {
                            let _ = outgoing.send(ServerMessage::Rejected {
                                current_sequence: game.sequence(),
                                reason: RejectionReason::Unauthorized,
                            });
                        }
                    }
                    ClientMessage::CatchUpAfter { sequence } => {
                        if session == heart || spade.as_ref() == Some(&session) {
                            let events = game
                                .events()
                                .iter()
                                .filter(|event| event.sequence() > sequence)
                                .cloned()
                                .collect();
                            let _ = outgoing.send(ServerMessage::Events { events });
                        } else {
                            let _ = outgoing.send(ServerMessage::Rejected {
                                current_sequence: game.sequence(),
                                reason: RejectionReason::Unauthorized,
                            });
                        }
                    }
                    ClientMessage::SubmitDelta {
                        known_sequence,
                        delta,
                    } => {
                        let owner = if session == heart {
                            Some(Owner::Heart)
                        } else if spade.as_ref() == Some(&session) {
                            Some(Owner::Spade)
                        } else {
                            None
                        };
                        let rejection = if owner != Some(game.active_player()) {
                            Some(RejectionReason::Unauthorized)
                        } else if known_sequence != game.sequence() {
                            Some(RejectionReason::StaleSequence)
                        } else {
                            None
                        };
                        if let Some(reason) = rejection {
                            let _ = outgoing.send(ServerMessage::Rejected {
                                current_sequence: game.sequence(),
                                reason,
                            });
                        } else if let Ok((event, _)) = game.apply_delta(delta) {
                            let update = ServerMessage::Events {
                                events: vec![event],
                            };
                            subscribers
                                .retain(|subscriber| subscriber.send(update.clone()).is_ok());
                        } else {
                            let _ = outgoing.send(ServerMessage::Rejected {
                                current_sequence: game.sequence(),
                                reason: RejectionReason::InvalidDelta,
                            });
                        }
                    }
                },
            }
        }
    });
    RoomHandle { sender }
}

fn existing_session(headers: &HeaderMap) -> Option<SessionId> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            part.trim()
                .strip_prefix("traffic_session=")
                .map(ToOwned::to_owned)
        })
}

fn session(headers: &HeaderMap) -> (SessionId, Option<HeaderValue>) {
    if let Some(existing) = existing_session(headers) {
        return (existing, None);
    }
    let value = Uuid::new_v4().to_string();
    let cookie = HeaderValue::from_str(&format!(
        "traffic_session={value}; Path=/; HttpOnly; SameSite=Lax"
    ))
    .expect("valid cookie");
    (value, Some(cookie))
}

fn cookie_header(cookie: Option<HeaderValue>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(cookie) = cookie {
        headers.insert(SET_COOKIE, cookie);
    }
    headers
}

fn room_code() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}
