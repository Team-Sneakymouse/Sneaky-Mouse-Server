//By Mami
#![allow(unused_variables)]
#![allow(unused_parens)]//why tf is this a default warning
extern crate redis;
extern crate getrandom;
extern crate rand_pcg;
extern crate rand;


mod config;
mod util;
mod event;
use event::*;
use rand::{Rng, RngCore, SeedableRng};
use std::time::{Instant, Duration};
use rand_pcg::*;


fn server_main() -> Option<bool> {
	let redis_address: String;
	match std::env::var("REDIS_ADDRESS") {
		Ok(val) => redis_address = val,
		Err(_) => redis_address = String::from("redis://127.0.0.1/"),
	}
	let con = util::connect_to(&redis_address[..])?;

	let mut server_state_mem = SneakyMouseServer{
		redis_con : con,
		redis_address : &redis_address[..],
		rng : Pcg64::from_entropy(),
		cur_time : 0.0,
		cheese_timeouts : Vec::new(),
		cheese_uids : Vec::new(),
		cheese_rooms : Vec::new(),
		cheese_ids : Vec::new(),
	};
	let mut trans_mem = Vec::new();
	let server_state = &mut server_state_mem;

	let events = get_event_list();

	let mut last_ids = Vec::<Vec<u8>>::new();//I wish I could jointly allocate these
	if !config::DEBUG_FLOOD_ALL_STREAMS {//get last ids from redis
		let mut cmd = redis::cmd("HMGET");
		cmd.arg(config::REDIS_LAST_ID_PREFIX);
		for event in events.iter() {
			cmd.arg(event);
		}

		let ids_data = util::auto_retry_cmd(server_state, &cmd)?;
		if let redis::Value::Bulk(ids) = ids_data {
			for id in ids {
				match id {
					redis::Value::Data(id_str) => last_ids.push(id_str),
					redis::Value::Nil => last_ids.push(config::REDIS_LAST_ID_DEFAULT.as_bytes().to_vec()),
					_ => util::mismatch_spec(server_state, file!(), line!())
				}
			}
		} else {
			util::mismatch_spec(server_state, file!(), line!())
		}
	} else {
		for _ in events.iter() {
			last_ids.push(b"0-0".to_vec());
		}
	}

	let mut last_time = Instant::now();

	let mut event_keys_mem : Vec<&[u8]> = Vec::<&[u8]>::new();
	let mut event_vals_mem : Vec<&[u8]> = Vec::<&[u8]>::new();
	loop {
		let cur_time = Instant::now();
		let delta : f64 = match cur_time.checked_duration_since(last_time) {
			Some(dur) => dur.as_secs_f64(),
			None => 0.0,
		};
		last_time = cur_time;
		let timeout = server_update(server_state, &mut trans_mem, delta)?;

		let opts = redis::streams::StreamReadOptions::default().count(config::REDIS_STREAM_READ_COUNT).block((timeout*1000.0) as usize);

		let mut cmd = redis::Cmd::xread_options(&events[..], &last_ids[..], &opts);
		let response : redis::Value = util::auto_retry_cmd(server_state, &mut cmd)?;


		if let redis::Value::Bulk(stream_responses) = &response {
		for stream_response_data in stream_responses {
			if let redis::Value::Bulk(stream_response) = &stream_response_data {
			if let redis::Value::Data(stream_name_raw) = &stream_response[0] {
			if let redis::Value::Bulk(stream_messages) = &stream_response[1] {
			for message_data in stream_messages {
				if let redis::Value::Bulk(message) = &message_data {
				if let redis::Value::Data(message_id_raw) = &message[0] {
				if let redis::Value::Bulk(message_body) = &message[1] {

				let mut event_keys = event_keys_mem;
				let mut event_vals = event_vals_mem;
				for i2 in 0..message_body.len()/2 {
					let i = i2*2;
					if let redis::Value::Data(message_key_raw) = &message_body[i] {
					if let redis::Value::Data(message_val_raw) = &message_body[i + 1] {

					let k = &message_key_raw[..];
					let v = &message_val_raw[..];
					event_keys.push(k);
					event_vals.push(v);

					} else {util::mismatch_spec(server_state, file!(), line!())}
					} else {util::mismatch_spec(server_state, file!(), line!())}
				}


				let i = events.binary_search(&&stream_name_raw[..]).expect("fatal error: we received an unrecognized event, how did this not get caught until now?");

				last_ids[i].clear();
				last_ids[i].extend(&message_id_raw[..]);//this avoids allocating

				if !config::DEBUG_FLOOD_ALL_STREAMS {
					let mut cmd = redis::cmd("HMSET");
					cmd.arg(config::REDIS_LAST_ID_PREFIX).arg(&stream_name_raw).arg(&last_ids[i]);

					if let None = util::auto_retry_cmd(server_state, &mut cmd) {
						util::send_error(server_state, &format!("the last consumed event was not saved! it had id {}\n", String::from_utf8_lossy(&last_ids[i])));
						return None;
					}
				}


				server_event_received(server_state, &stream_name_raw, message_id_raw, &event_keys[..], &event_vals[..], &mut trans_mem)?;

				//the borrow checker does not acknowledge that .clear() drops all borrowed references, so we have to force it to
				trans_mem.clear();
				event_keys.clear();
				event_vals.clear();
				event_keys_mem = unsafe {std::mem::transmute(event_keys)};
				event_vals_mem = unsafe {std::mem::transmute(event_vals)};

				} else {util::mismatch_spec(server_state, file!(), line!())}
				} else {util::mismatch_spec(server_state, file!(), line!())}
				} else {util::mismatch_spec(server_state, file!(), line!())}
			}
			} else {util::mismatch_spec(server_state, file!(), line!())}
			} else {util::mismatch_spec(server_state, file!(), line!())}
			} else {util::mismatch_spec(server_state, file!(), line!())}
		}
		} else if let redis::Value::Nil = response {
			print!("no events received before timeout: trying again...\n");
		}
	}
}

fn main() {
	server_main();
}
