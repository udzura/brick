#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

extern crate serde;

use rocket::response::content;
use rocket_contrib::json::Json;
use serde::Serialize;

#[derive(Serialize)]
struct Ret {
    status: i32,
}

#[get("/")]
fn index() -> Json<Ret> {
    Json(Ret { status: 200 })
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
