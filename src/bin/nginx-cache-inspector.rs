#![feature(fs_walk, path_ext)]

extern crate url;

use std::path::Path;
use std::fs::{File, PathExt, walk_dir};
use std::io::{BufReader, BufRead, Read};
use std::env;
use std::mem::transmute;
use url::Url;

#[allow(dead_code)]
struct NginxCacheHeader {
    unknown: [u8; 24],
    magic: u32, // '\nKEY'
    delim: u16 // ': '
}
// or maybe { unknown: [u8; 22], magic: u64 ('\0\0\nKEY: ') }
const HEADER_SIZE: usize = 32;

impl NginxCacheHeader {
    fn check_magic(&self) -> bool {
        self.magic == 0x59454b0a
    }
}

fn main() {
    let args : Vec<String> = env::args().collect();
    let root = Path::new(if args.len() < 2 { "/var/lib/nginx/cache" } else { &*args[1] });
    let files = walk_dir(&root).unwrap_or_else(|e| panic!("Nginx cache dir access error: {}", e))
        .filter_map(|e| e.ok())
        .map(|e| e.path()).filter(|p| p.is_file())
        .filter_map(|p| File::open(&p).ok().map(|f| BufReader::new(f))
                    .and_then(|mut f| {
                        let mut buf = [0u8; HEADER_SIZE];
                        f.read(&mut buf).unwrap();
                        let hdr: NginxCacheHeader = unsafe { transmute(buf) };
                        if hdr.check_magic() {
                            let mut u = String::new();
                            f.read_line(&mut u).ok().and_then(|_| Url::parse(&*u).ok())
                                .map(|u| (Path::new(&p).to_owned(), u))
                        } else {
                            None
                        }
                    }));

    for f in files {
        match f {
            (p, u) => println!("{} -> {}", p.display(), u),
        }
    }
}

