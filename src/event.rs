//By Mami
use crate::config::*;
use crate::config::event::*;
use crate::util::*;
use crate::db;
use crate::com;
use rand_pcg::*;
use rand::{Rng};
use chrono::{Utc};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};


pub fn get_event_list() -> [&'static [u8]; 6] {
	let mut v = [//this list is considered unordered
		event::input::CHEESE_GIVE,

		event::input::CHEESE_SPAWN,
		event::input::CHEESE_REQUEST,
		event::input::CHEESE_COLLECT,
		event::input::CHEESE_DESPAWN,

		event::input::DEBUG_CONSOLE,
		// event::input::SHUTDOWN,
	];
	v.sort_unstable();
	return v;
}

pub fn server_event_received(server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]], trans_mem: &mut Vec<u8>) -> Result<(), ()> {
	match event_name {
		event::input::CHEESE_GIVE => {//Non-atomic
			if let Some(dest_id) = find_field_u8s(field::DEST_ID, keys, vals) {


			let (dest_uuid, dest_user) = match db::get_user_from_id(&mut server_state.db, trans_mem, dest_id) {
				Ok(v) => v,
				Err(LayerError::NotFound) => {
					missing_user(server_state, event_name, event_uid, keys, vals, field::DEST_ID);
					return Ok(());
				},
				Err(LayerError::Fatal) => return Err(()),
			};


			let mut cheese_delta = find_field_i32(field::CHEESE_DELTA, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);
			let gem_delta = find_field_i32(field::GEM_DELTA, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);

			let strat = match find_field_u8s(field::DEST_ID, keys, vals).unwrap_or(val::CHEESE_STRAT_CANCEL) {
				val::CHEESE_STRAT_CANCEL => 0,
				val::CHEESE_STRAT_OVERFLOW => 1,
				val::CHEESE_STRAT_SATURATE => 2,
				_ => {
					invalid_value(server_state, event_name, event_uid, keys, vals, field::DEST_ID);
					0
				}
			};

			let mut do_transaction = true;
			if strat == 0 {
				do_transaction = check_user_has_enough_currency(&dest_user, Currency::CHEESE, cheese_delta);
			} else if strat == 2 {
				let (d, _) = check_user_saturating_currency(&dest_user, Currency::CHEESE, cheese_delta, 0);
				cheese_delta = d;
			}

			if do_transaction && check_user_has_enough_currency(&dest_user, Currency::GEMS, gem_delta) {
				if let Some(src_id) = find_field_u8s(field::SRC_ID, keys, vals) {

					let (src_uuid, src_user) = match db::get_user_from_id(&mut server_state.db, trans_mem, src_id) {
						Ok(v) => v,
						Err(LayerError::NotFound) => {
							missing_user(server_state, event_name, event_uid, keys, vals, field::DEST_ID);
							return Ok(());
						},
						Err(LayerError::Fatal) => return Err(()),
					};

					let cheese_cost_src = find_field_i32(field::CHEESE_COST, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);
					let gem_cost_src = find_field_i32(field::GEM_COST, server_state, event_name, event_uid, keys, vals).unwrap_or(0).clamp(-i32::MAX/2, i32::MAX/2);

					let mut do_transaction = true;
					if strat == 0 {
						do_transaction = check_user_has_enough_currency(&src_user, Currency::CHEESE, -cheese_delta - cheese_cost_src);
					} else if strat == 2 {
						let (d, is) = check_user_saturating_currency(&src_user, Currency::CHEESE, -cheese_delta, -cheese_cost_src);
						do_transaction = is;
						cheese_delta = -d;
					}

					if do_transaction && check_user_has_enough_currency(&src_user, Currency::GEMS, -gem_delta - gem_cost_src) {


						db::incr_user_currency(&mut server_state.db, trans_mem, src_uuid, &src_user, Currency::CHEESE, -cheese_delta - cheese_cost_src);
						db::incr_user_currency(&mut server_state.db, trans_mem, src_uuid, &src_user, Currency::GEMS, -gem_delta - gem_cost_src);

						db::incr_user_currency(&mut server_state.db, trans_mem, dest_uuid, &dest_user, Currency::CHEESE, cheese_delta);
						db::incr_user_currency(&mut server_state.db, trans_mem, dest_uuid, &dest_user, Currency::GEMS, gem_delta);

						com::cheese_award(&mut server_state.db, trans_mem, dest_id, Some(src_id), cheese_delta, gem_delta)?;
					}
				} else {
					db::incr_user_currency(&mut server_state.db, trans_mem, dest_uuid, &dest_user, Currency::CHEESE, cheese_delta);
					db::incr_user_currency(&mut server_state.db, trans_mem, dest_uuid, &dest_user, Currency::GEMS, gem_delta);

					com::cheese_award(&mut server_state.db, trans_mem, dest_id, None, cheese_delta, gem_delta)?;
				}
			}

			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::DEST_ID)}
		},
		event::input::CHEESE_SPAWN => {
			if let Some(room) = find_field_u8s(field::ROOM_ID, keys, vals) {
			let mut cheese_uid: Option<u64> = None;


			let mut cheese;
			let cheese_id_hash: u64;
			if let Some(cheese_id) = find_field_u8s(field::CHEESE_ID, keys, vals) {
				let mut hasher = DefaultHasher::new();//I swear to god if this allocates
				Hash::hash_slice(cheese_id, &mut hasher);
				cheese_id_hash = hasher.finish();

				for (i, cur_cheese_id) in server_state.cheese_ids.iter().enumerate() {
					if *cur_cheese_id == cheese_id_hash && server_state.cheese_rooms[i] == room {
						cheese_uid = Some(server_state.cheese_uids[i]);
						break;
					}
				}

				if let Some(uid) = cheese_uid {
					cheese = db::get_cheese_from_uuid(&mut server_state.db, trans_mem, uid)?;
				} else if cheese_id_hash == 0 {
					cheese = generate_default_cheese();
				} else {
					cheese = db::get_cheese_from_id(&mut server_state.db, trans_mem, cheese_id)?;
				}
			} else {
				cheese = generate_default_cheese();
				cheese_id_hash = 0;
			}

			if let Some(time_min) = find_field_u32(field::TIME_MIN, server_state, event_name, event_uid, keys, vals) {
				if let Some(time_max) = find_field_u32(field::TIME_MAX, server_state, event_name, event_uid, keys, vals) {
					cheese.time_min = time_min;
					cheese.time_max = time_max;
					if time_min > time_max {
						invalid_value(server_state, event_name, event_uid, keys, vals, field::TIME_MIN);
						cheese.time_min = time_max;
					}
				} else {
					cheese.time_min = time_min;
					cheese.time_max = time_min;
				}
			} else if let Some(time_max) = find_field_u32(field::TIME_MAX, server_state, event_name, event_uid, keys, vals) {
				cheese.time_min = time_max;
				cheese.time_max = time_max;
			}

			cheese.size = find_field_i32(field::SIZE, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.size).clamp(-CHEESE_SIZE_MAX, CHEESE_SIZE_MAX);
			if cheese.original_size == 0 {
				cheese.original_size = cheese.size;
			}
			cheese.gems = find_field_i32(field::GEMS, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.gems).clamp(-CHEESE_GEMS_MAX, CHEESE_GEMS_MAX);
			cheese.silent = find_field_bool(field::SILENT, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.silent);
			cheese.exclusive = find_field_bool(field::EXCLUSIVE, server_state, event_name, event_uid, keys, vals).unwrap_or(cheese.exclusive);

			cheese.image = find_field_u8s(field::IMAGE, keys, vals).unwrap_or(cheese.image);
			if let Some(s) = find_field_u8s(field::RADICAL_IMAGE, keys, vals) {
				cheese.radical_image = Some(s);
			}

			if let Some(m) = find_field_f32(field::SIZE_MULT, server_state, event_name, event_uid, keys, vals) {
				cheese.size = ((cheese.size as f32)*m) as i32;
			}
			if let Some(incr) = find_field_i32(field::SIZE_INCR, server_state, event_name, event_uid, keys, vals) {
				cheese.size = i32::saturating_add(cheese.size, incr);
			}
			cheese.size = cheese.size.clamp(-CHEESE_SIZE_MAX, CHEESE_SIZE_MAX);


			if let Some(uid) = cheese_uid {
				db::set_cheese(&mut server_state.db, trans_mem, uid, &cheese);

			} else {
				let uid = db::add_new_cheese(&mut server_state.db, trans_mem, &cheese)?;

				let time_out_ms = server_state.rng.gen_range(cheese.time_min..=cheese.time_max);
				let time_out = f64::min((time_out_ms as f64)/1000.0, CHEESE_TTL - 5.0);

				server_state.cheese_timeouts.push(time_out);
				server_state.cheese_uids.push(uid);
				server_state.cheese_rooms.push(Vec::from(room));
				server_state.cheese_ids.push(cheese_id_hash);
			}

			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::ROOM_ID)}
		},
		event::input::CHEESE_REQUEST => {
			if let Some(user_id) = find_field_u8s(field::USER_ID, keys, vals) {
			if let Some(user_screen_name) = find_field_u8s(field::USER_NAME, keys, vals) {
			if let Some(room) = find_field_u8s(field::ROOM_ID, keys, vals) {


			let (uuid, user) = db::get_or_create_user_from_id(&mut server_state.db, trans_mem, user_id, user_screen_name)?;
			com::cheese_queue(&mut server_state.db, trans_mem, room, user_id, &user)?;


			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::ROOM_ID)}
			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::USER_NAME)}
			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::USER_ID)}
		},
		event::input::CHEESE_COLLECT => {
			if let Some(cheese_uuid) = find_field_u64(field::CHEESE_UUID, server_state, event_name, event_uid, keys, vals) {
			if let Some(user_id) = find_field_u8s(field::USER_ID, keys, vals) {


			let (uuid, user) = match db::get_user_from_id(&mut server_state.db, trans_mem, user_id) {
				Ok(v) => v,
				Err(LayerError::NotFound) => {
					missing_user(server_state, event_name, event_uid, keys, vals, field::USER_ID);
					return Ok(());
				},
				Err(LayerError::Fatal) => return Err(()),
			};
			let cheese = db::get_cheese_from_uuid(&mut server_state.db, trans_mem, cheese_uuid)?;

			let incr;
			if cheese.squirrel_mult != 0.0 {
				incr = i32::saturating_add(((user.cheese as f64)*(cheese.squirrel_mult as f64)) as i32, cheese.size as i32);
			} else {
				incr = cheese.size;
			}


			db::incr_user_currency(&mut server_state.db, trans_mem, uuid, &user, Currency::CHEESE, incr);
			db::incr_user_currency(&mut server_state.db, trans_mem, uuid, &user, Currency::GEMS, cheese.gems);


			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::USER_ID)}
			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::CHEESE_UUID)}
		}
		event::input::CHEESE_DESPAWN => {
			if let Some(room) = find_field_u8s(field::ROOM_ID, keys, vals) {

			for (i, cur_room) in server_state.cheese_rooms.iter().enumerate() {
				if *cur_room == room {
					server_state.cheese_timeouts.swap_remove(i);
					server_state.cheese_uids.swap_remove(i);
					server_state.cheese_rooms.swap_remove(i);
					server_state.cheese_ids.swap_remove(i);

					break;
				}
			}

			com::cheese_despawn(&mut server_state.db, trans_mem, room)?;

			} else {missing_field(server_state, event_name, event_uid, keys, vals, field::ROOM_ID)}
		},
		event::input::DEBUG_CONSOLE => {
			print!("debug event: {} <", String::from_utf8_lossy(event_uid));
			for (i, key) in keys.iter().enumerate() {
				print!("{}:{}", String::from_utf8_lossy(key), String::from_utf8_lossy(vals[i]));
				if i+1 == keys.len() {
					print!("> {}\n", server_state.db.redis_address);
				} else {
					print!(", ");
				}
			}
		},
		_ => {
			panic!("fatal error: we received an unrecognized event from redis, '{}', please check the events list\n", String::from_utf8_lossy(event_name));
		}
	}
	Ok(())
}


pub fn server_update(server_state: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, delta: f64) -> Result<f64, ()> {
	server_state.cur_time += delta;

	//check unix timestamp and do reset if enough time has passed
	let now_unix: i64 = Utc::now().timestamp();
	let last_reset_unix: i64 = server_state.last_reset_otherwise_server_genisis_unix;
	/*
	We want to add a day to the last_reset time and round it down so that it equals SM_RESET_EPOCH%SECS_IN_DAY + c*SECS_IN_DAY for some int c.
	Given last_reset, SM_RESET_EPOCH, there exists c such that
		SM_RESET_EPOCH%SECS_IN_DAY + (c - 1)*SECS_IN_DAY last_reset <= last_reset < SM_RESET_EPOCH%SECS_IN_DAY + c*SECS_IN_DAY.
	We want to figure out the value of SM_RESET_EPOCH%SECS_IN_DAY + c*SECS_IN_DAY, so we want to find c.
	So (c - 1)*SECS_IN_DAY <= last_reset - SM_RESET_EPOCH%SECS_IN_DAY
		==> c - 1 <= (last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY
		==> c <= (last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY + 1.
	and last_reset < SM_RESET_EPOCH%SECS_IN_DAY + c*SECS_IN_DAY
		==> last_reset - SM_RESET_EPOCH%SECS_IN_DAY < c*SECS_IN_DAY
		==> (last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY < c.
	Thus (last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY < c <= (last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY + 1.
	Given a, a < floor(a + 1) <= a + 1.
	Thus only int c that can satisfy the above property is 'floor((last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY + 1)'.
	So c = floor((last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY + 1)
			= floor((last_reset - SM_RESET_EPOCH%SECS_IN_DAY)/SECS_IN_DAY) + 1
			= (last_reset - SM_RESET_EPOCH%SECS_IN_DAY) '/' SECS_IN_DAY + 1 (where '/' is integer division)
	*/
	let c: i64 = (last_reset_unix - SM_RESET_EPOCH_UNIX%SECS_IN_DAY_UNIX)/SECS_IN_DAY_UNIX + 1;
	let next_reset_unix: i64 = SM_RESET_EPOCH_UNIX%SECS_IN_DAY_UNIX + c*SECS_IN_DAY_UNIX;

	if next_reset_unix <= now_unix {
		//time going backwards as it does in unix time (leap seconds) will not affect this code since it always takes the first greater and setting that to the last reset time
		server_state.last_reset_otherwise_server_genisis_unix = now_unix;
		//do reset
		db::daily_reset(&mut server_state.db, trans_mem, now_unix);
	}


		//check for cheese timeouts
	let mut i = 0;
	let mut next_timeout = event::TIMEOUT_MAX;
	while i < server_state.cheese_timeouts.len() {
		if server_state.cheese_timeouts[i] <= server_state.cur_time {
			server_state.cheese_timeouts.swap_remove(i);
			let cheese_uuid = server_state.cheese_uids.swap_remove(i);
			let room = server_state.cheese_rooms.swap_remove(i);
			server_state.cheese_ids.swap_remove(i);

			let mut cheese = db::get_cheese_from_uuid(&mut server_state.db, trans_mem, cheese_uuid)?;

			if let Some(radical) = cheese.radical_image {
				cheese.image = radical;
				cheese.exclusive = true;
				cheese.size = (CHEESE_RADICAL_MULT*cheese.size as f32) as i32;
				cheese.squirrel_mult *= CHEESE_RADICAL_MULT;

				db::set_cheese(&mut server_state.db, trans_mem, cheese_uuid, &cheese);
				com::cheese_update(&mut server_state.db, trans_mem, &room, cheese_uuid, &cheese)?;
			} else {
				com::cheese_despawn(&mut server_state.db, trans_mem, &room)?;
			}


		} else {
			i += 1;
			next_timeout = f64::min(next_timeout, server_state.cheese_timeouts[i] - server_state.cur_time);
		}
	}

	Ok(next_timeout)
}

pub fn server_shutdown(server_state: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>) -> Result<(),()> {
	for i in 0..server_state.cheese_rooms.len() {
		com::cheese_despawn(&mut server_state.db, trans_mem, &server_state.cheese_rooms[i])?;
	}
	return db::flush(&mut server_state.db, trans_mem);
}
