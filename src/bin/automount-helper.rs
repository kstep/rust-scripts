#![cfg_attr(test, feature(test))]

#[cfg(test)]
extern crate test;

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::fs::metadata;
use std::os::unix::fs::MetadataExt;

#[cfg(test)]
use test::Bencher;

fn automount_name() -> String {
    env::var("ID_FS_LABEL").or_else(|_| env::var("ID_FS_UUID")).unwrap_or_else(|_| {
        format!("{}_{}_{}",
                env::var("ID_VENDOR").expect("ID_VENDOR env var"),
                env::var("ID_MODEL").expect("ID_MODEL env var"),
                env::args().nth(1).expect("device name is missing"))
    })
}

fn ismount(dir: &str) -> bool {
    let path = Path::new(dir);

    path.is_dir() &&
    {
        (match metadata(match path.parent() {
            Some(p) => p,
            None => return true,
        }) {
            Ok(s) => s,
            Err(_) => return false,
        })
        .dev() !=
        (match metadata(path) {
            Ok(s) => s,
            Err(_) => return false,
        })
        .dev()
    }

}

struct SystemdEscape<I: Iterator<Item=u8>> {
    iter: I,
    buf: Option<[u8; 3]>,
    idx: i8,
}

impl<I: Iterator<Item=u8>> SystemdEscape<I> {
    fn new<II: IntoIterator<Item=u8, IntoIter=I>>(iter: II) -> SystemdEscape<I> {
        SystemdEscape {
            iter: iter.into_iter(),
            buf: None,
            idx: 0,
        }
    }

    fn hex(x: u8) -> u8 {
        match x {
            0x0...0x9 => x | 0x30,
            0xa...0xf => x + 0x57,
            _ => unreachable!()
        }
    }
}

impl<I: Iterator<Item=u8>> Iterator for SystemdEscape<I> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        match self.buf {
            Some(cs) if self.idx < 2 => {
                self.idx += 1;
                return Some(cs[self.idx as usize] as char);
            }
            Some(_) => {
                self.buf = None;
            }
            None => ()
        }

        match self.iter.next() {
            None => None,
            Some(c) => match c {
                b'a'...b'z' | b'A'...b'Z' | b'0'...b'9' | b'_' => Some(c as char),
                _ => {
                    self.buf = Some([b'x', Self::hex((c & 0xf0) >> 4), Self::hex(c & 0x0f)]);
                    self.idx = -1;
                    Some('\\')
                }
            }
        }
    }
}

fn systemd_encode(inp: &str) -> String {
    SystemdEscape::new(inp.as_bytes().into_iter().cloned()).collect()
}

fn main() {
    let mut name = automount_name();

    while ismount(&*format!("/media/{}", name)) {
        name = name + "_";
    }

    let service_name = format!("{} /media/{}", env::var("DEVNAME").unwrap(), name);

    let mut out = io::stdout();
    out.write_all(name.as_bytes()).unwrap();
    out.write(&[0x0a]).unwrap();
    out.write_all(systemd_encode(&*service_name).as_bytes()).unwrap();
    out.write(&[0x0a]).unwrap();
    out.flush().unwrap();
}

#[test]
fn test_ismount() {
    assert_eq!(ismount("/"), true);
    assert_eq!(ismount("/tmp"), true);
    assert_eq!(ismount("/non-existant"), false);
    assert_eq!(ismount("/usr/bin"), false);
}

#[test]
fn test_systemd_encode() {
    assert_eq!(&*systemd_encode("hello_W0rld"), "hello_W0rld");
    assert_eq!(&*systemd_encode(r"/dev/sda1 /media/path"),
               r"\x2fdev\x2fsda1\x20\x2fmedia\x2fpath");
}

#[test]
fn test_automount_name() {
    env::remove_var("ID_FS_UUID");
    env::remove_var("ID_FS_LABEL");

    // TODO: how to fake os::args()?
    // env::set_var("ID_VENDOR", "Vendor");
    // env::set_var("ID_MODEL", "Model");
    // assert_eq!(&*automount_name(), "Vendor_Model_1");

    env::set_var("ID_FS_UUID", "UUID");
    assert_eq!(&*automount_name(), "UUID");

    env::set_var("ID_FS_LABEL", "LABEL");
    assert_eq!(&*automount_name(), "LABEL");
}

#[bench]
fn bench_systemd_encode(b: &mut Bencher) {
    b.iter(|| systemd_encode(r"/dev/sda1 /media/path"));
}

#[bench]
fn bench_ismount(b: &mut Bencher) {
    b.iter(|| ismount("/tmp"));
}

#[bench]
fn bench_automount_name_label(b: &mut Bencher) {
    env::set_var("ID_FS_LABEL", "LABEL");
    env::remove_var("ID_FS_UUID");
    b.iter(|| automount_name());
}

#[bench]
fn bench_automount_name_uuid(b: &mut Bencher) {
    env::set_var("ID_FS_UUID", "UUID");
    env::remove_var("ID_FS_LABEL");
    b.iter(|| automount_name());
}
