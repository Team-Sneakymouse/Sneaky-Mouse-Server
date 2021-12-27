//By Mami
use rand_pcg::*;
use std::net::TcpListener;

pub struct LayerData<'a> {
	pub redis_con: redis::Connection,
	pub redis_address: &'a str,
    pub pipe: redis::Pipeline,
}
pub struct HTTPServer<'a> {
	pub disabled: bool,
	pub addr: &'a str,
	pub listener: Option<TcpListener>,
}
pub struct HTTPServerOutput {
    pub shutdown: bool
}

pub struct SneakyMouseServer<'a> {
    pub db: LayerData<'a>,
	pub rng: Pcg64,
	pub cur_time: f64,
    pub last_reset_otherwise_server_genisis_unix: i64,

	pub cheese_timeouts: Vec<f64>,
	pub cheese_uids: Vec<u64>,
	pub cheese_rooms: Vec<Vec<u8>>,//I don't like this
	pub cheese_ids: Vec<u64>,//hashes
}

#[derive(Copy, Clone)]
pub enum Currency {
    CHEESE,
    GEMS,
}
#[derive(Copy, Clone)]
pub enum LayerError {
    Fatal, //(for now this is only thrown on lost connection)
    NotFound,
}

pub struct UserData<'a> {
    pub str_mem: Vec<u8>,
    pub screen_name: &'a[u8],
    pub hat: Option<&'a[u8]>,
    pub body: &'a[u8],
    pub cheese: i64,
    pub gems: i64,
}

pub struct CheeseData<'a> {
    pub str_mem: Vec<u8>,
    pub image: &'a[u8],
    pub radical_image: Option<&'a[u8]>,
    pub time_min: u32,//milisec
    pub time_max: u32,//milisec
    pub size: i32,
    pub original_size: i32,
    pub squirrel_mult: f32,
    pub silent: bool,
    pub exclusive: bool,
}


pub const ASCII_NEG: u8 = 45;
pub const ASCII_0: u8 = 48;
pub const ASCII_9: u8 = ASCII_0 + 9;
pub const ASCII_NEWLINE: u8 = 10;
pub const STR_NULL: &str = "null";

pub const SM_RESET_EPOCH_UNIX: i64 = 1640516400;
pub const SECS_IN_DAY_UNIX: i64 = 60*60*24;

pub const CHEESE_RADICAL_MULT: f32 = 1.1;
pub const CHEESE_TTL: f64 = 5.0*60.0*60.0;

pub mod event {
    pub mod input {
        pub const DEBUG_CONSOLE:  &[u8] = b"debug:console";
        pub const CHEESE_GIVE:    &[u8] = b"sm-cheese:give";
        pub const CHEESE_SPAWN  : &[u8] = b"sm-cheese:spawn";
        pub const CHEESE_REQUEST: &[u8] = b"sm-cheese:request";
        pub const CHEESE_COLLECT: &[u8] = b"sm-cheese:collect";
        pub const CHEESE_DESPAWN: &[u8] = b"sm-cheese:despawn";
    }
    pub mod output {
        pub const DEBUG_ERROR: &[u8] = b"debug:error";
        pub const CHEESE_AWARD: &[u8] = b"sm-cheese:award";
        pub const CHEESE_UPDATE : &[u8] = b"sm-cheese:update";
        pub const CHEESE_QUEUE  : &[u8] = b"sm-cheese:queue";
    }
    pub mod field {
        pub const MESSAGE: &str = "message";
        pub const MOUSE_BODY: &str = "body";
        pub const MOUSE_HAT: &str = "hat";
        pub const USER_ID: &str = "user-id";
        pub const ROOM_ID: &str = "room-id";
        pub const USER_NAME: &str = "user-name";
        pub const CHEESE_DELTA: &str = "cheese-delta";
        pub const GEM_DELTA: &str = "gems-delta";
        pub const CHEESE_COST: &str = "cheese-cost";
        pub const GEM_COST: &str = "gem-cost";
        pub const DEST_ID: &str = "dest-id";
        pub const SRC_ID: &str = "src-id";

        pub const CHEESE_ID: &str = "cheese-id";
        pub const CHEESE_UUID: &str = "cheese-uuid";
        pub const IMAGE: &str = "image";
        pub const RADICAL_IMAGE: &str = "radical-image";
        pub const TIME_MIN: &str = "time-min";
        pub const TIME_MAX: &str = "time-max";
        pub const EXCLUSIVE: &str = "exclusive";
        pub const SIZE: &str = "size";
        pub const ORIGINAL_SIZE: &str = "original-size";
        pub const SQUIRREL_MULT: &str = "squirrel-mult";
        pub const SILENT: &str = "silent";
        pub const SIZE_MULT: &str = "size-mult";
        pub const SIZE_INCR: &str = "size-add";
    }
    pub mod val {
        pub const CHEESE_STRAT_CANCEL: &[u8] = b"cancel";
        pub const CHEESE_STRAT_OVERFLOW: &[u8] = b"overflow";
        pub const CHEESE_STRAT_SATURATE: &[u8] = b"saturate";
    }
    pub const CHEESE_SIZE_MAX: i32 = 555_555_555;
    pub const TIMEOUT_MAX: f64 = 0.5;
}



pub mod layer {
    pub const TIME_BETWEEN_RETRY_CON: f64 = 5.0;
    pub const RETRY_CON_TIMEOUT: f64 = 60.0;
    pub const STREAM_READ_COUNT: usize = 55;

    pub mod default {//It is assumed that if a value is not present here its default value is 0 or None
        pub const CHEESE_IMAGE: &[u8] = b"danipls";
        pub const CHEESE_RADICAL_IMAGE: &[u8] = b"danipls";
        pub const CHEESE_TIME_MIN: u32 = 4*60*1000;
        pub const CHEESE_TIME_MAX: u32 = 5*60*1000;
        pub const CHEESE_SIZE: i32 = 1;
        pub const LAST_ID: &str = "0-0";//either $ or 0-0 are acceptable here
        pub const SCREEN_NAME: &str = "I am Error";
        pub const MOUSE_BODY: &str = "danipls";
    }
    pub mod key {
        pub const LAST_ID: &str = "last_id";
        pub const USERUUID_HM: &str = "user-uuid";
        pub const MAXUUID: &str = "user-uuid-max";
        pub const CHEESE_UID_MAX: &str = "cheese-uid-max";

        pub const GLOBAL_CHEESE_RANKING: &str = "global-cheese-ranking";
        pub const DAILY_RESET_TIMESTAMP_UNIX: &str = "daily-reset-unix";
        pub const GLOBAL_GEMS_RANKING: &str = "global-gems-ranking";
        pub const DAILY_CHEESE_RANKING: &str = "daily-cheese-ranking";

        pub mod prefix {
            pub const CHEESE_DATA: &str = "cheese-temp:";
            pub const USER: &str = "user:";
            pub const CHEESE: &str = "cheese:";
        }
        pub mod cheese {
            pub const IMAGE: &str = "image";
            pub const RADICAL: &str = "radical";
            pub const TIME_MIN: &str = "time-min";
            pub const TIME_MAX: &str = "time-max";
            pub const SIZE: &str = "size";
            pub const ORIGINAL_SIZE: &str = "original-size";
            pub const SQUIRREL_MULT: &str = "squirrel-mult";
            pub const SILENT: &str = "silent";
            pub const EXCLUSIVE: &str = "exclusive";
        }
        pub mod user {
            pub const SCREEN_NAME: &str = "screen-name";
            pub const BODY: &str = "body";
            pub const HAT: &str = "hat";
            pub const CHEESE: &str = "cheese";
            pub const GEMS: &str = "gems";
        }
    }
}
pub mod http {
    pub const CONNECTION_TIMEOUT: f64 = 1.0;
    pub const REQUEST_SIZE_MAX: usize = 5555;
    pub mod status {
        pub const OK: &str ="200 OK";
        pub const NOT_FOUND: &str ="404 Not Found";
        pub const METHOD_NOT_ALLOWED: &str = "405 Method Not Allowed";
        // pub const BAD_REQUEST: &str = "400 Bad Request";
    }
}

pub fn generate_default_user<'a>(screen_name: &[u8]) -> UserData<'a> {
    let mut mem = Vec::<u8>::new();
    mem.extend_from_slice(screen_name);
    let name = unsafe {
        std::slice::from_raw_parts(mem.as_ptr(), mem.len())
    };
    UserData{
        str_mem: mem,
        screen_name: name,
        hat: None,
        body: layer::default::MOUSE_BODY.as_bytes(),
        cheese: 0,
        gems: 0,
    }
}
pub fn generate_default_cheese<'a>() -> CheeseData<'a> {
    CheeseData{
        str_mem: Vec::<u8>::new(),
        image: layer::default::CHEESE_IMAGE,
        radical_image: Some(layer::default::CHEESE_RADICAL_IMAGE),
        time_min: layer::default::CHEESE_TIME_MIN,
        time_max: layer::default::CHEESE_TIME_MAX,
        size: layer::default::CHEESE_SIZE,
        original_size: layer::default::CHEESE_SIZE,
        squirrel_mult: 0.0,
        silent: false,
        exclusive: false,
    }
}

