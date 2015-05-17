#![feature(slicing_syntax)]
#![feature(os, path, io, os)]

extern crate url;

use std::path::Path;
use std::fs::{File, walk_dir};
use std::io::BufReader;
use std::env;
use url::Url;

#[allow(dead_code)]
struct NginxCacheHeader {
    unknown: [u8; 24],
    magic: u32, // '\nKEY'
    delim: u16 // ': '
}
// or maybe { unknown: [u8; 22], magic: u64 ('\0\0\nKEY: ') }
static HEADER_SIZE: usize = 30;

impl NginxCacheHeader {
    fn check_magic(&self) -> bool {
        self.magic == 0x59454b0a
    }
}

fn main() {
    let args : Vec<String> = env::args().collect();
    let root = Path::new(if args.len() < 2 { "/var/lib/nginx/cache" } else { &*args[1] });
    let mut files = walk_dir(&root).unwrap_or_else(|e| panic!("Nginx cache dir access error: {}", e)).filter(|p| p.is_file())
        .filter_map(|p| File::open(&p).ok().map(|f| BufReader::new(f))
                    .and_then(|mut f| f.read_exact(HEADER_SIZE).ok()
                         .and_then(|d| if unsafe { (*(d.as_ptr() as *const NginxCacheHeader)).check_magic() }
                                   { f.read_line().ok()
                                       .and_then(|u| Url::parse(&*u).ok())
                                       .map(|u| (Path::new(p.as_vec()), u))
                                   } else { None })));

    for f in files {
        match f {
            (p, u) => println!("{} -> {}", p.display(), u),
        }
    }
}

