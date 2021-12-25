//By Mami
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_parens)]//why tf is this a default warning
extern crate redis;
extern crate rand_pcg;
extern crate rand;

mod config;
mod db;
mod com;
mod http_server;
mod util;
mod event;
use event::*;
use rand::{Rng, RngCore, SeedableRng};
use std::time::{Instant, Duration};
use rand_pcg::*;
use config::*;


fn server_main() -> Result<(), ()> {
	let redis_address_mem = match std::env::var("REDIS_ADDRESS") {
		Ok(v) => Some(v),
		Err(_) => None,
	};
	let redis_address = match &redis_address_mem {
		Some(v) => &v[..],
		None => "redis://127.0.0.1/",
	};
	let http_address_mem = match std::env::var("HTTP_ADDRESS") {
		Ok(v) => Some(v),
		Err(_) => None,
	};
	let http_address = match &http_address_mem {
		Some(v) => &v[..],
		None => "127.0.0.1:80",
	};

	let mut server_state = SneakyMouseServer{
		db: LayerData{
			redis_con : com::connect_to(redis_address)?,
			redis_address : redis_address,
			pipe: redis::Pipeline::new(),
		},
		rng : Pcg64::from_entropy(),
		cur_time : 0.0,
		cheese_timeouts : Vec::new(),
		cheese_uids : Vec::new(),
		cheese_rooms : Vec::new(),
		cheese_ids : Vec::new(),
	};
	let mut http_server_state = HTTPServer{
		addr: http_address,
		disabled: false,
		listener: None,
	};
	let mut trans_mem = Vec::new();

	let events = get_event_list();

	let mut last_ids = Vec::<Vec<u8>>::new();
	server_state.db.pipe.cmd("HMGET");
	server_state.db.pipe.arg(layer::key::LAST_ID);
	for event in events.iter() {
		server_state.db.pipe.arg(event);
	}

	if let redis::Value::Bulk(ids) = com::auto_retry_flush_pipe(&mut server_state.db)? {
		for id in ids {
			match id {
				redis::Value::Data(id_str) => last_ids.push(id_str),
				redis::Value::Nil => last_ids.push(layer::default::LAST_ID.as_bytes().to_vec()),
				_ => db::mismatch_spec(&mut server_state.db, file!(), line!())
			}
		}
	} else {
		db::mismatch_spec(&mut server_state.db, file!(), line!())
	}

	let mut last_time = Instant::now();

	let mut event_keys_mem : Vec<&[u8]> = Vec::<&[u8]>::new();
	let mut event_vals_mem : Vec<&[u8]> = Vec::<&[u8]>::new();
	loop {
		let delta : f64;
		{
			let cur_time = Instant::now();
			delta = match cur_time.checked_duration_since(last_time) {
				Some(dur) => dur.as_secs_f64(),
				None => 0.0,
			};
			last_time = cur_time;
		}

		let timeout = server_update(&mut server_state, &mut trans_mem, delta)?;

		let opts = redis::streams::StreamReadOptions::default().count(layer::STREAM_READ_COUNT).block((timeout*1000.0) as usize);
		server_state.db.pipe.xread_options(&events[..], &last_ids[..], &opts);
		let response = com::auto_retry_flush_pipe(&mut server_state.db)?;


		if let redis::Value::Bulk(stream_responses) = &response {
		for stream_response_data in stream_responses {
			if let redis::Value::Bulk(stream_response) = &stream_response_data {
			if let redis::Value::Data(stream_name_raw) = &stream_response[0] {
			if let redis::Value::Bulk(stream_messages) = &stream_response[1] {
			for message_data in stream_messages {
				if let redis::Value::Bulk(message) = &message_data {
				if let redis::Value::Data(message_id_raw) = &message[0] {
				if let redis::Value::Bulk(message_body) = &message[1] {

				//clear all transient memory
				trans_mem.clear();

				//the borrow checker does not acknowledge that .clear() drops all borrowed references, so we have to force it to
				event_keys_mem.clear();
				let mut event_keys = unsafe {Vec::from_raw_parts(event_keys_mem.as_mut_ptr(), 0, event_keys_mem.capacity())};
				event_vals_mem.clear();
				let mut event_vals = unsafe {Vec::from_raw_parts(event_vals_mem.as_mut_ptr(), 0, event_vals_mem.capacity())};

				for i2 in 0..message_body.len()/2 {
					let i = i2*2;
					if let redis::Value::Data(message_key_raw) = &message_body[i] {
					if let redis::Value::Data(message_val_raw) = &message_body[i + 1] {

					let k = &message_key_raw[..];
					let v = &message_val_raw[..];
					event_keys.push(k);
					event_vals.push(v);

					} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
					} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
				}


				let i = events.binary_search(&&stream_name_raw[..]).expect("fatal error: we received an unrecognized event, how did this not get caught until now?");

				last_ids[i].clear();
				last_ids[i].extend(&message_id_raw[..]);//this avoids allocating

				server_state.db.pipe.cmd("HMSET").ignore();
				server_state.db.pipe.arg(layer::key::LAST_ID).arg(&stream_name_raw).arg(&last_ids[i]);

				server_event_received(&mut server_state, &stream_name_raw, message_id_raw, &event_keys[..], &event_vals[..], &mut trans_mem)?;


				} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
				} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
				} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
			}
			} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
			} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
			} else {db::mismatch_spec(&mut server_state.db, file!(), line!())}
		}
		} else if let redis::Value::Nil = response {
			print!("no events received before timeout: trying again...\n");
		}

		{
			let output = http_server::poll(&mut http_server_state, &mut trans_mem);

			if output.shutdown {//This is the only intended exit point atm
				return db::flush(&mut server_state.db, &mut trans_mem);
			}
		}
	}
}

fn main() {
	match server_main() {
		Ok(()) => print!("server has closed\n"),
		Err(()) => print!("server has closed due to fatal error\n"),
	};
}
