// #![feature(proc_macro_hygiene, decl_macro)]

// #[macro_use]
extern crate redis;
// extern crate rocket;

fn main() {
    let client = redis::Client::open("redis://127.0.0.1/").expect("could not connect");
    let mut con = client.get_connection().expect("could not connect");

    // let _ : redis::Value = redis::cmd("SET").arg("my_key").arg(42i32).query(&mut con).expect("SET failed");
    let val : String = redis::cmd("GET").arg("my_key").query(&mut con).expect("GET failed");
    print!("{}", val);
}
