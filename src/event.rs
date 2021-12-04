//By Mami
use rand_pcg::*;
use crate::config::*;
use crate::util::*;
use rand::{Rng};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct SneakyMouseServer<'a> {
	pub redis_con : redis::Connection,
	pub redis_address : &'a str,
	pub rng : Pcg64,
	pub cur_time : f64,
	pub cheese_timeouts : Vec<f64>,
	pub cheese_uids : Vec<u64>,
	pub cheese_rooms : Vec<Vec<u8>>,//I don't like this
	pub cheese_ids : Vec<u64>,//hashes
}

pub fn get_event_list() -> [&'static [u8]; 7] {
	//Normally I wouldn't have a function return an allocation, but this is only called once by main to configure itself
	let mut v = [//this list is considered unordered
		IN_EVENT_CHEESE_GIVE,

		IN_EVENT_CHEESE_SPAWN,
		IN_EVENT_CHEESE_REQUEST,
		IN_EVENT_CHEESE_COLLECT,
		IN_EVENT_CHEESE_DESPAWN,

		IN_EVENT_DEBUG_CONSOLE,
		IN_EVENT_SHUTDOWN,
	];
	v.sort_unstable();
	return v;
}

pub fn server_event_received(server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]], trans_mem : &mut Vec<u8>) -> Option<bool> {
	match event_name {
		IN_EVENT_CHEESE_GIVE => {//Non-atomic
			if let Some(dest_uid_raw) = find_field_u8s(FIELD_DEST_UID, keys, vals) {
			let dest_uuid = lookup_user_uuid(server_state, dest_uid_raw)?;

			let mut cheese_delta = find_field_i32(FIELD_CHEESE_DELTA, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);
			let gem_delta = find_field_i32(FIELD_GEM_DELTA, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);

			let dest_key = push_u64_prefix(trans_mem, KEY_USER_PREFIX, dest_uuid);

			let strat = match find_field_u8s(FIELD_DEST_UID, keys, vals).unwrap_or(VAL_CHEESE_STRAT_CANCEL) {
				VAL_CHEESE_STRAT_CANCEL => 0,
				VAL_CHEESE_STRAT_OVERFLOW => 1,
				VAL_CHEESE_STRAT_SATURATE => 2,
				_ => {
					invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_DEST_UID);
					0
				}
			};

			let mut do_transaction = true;
			if strat == 0 {
				do_transaction = check_user_has_enough_currency(server_state, dest_key, FIELD_CHEESE_TOTAL, cheese_delta)?;
			} else if strat == 0 {
				let (d, _) = check_user_saturating_currency(server_state, dest_key, FIELD_CHEESE_TOTAL, cheese_delta, 0)?;
				cheese_delta = d;
			}

			if do_transaction && check_user_has_enough_currency(server_state, dest_key, FIELD_GEM_TOTAL, gem_delta)? {
				if let Some(src_uid_raw) = find_field_u8s(FIELD_SRC_UID, keys, vals) {
					let src_uuid = lookup_user_uuid(server_state, src_uid_raw)?;

					let cheese_cost_src = find_field_i32(FIELD_USER_UID, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);
					let gem_cost_src = find_field_i32(FIELD_USER_UID, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);

					let src_key = push_u64_prefix(trans_mem, KEY_USER_PREFIX, src_uuid);

					let mut do_transaction = true;
					if strat == 0 {
						do_transaction = check_user_has_enough_currency(server_state, src_key, FIELD_CHEESE_TOTAL, -cheese_delta - cheese_cost_src)?;
					} else if strat == 0 {
						let (d, is) = check_user_saturating_currency(server_state, src_key, FIELD_CHEESE_TOTAL, -cheese_delta, -cheese_cost_src)?;
						do_transaction = is;
						cheese_delta = -d;
					}

					if do_transaction && check_user_has_enough_currency(server_state, src_key, FIELD_GEM_TOTAL, -gem_delta - gem_cost_src)? {

						let mut pipe = redis::pipe();
						pipe.cmd("XADD");
						pipe.arg(OUT_EVENT_CHEESE_AWARD).arg("*");
						pipe.arg(FIELD_DEST_UID).arg(dest_uid_raw);
						pipe.arg(FIELD_SRC_UID).arg(src_uid_raw);
						pipe.arg(FIELD_CHEESE_DELTA).arg(cheese_delta);
						pipe.arg(FIELD_GEM_DELTA).arg(gem_delta);

						pipe.hincr(dest_key, FIELD_CHEESE_TOTAL, cheese_delta);
						pipe.hincr(dest_key, FIELD_GEM_TOTAL, gem_delta);

						pipe.hincr(src_key, FIELD_CHEESE_TOTAL, -cheese_delta - cheese_cost_src);
						pipe.hincr(src_key, FIELD_GEM_TOTAL, -gem_delta - gem_cost_src);

						auto_retry_pipe(server_state, &mut pipe)?;

					}
				} else {

					let mut pipe = redis::pipe();
					pipe.cmd("XADD");
					pipe.arg(OUT_EVENT_CHEESE_AWARD).arg("*");
					pipe.arg(FIELD_DEST_UID).arg(dest_uid_raw);
					pipe.arg(FIELD_CHEESE_DELTA).arg(cheese_delta);
					pipe.arg(FIELD_GEM_DELTA).arg(gem_delta);

					pipe.hincr(dest_key, FIELD_CHEESE_TOTAL, cheese_delta);
					pipe.hincr(dest_key, FIELD_GEM_TOTAL, gem_delta);

					auto_retry_pipe(server_state, &mut pipe)?;

				}
			}

			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_DEST_UID)}
		},
		IN_EVENT_CHEESE_SPAWN => {
			if let Some(room) = find_field_u8s(FIELD_ROOM_UID, keys, vals) {

			let mut cheese;
			let cheese_id_hash : u64;
			if let Some(cheese_id) = find_field_u8s(FIELD_CHEESE_ID, keys, vals) {
				let mut hasher = DefaultHasher::new();//I swear to god if this allocates
				Hash::hash_slice(cheese_id, &mut hasher);
				cheese_id_hash = hasher.finish();

				let mut cheese_uid = u64::MAX;
				for (i, cur_cheese_id) in server_state.cheese_ids.iter().enumerate() {
					if *cur_cheese_id == cheese_id_hash && server_state.cheese_rooms[i] == room {
						cheese_uid = server_state.cheese_uids[i];
						break;
					}
				}

				if cheese_uid == u64::MAX {
					cheese = get_cheese_from_uid(server_state, cheese_uid, trans_mem)?;
				} else if cheese_id_hash == 0 {
					cheese = generate_default_cheese();
				} else {
					cheese = get_cheese_from_id(server_state, cheese_id, trans_mem)?;
				}
			} else {
				cheese = generate_default_cheese();
				cheese_id_hash = 0;
			}

			if let Some(time_min) = find_field_u32(FIELD_TIME_MIN, server_state, event_name, event_uid, keys, vals) {
				if let Some(time_max) = find_field_u32(FIELD_TIME_MAX, server_state, event_name, event_uid, keys, vals) {
					cheese.time_min = time_min;
					cheese.time_max = time_max;
					if time_min > time_max {
						invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_TIME_MIN);
						cheese.time_min = time_max;
					}
				} else {
					cheese.time_min = time_min;
					cheese.time_max = time_min;
				}
			} else if let Some(time_max) = find_field_u32(FIELD_TIME_MAX, server_state, event_name, event_uid, keys, vals) {
				cheese.time_min = time_max;
				cheese.time_max = time_max;
			}

			cheese.size = find_field_i32(FIELD_SIZE, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.size);
			if cheese.original_size == 0 {
				cheese.original_size = cheese.size;
			}
			cheese.silent = find_field_bool(FIELD_SILENT, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.silent);
			cheese.exclusive = find_field_bool(FIELD_EXCLUSIVE, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.exclusive);

			cheese.image = find_field_and_allocate(FIELD_IMAGE, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.image);
			if let Some(s) = find_field_and_allocate(FIELD_RADICALIZES, server_state, event_name, event_uid, keys, vals) {
				cheese.radicalizes = Some(s);
			}

			if let Some(m) = find_field_f32(FIELD_SIZE_MULT, server_state, event_name, event_uid, keys, vals) {
				cheese.size = ((cheese.size as f32)*m) as i32;
			}
			if let Some(incr) = find_field_i32(FIELD_SIZE_INCR, server_state, event_name, event_uid, keys, vals) {
				cheese.size = i32::saturating_add(cheese.size, incr);
			}
			cheese.size = i32::min(CHEESE_SIZE_MAX, cheese.size);


			let mut cmd_getuid = redis::Cmd::incr(KEY_CHEESE_UID_MAX, 1i32);
			let val = auto_retry_cmd(server_state, &mut cmd_getuid)?;


			let cheese_uid = get_u64_from_val_or_panic(server_state, &val);


			let mut pipe = redis::pipe();

			pipe.cmd("HMSET");
			let cheese_key = push_u64_prefix(trans_mem, KEY_CHEESE_PREFIX, cheese_uid);
			pipe.arg(cheese_key);
			save_cheese(server_state, &mut pipe, &cheese);


			pipe.expire(cheese_key, VAL_CHEESE_MAX_TTL as usize);

			pipe.cmd("XADD");
			pipe.arg(OUT_EVENT_CHEESE_UPDATE).arg("*");
			pipe.arg(FIELD_CHEESE_UID).arg(cheese_uid);
			pipe.arg(FIELD_TRIGGER).arg(event_uid);
			pipe.arg(FIELD_ROOM_UID).arg(room);
			save_cheese(server_state, &mut pipe, &cheese);


			let time_out_ms = server_state.rng.gen_range(cheese.time_min..=cheese.time_max);
			let time_out = f64::min((time_out_ms as f64)/1000.0, VAL_CHEESE_MAX_TTL - 5.0);

			server_state.cheese_timeouts.push(time_out);
			server_state.cheese_uids.push(cheese_uid);
			server_state.cheese_rooms.push(Vec::from(room));
			server_state.cheese_ids.push(cheese_id_hash);


			auto_retry_pipe(server_state, &mut pipe)?;


			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_ROOM_UID)}
		},
		IN_EVENT_CHEESE_REQUEST => {
			if let Some(useruid_raw) = find_field_u8s(FIELD_USER_UID, keys, vals) {
			if let Some(username) = find_field_u8s(FIELD_USER_NAME, keys, vals) {
			if let Some(room) = find_field_u8s(FIELD_ROOM_UID, keys, vals) {

			let uuid = lookup_user_uuid(server_state, useruid_raw)?;

			let mut cmd = redis::cmd("HMGET");
			let userdata_key = push_u64_prefix(trans_mem, KEY_USER_PREFIX, uuid);
			cmd.arg(userdata_key).arg(KEY_MOUSE_BODY).arg(KEY_MOUSE_HAT);


			if let redis::Value::Bulk(values) = auto_retry_cmd(server_state, &mut cmd)? {


			let mut cmd_xadd = redis::cmd("XADD");
			cmd_xadd.arg(OUT_EVENT_CHEESE_QUEUE).arg("*");
			cmd_xadd.arg(FIELD_ROOM_UID).arg(room);
			cmd_xadd.arg(FIELD_USER_UID).arg(useruid_raw);
			cmd_xadd.arg(FIELD_USER_NAME).arg(username);

			match &values[0] {
				redis::Value::Data(body) => {
					cmd_xadd.arg(FIELD_MOUSE_BODY).arg(body);
				}
				redis::Value::Nil => {//set the default body
					cmd_xadd.arg(FIELD_MOUSE_BODY).arg(VAL_MOUSE_BODY_DEFAULT);
					let mut cmdset = redis::Cmd::hset( userdata_key, KEY_MOUSE_BODY, VAL_MOUSE_BODY_DEFAULT);


					auto_retry_cmd(server_state, &mut cmdset)?;


				}
				_ => mismatch_spec(server_state, file!(), line!())
			}
			match &values[1] {
				redis::Value::Data(hat) => {
					cmd_xadd.arg(FIELD_MOUSE_HAT).arg(hat);
				}
				redis::Value::Nil => (),
				_ => mismatch_spec(server_state, file!(), line!())
			}


			auto_retry_cmd(server_state, &mut cmd_xadd)?;


			} else {mismatch_spec(server_state, file!(), line!());}
			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_ROOM_UID)}
			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_USER_NAME)}
			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_USER_UID)}
		},
		IN_EVENT_CHEESE_COLLECT => {//Non-atomic
			if let Some(cheese_uid) = find_field_u64(FIELD_CHEESE_UID, server_state, event_name, event_uid, keys, vals) {
			if let Some(useruid_raw) = find_field_u8s(FIELD_USER_UID, keys, vals) {


			let uuid = lookup_user_uuid(server_state, useruid_raw)?;
			let cheese = get_cheese_from_uid(server_state, cheese_uid, trans_mem)?;


			let userdata_key = push_u64_prefix(trans_mem, KEY_USER_PREFIX, uuid);

			let incr;
			if cheese.squirrel_mult != 0.0 {


				let mut cmdget = redis::Cmd::hget(userdata_key, FIELD_CHEESE_TOTAL);
				let val = auto_retry_cmd(server_state, &mut cmdget)?;


				if let Some(user_cheese) = get_db_entry_i64(server_state, userdata_key, FIELD_CHEESE_TOTAL, val) {
					incr = i64::saturating_add(((user_cheese as f64)*(cheese.squirrel_mult as f64)) as i64, cheese.size as i64);
				} else {
					incr = 0;
				}
			} else if cheese.size != 0 {
				incr = cheese.size as i64;
			} else {
				incr = 0;
			}


			if incr != 0 {
				let mut cmdincr = redis::Cmd::hincr(userdata_key, FIELD_CHEESE_TOTAL, incr);
				auto_retry_cmd(server_state, &mut cmdincr)?;
			}


			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_USER_UID)}
			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_CHEESE_UID)}
		}
		IN_EVENT_CHEESE_DESPAWN => {
			if let Some(cheese_uid) = find_field_u64(FIELD_CHEESE_UID, server_state, event_name, event_uid, keys, vals) {

			for (i, cur_cheese_uid) in server_state.cheese_uids.iter().enumerate() {
				if *cur_cheese_uid == cheese_uid {
					server_state.cheese_timeouts.swap_remove(i);
					server_state.cheese_uids.swap_remove(i);
					let room = server_state.cheese_rooms.swap_remove(i);
					server_state.cheese_ids.swap_remove(i);

					let mut cmd = redis::cmd("XADD");
					cmd.arg(OUT_EVENT_CHEESE_UPDATE).arg("*");
					cmd.arg(FIELD_ROOM_UID).arg(room);


					auto_retry_cmd(server_state, &mut cmd)?;


					break;
				}
			}

			} else {missing_field(server_state, event_name, event_uid, keys, vals, FIELD_CHEESE_UID)}
		},
		IN_EVENT_DEBUG_CONSOLE => {
			print!("debug event: {} <", String::from_utf8_lossy(event_uid));
			for (i, key) in keys.iter().enumerate() {
				print!("{}:{}", String::from_utf8_lossy(key), String::from_utf8_lossy(vals[i]));
				if i+1 == keys.len() {
					print!("> {}\n", server_state.redis_address);
				} else {
					print!(", ");
				}
			}
		},
		IN_EVENT_SHUTDOWN => {
			print!("shutdown request acknowledged, closing the server\n");
			//TODO: pipeline this above all else, a slow closing server is unacceptable

			for i in 0..server_state.cheese_rooms.len() {

				let mut cmd_xadd = redis::cmd("XADD");
				cmd_xadd.arg(OUT_EVENT_CHEESE_UPDATE).arg("*");
				cmd_xadd.arg(FIELD_ROOM_UID).arg(&server_state.cheese_rooms[i]);


				auto_retry_cmd(server_state, &mut cmd_xadd)?;


			}
			return None
		}
		_ => {
			panic!("fatal error: we received an unrecognized event from redis, '{}', please check the events list\n", String::from_utf8_lossy(event_name));
		}
	}
	Some(true)
}


pub fn server_update(server_state : &mut SneakyMouseServer, trans_mem : &mut Vec<u8>, delta : f64) -> Option<f64> {
	server_state.cur_time += delta;

	let mut i = 0;
	let mut next_timeout = REDIS_STREAM_TIMEOUT_MAX;
	while i < server_state.cheese_timeouts.len() {
		if server_state.cheese_timeouts[i] <= server_state.cur_time {
			server_state.cheese_timeouts.swap_remove(i);
			let cheese_uid = server_state.cheese_uids.swap_remove(i);
			let room = server_state.cheese_rooms.swap_remove(i);
			server_state.cheese_ids.swap_remove(i);

			let mut cheese = get_cheese_from_uid(server_state, cheese_uid, trans_mem)?;

			let mut pipe = redis::pipe();
			pipe.cmd("XADD");
			pipe.arg(OUT_EVENT_CHEESE_UPDATE).arg("*");
			pipe.arg(FIELD_ROOM_UID).arg(room);

			if let Some(radical) = &cheese.radicalizes {
				cheese.image.clear();
				cheese.image.extend(radical);
				cheese.exclusive = true;
				cheese.size = (CHEESE_RADICAL_MULT*cheese.size as f32) as i32;
				cheese.squirrel_mult *= CHEESE_RADICAL_MULT;

				save_cheese(server_state, &mut pipe, &cheese);

				pipe.cmd("HMSET");
				pipe.arg(push_u64_prefix(trans_mem, KEY_CHEESE_PREFIX, cheese_uid));

				save_cheese(server_state, &mut pipe, &cheese);
			}


			auto_retry_pipe(server_state, &mut pipe)?;


		} else {
			i += 1;
			next_timeout = f64::min(next_timeout, server_state.cheese_timeouts[i] - server_state.cur_time);
		}
	}

	Some(next_timeout)
}
