use listenfd::ListenFd;
use actix::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, middleware::Logger};
use actix_web_actors::ws;
use env_logger;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Message)]
#[rtype(WebSocketConnectedResult)]
struct WebSocketConnected {
    addr: Recipient<WsEvent>,
    room_id: String,
}

#[derive(Message)]
#[rtype(result = "()")]
struct WebSocketDisconnected {
    id: SessionId
}

#[derive(Message)]
#[rtype(result = "PictionaryModel")]
struct CreateRoom {
    room: String
}

struct PictionaryWebSocketSession {
    id: SessionId,
    room_id: String,
    addr: Addr<PictionaryServer>
}

#[derive(Serialize)]
struct ModelStateAction {
    #[serde(rename="type")]
    event_type: String,
    payload: PictionaryModel
}

impl Actor for PictionaryWebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        println!("PictionaryWebSocketSession started");
        self.addr.send(WebSocketConnected {
            addr: context.address().recipient(),
            room_id: self.room_id.clone(),
        })
        .into_actor(self)
        .then(|response, actor, context| {
            match response {
                Ok(response) => {
                    actor.id = response.id;
                    context.text(serde_json::to_string(&ModelStateAction {
                        event_type: String::from("MODEL_STATE"),
                        payload: response.model
                    }).unwrap())
                },
                _ => context.stop()
            }
            fut::ready(())
        })
        .wait(context);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        println!("PictionaryWebSocketSession stopped");
        self.addr.do_send(WebSocketDisconnected { id: self.id });
        Running::Stop
    }
}

impl Handler<WsEvent> for PictionaryWebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: WsEvent, context: &mut Self::Context) {
        context.text(serde_json::to_string(&msg).unwrap());
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(rename_all="camelCase")]
#[rtype(result = "()")]
struct WsEvent {
    #[serde(rename = "type")]
    event_type: String,
    payload: Option<Point>
}

#[derive(Debug, Serialize, Message)]
#[rtype(result = "()")]
struct WsRoomEvent {
    room_id: String,
    event: WsEvent
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PictionaryWebSocketSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                self.addr.do_send(WsRoomEvent {
                    room_id: self.room_id.clone(),
                    event: serde_json::from_str(&text).unwrap()
                });
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(_)) => ctx.stop(),
            Err(_) => ctx.stop(),
            _ => (),
        }
    }
}

async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    server: web::Data<Addr<PictionaryServer>>,
    info: web::Path<String>,
) -> Result<HttpResponse, Error> {
    println!("Opening socket for room: {:?}", info);
    let resp = ws::start(PictionaryWebSocketSession {
        id: Uuid::new_v4(),
        room_id: info.to_string(),
        addr: server.get_ref().clone(),
    }, &req, stream);
    println!("{:?}", resp);
    resp
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Clone, Debug, Message, Serialize)]
#[serde(rename_all = "camelCase")]
#[rtype(result = "()")]
struct PictionaryModel {
    current_word: String,
    used_words: Vec<String>,
    paths: Vec<Vec<Point>>,
}

impl Default for PictionaryModel {
    fn default() -> PictionaryModel {
        PictionaryModel {
            current_word: String::from("monkey"),
            used_words: vec![],
            paths: vec![vec![]],
        }
    }
}

type SessionId = Uuid;
struct PictionaryServer {
    sessions_by_id: HashMap<SessionId, Recipient<WsEvent>>,
    sessions_by_room_id: HashMap<String, HashSet<SessionId>>, // room id / session ids
    models_by_room_id: HashMap<String, PictionaryModel>,
}

impl Default for PictionaryServer {
    fn default() -> PictionaryServer {
        println!("PictionaryServer#default");
        PictionaryServer {
            sessions_by_id: HashMap::new(),
            sessions_by_room_id: HashMap::new(),
            models_by_room_id: HashMap::new(),
        }
    }
}

impl Actor for PictionaryServer {
    type Context = Context<Self>;
}

#[derive(Serialize)]
struct WebSocketConnectedResult {
    id: SessionId,
    model: PictionaryModel,
}

impl<A, M> actix::dev::MessageResponse<A, M> for WebSocketConnectedResult
where
    A: Actor,
    M: Message<Result = WebSocketConnectedResult>,
{
    fn handle<R: actix::dev::ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}


impl Handler<WebSocketConnected> for PictionaryServer {
    type Result = WebSocketConnectedResult;

    fn handle(&mut self, msg: WebSocketConnected, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined room with id: {:?}", msg.room_id);
        // TODO: return error if room does not exist
        // TODO: send player joined message to all sessions in room
        let id = Uuid::new_v4();
        self.sessions_by_id.insert(id, msg.addr);
        self.sessions_by_room_id.get_mut(&msg.room_id).unwrap().insert(id);
        let model = self.models_by_room_id.get(&msg.room_id).unwrap().clone();
        WebSocketConnectedResult { id, model }
    }
}

impl Handler<WebSocketDisconnected> for PictionaryServer {
    type Result = ();

    fn handle(&mut self, msg: WebSocketDisconnected, _: &mut Self::Context) {
        println!("Someone disconnected");

        if self.sessions_by_id.remove(&msg.id).is_some() {
            for (_name, sessions) in &mut self.sessions_by_room_id {
                sessions.remove(&msg.id);
                // TODO: maybe send message to room about disconnect
            }
        }
    }
}

impl<A, M> actix::dev::MessageResponse<A, M> for PictionaryModel
where
    A: Actor,
    M: Message<Result = PictionaryModel>,
{
    fn handle<R: actix::dev::ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

impl Handler<CreateRoom> for PictionaryServer {
    type Result = PictionaryModel;

    fn handle(&mut self, msg: CreateRoom, _: &mut Self::Context) -> Self::Result {
        println!("Creating room {:?} ", msg.room);

        self.sessions_by_room_id.insert(msg.room.clone(), HashSet::new());
        let model = PictionaryModel::default();
        self.models_by_room_id.insert(msg.room.clone(), model.clone());
        model
    }
}

impl Handler<WsRoomEvent> for PictionaryServer {
    type Result = ();

    fn handle(&mut self, msg: WsRoomEvent, _: &mut Self::Context) {
        // TODO: don't re-send to the sender
        let model = self.models_by_room_id.get_mut(&msg.room_id).unwrap();
        match msg.event.event_type.as_ref() {
            "POINT_CREATED" => {
                model.paths.last_mut().unwrap().push(msg.event.clone().payload.unwrap());
            },
            "PATH_CLOSED" => {
                model.paths.push(vec![]);
            },
            _ => ()
        }
        let session_ids: &HashSet<SessionId> = self.sessions_by_room_id.get(&msg.room_id).unwrap();
        for session_id in session_ids {
            if let Some(addr) = self.sessions_by_id.get(session_id) {
                addr.do_send(msg.event.clone()).unwrap();
            }
        }
    }
}

#[derive(Serialize)]
struct CreateRoomResponse {
    room: String,
    model: PictionaryModel
}

async fn create_room(server: web::Data<Addr<PictionaryServer>>) -> impl Responder {
    let response: PictionaryModel = server.send(CreateRoom {
        room: String::from("xkcd")
    }).await.unwrap();
    HttpResponse::Ok().json(CreateRoomResponse {
        room: String::from("xkcd"),
        model: response
    })
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "my_errors=debug,actix_web=info");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    let pictionary_server = PictionaryServer::default().start();
    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .data(pictionary_server.clone())
            .route("/api/v1/rooms", web::post().to(create_room))
            .route("/api/v1/rooms/{roomId}/ws", web::get().to(ws_handler))
            .service(fs::Files::new("/", "../frontend/public").index_file("index.html"))
    });

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind("127.0.0.1:3000")?
    };

    server.run().await
}
