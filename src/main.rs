// #![feature(proc_macro_hygiene, decl_macro)]

// #[macro_use]
extern crate redis;

fn main() {
    let client = redis::Client::open("redis://127.0.0.1/").expect("Could not connect");
    let mut con = client.get_connection().expect("Could not connect");

    let _ : redis::Value = redis::cmd("SET").arg("my_key").arg(42i32).query(&mut con).expect("Could not SET my_key to redis database");
    let val : String = redis::cmd("GET").arg("my_key").query(&mut con).expect("Could not GET my_key from redis database");
    print!("{}\n", val);


    let _ : redis::Value = redis::cmd("XADD").arg("my_stream").arg("*").arg("my_key").arg("my_val").query(&mut con).expect("XADD failed");
    let query : redis::Value = redis::cmd("XRANGE").arg("my_stream").arg("-").arg("+").query(&mut con).expect("XADD failed");
    //NOTE: XRANGE does not "consume" messages from the stream, so every time the above code is run query will get longer, because my_stream will have all the unconsumed messages from the previous runs

    print!("{:?}\n", query);
    // let query_as_str : String = redis::from_redis_value(&query).expect("Could not interpret redis stream query as a rust string");//This will cause the written error if run, because the returned query is not compatible with the string type! query, roughly speaking, is more of a multidimensional list than a string
    // print!("{}\n", query_as_str);
}
