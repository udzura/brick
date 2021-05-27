#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

extern crate serde;

use crossbeam::channel;
use rocket::response::content;
use rocket::*;
use rocket_contrib::json::Json;
use serde::Serialize;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::Duration,
};

#[derive(Serialize, Default)]
struct Ret {
    status: i32,
    msg: Option<String>,
}

#[derive(Debug, Clone)]
struct Message {
    id: i32,
    cmd: String,
}

struct ChanManager {
    pub retval: Arc<RwLock<HashMap<u32, String>>>,
    pub sender: Arc<channel::Sender<Message>>,
}

#[get("/")]
fn index() -> Json<Ret> {
    Json(Ret {
        status: 200,
        msg: "¡Hola!".to_string().into(),
    })
}

#[get("/enqueue")]
fn enqueue(mng: State<ChanManager>) -> Json<Ret> {
    use rand::Rng;

    let id = rand::thread_rng().gen();
    let msg = Message {
        id,
        cmd: "uname -a".to_string(),
    };

    let sender = mng.sender.clone();
    match sender.send(msg) {
        Ok(_) => Json(Ret {
            status: 200,
            ..Default::default()
        }),
        Err(e) => Json(Ret {
            status: 503,
            msg: format!("Error: {:?}", e).into(),
        }),
    }
}

#[get("/status/<id>")]
fn status(id: u32) -> Json<Ret> {
    Json(Ret {
        status: 200,
        ..Default::default()
    })
}

fn main() {
    let (s, r) = channel::bounded::<Message>(256);

    let mng = ChanManager {
        retval: Arc::new(RwLock::new(HashMap::new())),
        sender: Arc::new(s),
    };

    thread::spawn(move || loop {
        match r.recv() {
            Ok(got) => println!("Got: {:?}", got),
            Err(e) => println!("Some error: {:?}", e),
        };
    });

    rocket::ignite()
        .mount("/", routes![index, enqueue, status])
        .manage(mng)
        .launch();
}
