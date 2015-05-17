extern crate pb;
extern crate rustc_serialize;
extern crate script_utils as utils;

use pb::{PbAPI, PushMsg, TargetIden, Push, PushData};
use std::env;

#[derive(RustcDecodable)]
struct Config {
    access_token: String
}

fn main() {
    let mut api = PbAPI::new(&*utils::load_config::<Config>("pushbullet/creds.toml").unwrap().access_token);
    let torrent_name = env::var("TR_TORRENT_NAME").unwrap();
    let torrent_dir = env::var("TR_TORRENT_DIR").unwrap();
    let push = PushMsg {
        title: Some("Torrent download complete".to_string()),
        body: Some(format!("{} downloaded to {}", torrent_name, torrent_dir)),
        target: TargetIden::CurrentUser,
        data: PushData::Note,
        source_device_iden: None
    };

    let result: Push = api.send(&push).unwrap();
    println!("notified with push {}", result.iden);
}

