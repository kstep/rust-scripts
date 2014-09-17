extern crate url;

use std::io::fs;
use std::os;
use url::Url;

#[allow(dead_code)]
#[packed]
struct NginxCacheHeader {
    unknown: [u8, ..24],
    magic: u32, // '\nKEY'
    delim: u16 // ': '
}
// or maybe { unknown: [u8, ..22], magic: u64 ('\0\0\nKEY: ') }
static HEADER_SIZE: uint = 30;

impl NginxCacheHeader {
    fn check_magic(&self) -> bool {
        self.magic == 0x59454b0a
    }
}

fn read_line(reader: &mut Reader) -> Option<String> {
    let mut buf = Vec::<u8>::with_capacity(64);
    while reader.push(64, &mut buf).ok().unwrap_or(0) > 0 {
        if buf.contains(&0x0a) {
            buf.iter().position(|c| *c == 0x0a).map(|p| buf.truncate(p));
            return String::from_utf8(buf).ok();
        }
    }

    None
}

fn main() {
    let args = os::args();
    let root = Path::new(if args.len() < 2 { "/var/lib/nginx/cache" } else { args[1].as_slice() });
    let mut files = fs::walk_dir(&root).unwrap_or_else(|e| fail!("Nginx cache dir access error: {}", e)).filter(|p| p.is_file())
        .filter_map(|p| fs::File::open(&p).ok()
                    .and_then(|mut f| f.read_exact(HEADER_SIZE).ok()
                         .and_then(|d| if unsafe { (*(d.as_ptr() as *const NginxCacheHeader)).check_magic() }
                                   { read_line(&mut f)
                                       .and_then(|u| Url::parse(u.as_slice()).ok())
                                       .map(|u| (Path::new(p.as_vec()), u))
                                   } else { None })));

    for f in files {
        match f {
            (p, u) => println!("{} -> {}", p.display(), u),
        }
    }
}
