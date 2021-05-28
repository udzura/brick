#![feature(proc_macro_hygiene, decl_macro)]

extern crate rocket;

extern crate serde;

use crossbeam::channel;
use rocket::*;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};

use std::io::prelude::*;
use std::io::BufReader;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread,
};

#[derive(Serialize, Default, Debug)]
struct Ret {
    status: i32,
    msg: Option<String>,
}

#[derive(Serialize, Default, Debug)]
struct RetEnqueue {
    status: i32,
    id: Option<u32>,
    msg: Option<String>,
}

#[derive(Serialize, Default, Debug)]
struct RetStatus {
    status: i32,
    id: u32,
    running: bool,
    cmd_result: Option<u32>,
    stdout: String,
}

#[derive(Deserialize, Default, Debug)]
struct BodyEnqueue {
    cmd: String,
}

#[derive(Debug, Clone, Default)]
struct Message {
    id: u32,
    cmd: String,
}

struct ChanManager {
    pub retval: Arc<RwLock<HashMap<u32, Mutex<String>>>>,
    pub sender: Arc<channel::Sender<Message>>,
}

#[get("/")]
fn index() -> Json<Ret> {
    Json(Ret {
        status: 200,
        msg: "¡Hola!".to_string().into(),
    })
}

#[post("/enqueue", data = "<body>")]
fn enqueue(mng: State<ChanManager>, body: Json<BodyEnqueue>) -> Json<RetEnqueue> {
    use rand::Rng;

    let id = rand::thread_rng().gen();
    let msg = Message {
        id,
        cmd: body.cmd.clone(),
    };

    let sender = mng.sender.clone();
    match sender.send(msg) {
        Ok(_) => Json(RetEnqueue {
            status: 200,
            id: id.into(),
            msg: "Enqueued".to_string().into(),
        }),
        Err(e) => Json(RetEnqueue {
            status: 503,
            msg: format!("Error: {:?}", e).into(),
            ..Default::default()
        }),
    }
}

#[get("/status/<id>")]
fn status(mng: State<ChanManager>, id: u32) -> Json<Ret> {
    let retval = mng.retval.clone();
    let x = match retval.read() {
        Ok(map) => {
            let got = map.get(&id);
            Json(Ret {
                status: 200,
                msg: format!("Got: {:?}", got).into(),
            })
        }
        Err(e) => Json(Ret {
            status: 503,
            msg: format!("Error: {:?}", e).into(),
        }),
    };
    x
}

fn main() {
    let (s, r) = channel::bounded::<Message>(256);
    let store = Arc::new(RwLock::new(HashMap::new()));

    let mng = ChanManager {
        retval: store.clone(),
        sender: Arc::new(s),
    };
    thread::spawn(move || loop {
        // clone store in each iteration
        let store = store.clone();
        match r.recv() {
            Ok(got) => {
                println!("Got: {:?}", got);
                thread::spawn(move || {
                    use std::process::{Command, Stdio};

                    let mut process = match Command::new("bash")
                        .arg("-c")
                        .arg(&got.cmd)
                        .stdout(Stdio::piped())
                        .spawn()
                    {
                        Err(why) => panic!("couldn't spawn: {:?}", why),
                        Ok(process) => process,
                    };

                    let fd = process.stdout.as_ref().unwrap().as_raw_fd();
                    let io = unsafe { std::fs::File::from_raw_fd(fd) };
                    let mut reader = BufReader::new(io);

                    loop {
                        let mut line = String::new();
                        match reader.read_line(&mut line) {
                            Ok(len) => {
                                if len > 0 {
                                    let content = {
                                        let map = store.read().expect("RwLock poisoned");
                                        match map.get(&got.id) {
                                            None => line.clone(),
                                            Some(existing) => {
                                                let existing =
                                                    existing.lock().expect("Mutex poisoned");
                                                format!("{}{}", existing, line)
                                            }
                                        }
                                    };
                                    let mut map = store.write().expect("RwLock poisoned");
                                    *map.entry(got.id)
                                        .or_insert_with(|| Mutex::new("".to_string())) =
                                        Mutex::new(content);
                                }
                            }
                            Err(e) => {
                                println!("error attempting to wait: {:?}", e);
                                break;
                            }
                        }

                        match process.try_wait() {
                            Ok(Some(status)) => {
                                println!("command {} exited with: {}", got.cmd, status);
                                break;
                            }
                            Ok(None) => {
                                continue;
                            }
                            Err(e) => {
                                println!("error attempting to wait: {:?}", e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => println!("Some error: {:?}", e),
        };
    });

    rocket::ignite()
        .mount("/", routes![index, enqueue, status])
        .manage(mng)
        .launch();
}
