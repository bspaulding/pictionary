use listenfd::ListenFd;
use actix::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, middleware::Logger};
use actix_web_actors::ws;
use env_logger;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
#[rtype(result = "CreateRoomResponse")]
struct CreateRoom;

struct PictionaryWebSocketSession {
    id: SessionId,
    room_id: String,
    addr: Addr<PictionaryServer>
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
                    match response {
                        WebSocketConnectedResult::JoinedRoom { id } => {
                            actor.id = id;
                        },
                        WebSocketConnectedResult::RoomNotFound => {
                            context.text(serde_json::to_string(&response).unwrap());
                            context.stop();
                        }
                    }
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
enum WsEvent {
    PointCreated(Point),
    PathClosed,
    SkipWordStart,
    SkipWordCompleted(String),
    PathSet(Vec<Vec<Point>>),
    NewWord(String),
    NextRound,
}

#[derive(Debug, Serialize, Message)]
#[rtype(result = "()")]
struct WsRoomEvent {
    room_id: String,
    session_id: SessionId,
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
                    session_id: self.id,
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
    ws::start(PictionaryWebSocketSession {
        id: Uuid::new_v4(),
        room_id: info.to_string(),
        addr: server.get_ref().clone(),
    }, &req, stream)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Clone, Debug, Message, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[rtype(result = "()")]
struct PictionaryModel {
    current_word: String,
    used_words: Vec<String>,
    paths: Vec<Vec<Point>>,
    words: PictionaryWords,
    current_session_id: Option<SessionId>,
}

impl Default for PictionaryModel {
    fn default() -> PictionaryModel {
        let mut words = PictionaryWords::default();
        let current_word = words.easy.pop().unwrap();
        PictionaryModel {
            current_word,
            used_words: vec![],
            paths: vec![vec![]],
            words,
            current_session_id: None
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
struct PictionaryWords {
    easy: Vec<String>,
    hard: Vec<String>
}

fn strs_to_strings(xs: Vec<&str>) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

impl Default for PictionaryWords {
    fn default() -> PictionaryWords {
        let mut rng = rand::thread_rng();
        let mut easy = strs_to_strings(vec!["Swing","Coat","Shoe","Ocean","Dog","Mouth","Milk","Duck","Skateboard","Bird","Mouse","Whale","Jacket","Shirt","Hippo","Beach","Egg","Cookie","Cheese","Skip","Drum","homework","glue","eraser","peace","panic","alarm","far","comfy","dripping","boring","hot","cold","parents","closet","laugh","falling","sleepover","calendar","sunscreen","panda","detention","hair","ice skating","afraid","dictionary","homerun","root beer float","hibernation","street sweeper","spitball","drinking fountain","imagination","Angry","Fireworks","Pumpkin","Baby","Flower","Rainbow","Beard","Flying saucer","Recycle","Bible","Giraffe","Sand castle","Bikini","Glasses","Snowflake","Book","High heel","Stairs","Bucket","Ice cream cone","Starfish","Bumble bee","Igloo","Strawberry","Butterfly","Lady bug","Sun","Camera","Lamp","Tire","Cat","Lion","Toast","Church","Mailbox","Toothbrush","Crayon","Night","Toothpaste","Dolphin","Nose","Truck","Egg","Olympics","Volleyball","Eiffel Tower","Peanut","half cardboard","oar","baby-sitter","drip","shampoo","point","time machine","yardstick","think","lace darts","world","avocado bleach","shower","curtain","extension cord dent","birthday lap","sandbox","bruise","quicksand","fog","gasoline","pocket","honk","sponge","rim","bride","wig","zipper","wag","letter opener","fiddle","water buffalo","pilot","brand pail","baguette","rib mascot","fireman","pole zoo sushi","fizz ceiling","fan bald","banister punk","post office","season","Internet","chess","puppet","chime","ivy"]);
        easy.shuffle(&mut rng);
        let mut hard = strs_to_strings(vec!["applause","application","avocato","award","badge","baggage","baker","barber","bargain","basket","bedbug","bettle","beggar","birthday","biscuit","bleach","blinds","bobsled","Bonnet","bookend","boundary","brain","bruise","bubble"]);
        hard.shuffle(&mut rng);
        PictionaryWords {
            easy,
            hard
        }
    }
}

type SessionId = Uuid;
struct PictionaryServer {
    sessions_by_id: HashMap<SessionId, Recipient<WsEvent>>,
    sessions_by_room_id: HashMap<String, Vec<SessionId>>, // room id / session ids
    models_by_room_id: HashMap<String, PictionaryModel>,
}

impl Default for PictionaryServer {
    fn default() -> PictionaryServer {
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
enum WebSocketConnectedResult {
    JoinedRoom { id: SessionId },
    RoomNotFound
}

impl<A, M> actix::dev::MessageResponse<A, M> for WebSocketConnectedResult
where
    A: Actor,
    M: Message<Result = WebSocketConnectedResult>,
{
    fn handle(self, ctx: &mut A::Context, tx: Option<actix::dev::OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}


impl Handler<WebSocketConnected> for PictionaryServer {
    type Result = WebSocketConnectedResult;

    fn handle(&mut self, msg: WebSocketConnected, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined room with id: {:?}", msg.room_id);
        // TODO: send player joined message to all sessions in room
        let id = Uuid::new_v4();
        self.sessions_by_id.insert(id, msg.addr);
        match self.sessions_by_room_id.get_mut(&msg.room_id) {
            Some(sessions) => {
                sessions.push(id);
                let model = self.models_by_room_id.get_mut(&msg.room_id).unwrap();
                let session = self.sessions_by_id.get(&id).unwrap();
                if sessions.len() == 1 {
                    model.current_session_id = Some(id);
                    session.do_send(WsEvent::NewWord(model.current_word.clone()));
                }
                session.do_send(WsEvent::PathSet(model.paths.clone()));
                WebSocketConnectedResult::JoinedRoom { id }
            }
            None => WebSocketConnectedResult::RoomNotFound
        }
    }
}

impl Handler<WebSocketDisconnected> for PictionaryServer {
    type Result = ();

    fn handle(&mut self, msg: WebSocketDisconnected, _: &mut Self::Context) {
        println!("Someone disconnected");

        let mut rooms_to_remove: Vec<String> = vec![];
        if self.sessions_by_id.remove(&msg.id).is_some() {
            for (room_id, sessions) in &mut self.sessions_by_room_id {
                match sessions.iter().position(|&x| x == msg.id) {
                    Some(index) => { sessions.remove(index); },
                    None => ()
                }
                // TODO: maybe send message to room about disconnect
                if sessions.is_empty() {
                    rooms_to_remove.push(room_id.clone());
                }
            }
        }
        for room_id in rooms_to_remove {
            self.sessions_by_room_id.remove(&room_id);
            self.models_by_room_id.remove(&room_id);
        }
    }
}

impl<A, M> actix::dev::MessageResponse<A, M> for CreateRoomResponse
where
    A: Actor,
    M: Message<Result = CreateRoomResponse>,
{
    fn handle(self, ctx: &mut A::Context, tx: Option<actix::dev::OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

impl Handler<CreateRoom> for PictionaryServer {
    type Result = CreateRoomResponse;

    fn handle(&mut self, _msg: CreateRoom, _: &mut Self::Context) -> Self::Result {
        let letters: Vec<char> = vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'];
        let mut rng = rand::thread_rng();
        let mut room: String = letters.choose_multiple(&mut rng, 4).collect::<String>();
        while self.sessions_by_room_id.contains_key(&room) {
            room = letters.choose_multiple(&mut rng, 4).collect::<String>();
        }
        println!("Creating room {}...", room);

        self.sessions_by_room_id.insert(room.clone(), vec![]);
        let model = PictionaryModel::default();
        self.models_by_room_id.insert(room.clone(), model.clone());
        CreateRoomResponse {
            room,
            model
        }
    }
}

impl Handler<WsRoomEvent> for PictionaryServer {
    type Result = ();

    fn handle(&mut self, msg: WsRoomEvent, _: &mut Self::Context) {
        let model = self.models_by_room_id.get_mut(&msg.room_id).unwrap();
        let mut owner_responses = vec![];
        let mut responses = vec![];
        let session_ids: &Vec<SessionId> = self.sessions_by_room_id.get(&msg.room_id).unwrap();
        match &msg.event {
            WsEvent::PointCreated(point) => {
                model.paths.last_mut().unwrap().push(point.clone());
            },
            WsEvent::PathClosed => {
                model.paths.push(vec![]);
            },
            WsEvent::SkipWordStart => {
                model.current_word = model.words.easy.pop().unwrap_or_else(|| {
                    model.words = PictionaryWords::default();
                    model.words.easy.pop().unwrap()
                });
                owner_responses.push(WsEvent::SkipWordCompleted(model.current_word.clone()));

                model.paths.clear();
                model.paths.push(vec![]);
                responses.push(WsEvent::PathSet(model.paths.clone()));
            },
            WsEvent::NextRound => {
                // Pick a new drawing player
                let current_session_index = session_ids.iter().position(|&x| x == model.current_session_id.unwrap()).unwrap();
                let next_session_index = if current_session_index == session_ids.len() - 1 {
                    0
                } else {
                    current_session_index + 1
                };
                model.current_session_id = Some(session_ids[next_session_index]);

                // pick a new word
                model.current_word = model.words.easy.pop().unwrap_or_else(|| {
                    model.words = PictionaryWords::default();
                    model.words.easy.pop().unwrap()
                });
                owner_responses.push(WsEvent::NewWord(model.current_word.clone()));

                // clear the paths
                model.paths.clear();
                model.paths.push(vec![]);
                responses.push(WsEvent::PathSet(model.paths.clone()));
            },
            _ => ()
        }
        for session_id in session_ids {
            if let Some(addr) = self.sessions_by_id.get(session_id) {
                if session_id != &msg.session_id {
                    addr.do_send(msg.event.clone());
                }
                for response in responses.iter() {
                    addr.do_send(response.clone());
                }
                if session_id == &model.current_session_id.unwrap() {
                    for response in owner_responses.iter() {
                        addr.do_send(response.clone());
                    }
                }
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
    let response: CreateRoomResponse = server.send(CreateRoom).await.unwrap();
    HttpResponse::Ok().json(response)
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
            .service(fs::Files::new("/", "../frontend/dist").index_file("index.html"))
    });

    let port = match std::env::var("PORT") {
        Ok(port) => port,
        Err(_) => "3000".to_string()
    };
    println!("Binding to port {}", port);
    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind(format!("0.0.0.0:{}", port))?
    };

    server.run().await
}
