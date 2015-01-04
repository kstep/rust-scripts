#![feature(slicing_syntax)]
#![feature(old_orphan_check)]

extern crate "rustc-serialize" as serialize;
extern crate collections;

#[cfg(test)]
extern crate test;

use std::io::fs::PathExtensions;
use std::os;
use std::io;
use std::fmt;

#[cfg(test)]
use test::Bencher;

fn automount_name() -> String {
    os::getenv("ID_FS_LABEL").or_else(|| os::getenv("ID_FS_UUID")).unwrap_or_else(|| {
        format!("{}_{}_{}", os::getenv("ID_VENDOR").unwrap(), os::getenv("ID_MODEL").unwrap(), os::args()[1])
    })
}

fn ismount(dir: &str) -> bool {
    let path = Path::new(dir);
    let stat = match path.stat() {
        Ok(s) => s,
        Err(_) => return false
    };

    stat.kind == io::FileType::Directory && (path == path.dir_path() || stat.unstable.device != (match path.dir_path().stat() { Ok(s) => s, Err(_) => return false }).unstable.device)
}

fn systemd_encode(inp: &str) -> String {
    let mut out = String::new();
    for &b in inp.as_bytes().iter() {
        if ('a' as u8) <= b && b <= ('z' as u8)
            || ('A' as u8) <= b && b <= ('Z' as u8)
            || ('0' as u8) <= b && b <= ('9' as u8)
            || b == ('_' as u8) { unsafe{ out.as_mut_vec().push(b); } }
        else {
            out.push_str(r"\x");
            out.push_str(fmt::radix(b, 16).to_string()[]);
        }
    }
    out
}

fn main() {
    let mut name = automount_name();

    while ismount(format!("/media/{}", name)[]) {
        name = name + "_";
    }

    let service_name = format!("{} /media/{}", os::getenv("DEVNAME").unwrap(), name);

    io::stdio::println(name[]);
    io::stdio::println(systemd_encode(service_name[])[]);
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
    assert_eq!(systemd_encode("hello_W0rld")[], "hello_W0rld");
    assert_eq!(systemd_encode(r"/dev/sda1 /media/path")[], r"\x2fdev\x2fsda1\x20\x2fmedia\x2fpath");
}

#[test]
fn test_automount_name() {
    // TODO: how to fake os::args()?
    //os::setenv("ID_VENDOR", "Vendor");
    //os::setenv("ID_MODEL", "Model");
    //assert_eq!(automount_name()[], "Vendor_Model_1");

    os::setenv("ID_FS_UUID", "UUID");
    assert_eq!(automount_name()[], "UUID");

    os::setenv("ID_FS_LABEL", "LABEL");
    assert_eq!(automount_name()[], "LABEL");
}

#[bench]
fn bench_systemd_encode(b: &mut Bencher) {
    b.iter(|| {
        systemd_encode(r"/dev/sda1 /media/path")
    });
}

#[bench]
fn bench_ismount(b: &mut Bencher) {
    b.iter(|| {
        ismount("/tmp")
    });
}

#[bench]
fn bench_automount_name_label(b: &mut Bencher) {
    os::setenv("ID_FS_LABEL", "LABEL");
    b.iter(|| {
        automount_name()
    });
}

#[bench]
fn bench_automount_name_uuid(b: &mut Bencher) {
    os::setenv("ID_FS_UUID", "UUID");
    b.iter(|| {
        automount_name()
    });
}
