//By Mami

pub struct CheeseData {
    pub image : Vec<u8>,
    pub radicalizes : Option<Vec<u8>>,
    pub time_min : u32,//milisec
    pub time_max : u32,//milisec
    pub size : i32,
    pub original_size : i32,
    pub squirrel_mult : f32,
    pub silent : bool,
    pub exclusive : bool,
}
pub fn generate_default_cheese() -> CheeseData {
    CheeseData{
        image : Vec::from(&b"danipls"[..]),
        radicalizes : Some(Vec::from(&b"danipls"[..])),
        time_min : 4*60*1000,
        time_max : 5*60*1000,
        size : 1,
        original_size : 1,
        squirrel_mult : 0.0,
        silent : false,
        exclusive : false,
    }
}

pub const REDIS_STREAM_READ_COUNT : usize = 55;
pub const REDIS_STREAM_TIMEOUT_MAX : f64 = 3.0;
pub const REDIS_TIME_BETWEEN_RETRY_CON : u64 = 5;
pub const REDIS_RETRY_CON_MAX_ATTEMPTS : i32 = 5;

pub const REDIS_LAST_ID_PREFIX : &str = "last_id";
pub const REDIS_LAST_ID_DEFAULT : &str = "0-0";//either $ or 0-0 are acceptable here

pub const KEY_USERUUID_HM : &str = "user-uuid";
pub const KEY_MAXUUID : &str = "user-uuid-max";
pub const KEY_CHEESE_UID_MAX : &str = "cheese-uid-max";
pub const KEY_CHEESE_PREFIX : &str = "cheese:";
pub const KEY_CHEESE_DATA_PREFIX : &str = "cheese-temp:";
pub const KEY_CHEESE_IMAGE : &str = "image";
pub const KEY_CHEESE_SILENT : &str = "silent";
pub const KEY_USER_PREFIX : &str = "user:";
pub const KEY_MOUSE_BODY : &str = "mouse:body";
pub const KEY_MOUSE_HAT : &str = "mouse:hat";

pub const VAL_MOUSE_BODY_DEFAULT : &str = "danipls";
pub const VAL_CHEESE_DEFAULT_IMAGE : &[u8] = b"danipls";
pub const VAL_CHEESE_MAX_TTL : f64 = 5.0*60.0*60.0;
// pub const VAL_TRUE : &str = "true";
// pub const VAL_FALSE : &str = "false";

pub const EVENT_DEBUG_CONSOLE : &[u8] = b"debug:console";
pub const EVENT_DEBUG_ERROR : &[u8] = b"debug:error";
pub const EVENT_SHUTDOWN : &[u8] = b"sm:shutdown";
pub const EVENT_CHEESE_REQUEST : &[u8] = b"sm-cheese:request";
pub const EVENT_CHEESE_COLLECT : &[u8] = b"sm-cheese:collect";
pub const EVENT_CHEESE_SPAWN : &[u8] = b"sm-cheese:spawn";
pub const EVENT_CHEESE_UPDATE : &[u8] = b"sm-cheese:update";
pub const EVENT_CHEESE_COLLECTED : &[u8] = b"sm-cheese:collected";


pub const FIELD_TRIGGER : &str = "trigger";
pub const FIELD_MESSAGE : &str = "message";
pub const FIELD_MOUSE_BODY : &str = "body";
pub const FIELD_MOUSE_HAT : &str = "hat";
pub const FIELD_USER_UID : &str = "user-uid";
pub const FIELD_USER_NAME : &str = "user-name";
pub const FIELD_ROOM_UID : &str = "room-uid";
pub const FIELD_CHEESE_TOTAL : &str = "cheese";

pub const FIELD_CHEESE_ID : &str = "cheese-id";
pub const FIELD_CHEESE_UID : &str = "cheese-uid";
pub const FIELD_IMAGE : &str = "image";
pub const FIELD_RADICALIZES : &str = "radical-image";
pub const FIELD_TIME_MIN : &str = "time-min";
pub const FIELD_TIME_MAX : &str = "time-max";
pub const FIELD_EXCLUSIVE : &str = "exclusive";
pub const FIELD_SIZE : &str = "size";
pub const FIELD_SQUIRREL_MULT : &str = "squirrel-mult";
pub const FIELD_SILENT : &str = "silent";
pub const FIELD_SIZE_MULT : &str = "size-mult";
pub const FIELD_SIZE_INCR : &str = "size-add";


pub const CHEESE_SIZE_MAX : u32 = 555_555_555;

pub const CHEESE_MAX_CONCURRENT : usize = 555;
pub const CHEESE_FLAG_SILENT : u64 = 1;
pub const CHEESE_FLAG_EXCLUSIVE : u64 = 2;
pub const CHEESE_DEFAULT : &[u8] = b"cheese";
pub const CHEESE_RADICAL_MULT : f32 = 1.1;


pub const DEBUG_FLOOD_ALL_STREAMS : bool = true;//sets all streams to 0-0 as their last id and disables last id saving
