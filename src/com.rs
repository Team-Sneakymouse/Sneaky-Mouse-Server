//By Mami
use rand_pcg::*;
use crate::config::*;
use crate::config::event::*;
use crate::util::*;
use rand::{Rng};


pub fn send_error(db: &mut LayerData, error: &String) {
	//Unlike all of our other functions, this one will only attempt to send the error to redis once and then move on if it fails
	let mut cmd = redis::cmd("XADD");
	cmd.arg(output::DEBUG_ERROR).arg("*");
	cmd.arg(field::MESSAGE).arg(error);

	match cmd.query::<redis::Value>(&mut db.redis_con) {
		Ok(_) => (),
		Err(error) => match error.kind() {
			redis::ErrorKind::InvalidClientConfig => {
				panic!("fatal error: the redis command was formatted invalidly {}\n", error);
			}
			redis::ErrorKind::TypeError => {
				panic!("fatal error: TypeError thrown by redis {}, this should not be possible since we are explicitly trying to prevent redis from doing any type conversions\n", error);
			}
			_ => {
				print!("lost connection to the server: {}\n", error);
				print!("we will not attempt to reconnect\n");
			}
		}
	}
}

pub fn connect_to(redis_address: &str) -> Result<redis::Connection, ()> {
	let mut timeout = 0.0;
	let mut attempts = 0;
	loop {
		attempts += 1;
		match redis::Client::open(redis_address) {
			Ok(client) => match client.get_connection() {
				Ok(con) => {
					print!("successfully connected to server\n");
					return Ok(con);
				}
				Err(error) => {
					print!("failed to connect to '{}': {}\n", redis_address, error);
				}
			}
			Err(error) => panic!("could not parse redis url \'{}\': {}\n", redis_address, error)
		}
		if timeout < layer::RETRY_CON_TIMEOUT {
			std::thread::sleep(std::time::Duration::from_millis((layer::TIME_BETWEEN_RETRY_CON*1000.0) as u64));
			timeout += layer::TIME_BETWEEN_RETRY_CON;
		} else {
			break;
		}
	}

	print!("failed to connect to the redis server after {} attempts, shutting down: contact an admin to restart the server\n", attempts);
	return Err(());
}

pub fn auto_retry_flush_pipe(db: &mut LayerData) -> Result<redis::Value, ()> {
	//Only returns None if a connection cannot be established to the server, only course of action is to shut down until an admin intervenes
	//NOTE: there are a couple of panics for malformed programmer input, if this program is bug-free they will never trigger
	//NOTE: this can trigger a long thread::sleep() if reconnection fails
	match db.pipe.query(&mut db.redis_con) {
		Ok(data) => {
			db.pipe.clear();
			return Ok(data);
		},
		Err(error) => match error.kind() {
			redis::ErrorKind::InvalidClientConfig => {
				panic!("fatal error: the redis command was formatted invalidly {}\n", error);
			}
			redis::ErrorKind::TypeError => {
				panic!("fatal error: TypeError thrown by redis {}, this should not be possible since we are explicitly trying to prevent redis from doing any type conversions\n", error);
			}
			_ => {
				print!("lost connection to the server: {}\n", error);
				print!("attempting to reconnect\n");

				db.redis_con = connect_to(&db.redis_address[..])?;
				match db.pipe.query(&mut db.redis_con) {
					Ok(data) => {
						db.pipe.clear();
						return Ok(data);
					},
					Err(error) => {
						print!("connection immediately failed on retry, shutting down: {}\n", error);
						db.pipe.clear();
						return Err(());
					}
				}
			}
		}
	}
}


pub fn cheese_queue(db: &mut LayerData, trans_mem: &mut Vec<u8>, room: &[u8], user_id: &[u8], user: UserData) -> Result<(), ()> {
	db.pipe.cmd("XADD");

	db.pipe.arg(event::output::CHEESE_QUEUE).arg("*");
	db.pipe.arg(field::ROOM_ID).arg(room);
	db.pipe.arg(field::USER_ID).arg(user_id);
	db.pipe.arg(field::USER_NAME).arg(user.screen_name);

	db.pipe.arg(field::MOUSE_BODY).arg(user.body);
	if let Some(hat) = user.hat {
		db.pipe.arg(field::MOUSE_HAT).arg(hat);
	}


	auto_retry_flush_pipe(db)?;
	return Ok(());
}


pub fn cheese_despawn(db: &mut LayerData, trans_mem: &mut Vec<u8>, room: &[u8]) -> Result<(), ()> {
	db.pipe.cmd("XADD");
	db.pipe.arg(event::output::CHEESE_UPDATE).arg("*");
	db.pipe.arg(field::ROOM_ID).arg(room);


	auto_retry_flush_pipe(db)?;
	return Ok(());
}

pub fn save_cheese_to_pipe(db: &mut LayerData, cheese: &CheeseData) {
	if let Some(s) = &cheese.radical_image {
		db.pipe.arg(field::RADICAL_IMAGE).arg(s);
	} else {
		db.pipe.arg(field::RADICAL_IMAGE).arg(STR_NULL);
	}
	db.pipe.arg(field::TIME_MIN).arg(cheese.time_min);
	db.pipe.arg(field::TIME_MAX).arg(cheese.time_max);
	db.pipe.arg(field::SIZE).arg(cheese.size);
	db.pipe.arg(field::ORIGINAL_SIZE).arg(cheese.original_size);
	db.pipe.arg(field::SQUIRREL_MULT).arg(cheese.squirrel_mult);
	db.pipe.arg(field::SILENT).arg(cheese.silent);
	db.pipe.arg(field::EXCLUSIVE).arg(cheese.exclusive);
}

pub fn cheese_update(db: &mut LayerData, trans_mem: &mut Vec<u8>, room: &[u8], uuid: u64, cheese: &CheeseData) -> Result<(), ()> {
	db.pipe.cmd("XADD");
	db.pipe.arg(event::output::CHEESE_UPDATE).arg("*");
	db.pipe.arg(field::CHEESE_UUID).arg(uuid);
	db.pipe.arg(field::ROOM_ID).arg(room);

	save_cheese_to_pipe(db, cheese);

	auto_retry_flush_pipe(db)?;
	return Ok(());
}

pub fn cheese_award(db: &mut LayerData, trans_mem: &mut Vec<u8>, dest_id: &[u8], src_id_opt: Option<&[u8]>, cheese_delta: i32, gems_delta: i32) -> Result<(), ()> {
	db.pipe.cmd("XADD");
	db.pipe.arg(event::output::CHEESE_AWARD).arg("*");
	db.pipe.arg(field::DEST_ID).arg(dest_id);
	if let Some(src_id) = src_id_opt {
		db.pipe.arg(field::SRC_ID).arg(src_id);
	}
	if cheese_delta != 0 {
		db.pipe.arg(field::CHEESE_DELTA).arg(cheese_delta);
	}
	if gems_delta != 0 {
		db.pipe.arg(field::GEM_DELTA).arg(gems_delta);
	}

	auto_retry_flush_pipe(db)?;
	return Ok(());
}

