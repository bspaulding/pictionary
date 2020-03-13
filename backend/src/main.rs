use listenfd::ListenFd;
use actix_files as fs;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Serialize};

#[derive(Debug)]
struct Point {
    x: u8,
    y: u8,
}

#[derive(Debug)]
struct PictionaryModel {
    current_word: String,
    used_words: Vec<String>,
    points_drawn: Vec<Point>,
}

#[derive(Serialize)]
struct CreateRoomResponse {
    room: String
}

async fn create_room() -> impl Responder {
    HttpResponse::Ok().json(CreateRoomResponse {
        room: String::from("xkcd")
    })
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // let model = PictionaryModel {
    //     current_word: String::from("bulldozer"),
    //     used_words: vec![],
    //     points_drawn: vec![],
    // };
    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(|| {
        App::new()
            .route("/api/v1/rooms", web::post().to(create_room))
            .service(fs::Files::new("/", "../frontend/public").index_file("index.html"))
    });

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind("127.0.0.1:3000")?
    };

    server.run().await
}
