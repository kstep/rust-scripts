#![allow(unstable)]

extern crate inotify;

use inotify::{INotify, ffi};
use std::path::Path;

fn main() {
    let mut inotify = INotify::init().unwrap();
    let watcher = inotify.add_watch(&Path::new("/home/kstep/.config/vimb/queue"), ffi::IN_CLOSE_WRITE).unwrap();

    loop {
        for ev in inotify.wait_for_events().unwrap().iter() {
            println!("{:?}", ev);
        }
    }
}
