#![feature(slicing_syntax)]

extern crate pb;

use pb::api::PbAPI;
use pb::messages::{PushMsg, TargetIden};
use pb::objects::{Push, PushData};
use std::os::getenv;

fn main() {
    let api = PbAPI::new(getenv("PB_API_KEY").unwrap()[]);
    let torrent_name = getenv("TR_TORRENT_NAME").unwrap();
    let torrent_dir = getenv("TR_TORRENT_DIR").unwrap();
    let push = PushMsg {
        title: Some("Torrent download complete".to_string()),
        body: Some(format!("{} downloaded to {}", torrent_name, torrent_dir)),
        target: TargetIden::CurrentUser,
        data: PushData::Note,
        source_device_iden: None
    };


    api.send::<Push, PushMsg>(&push).unwrap();
}
