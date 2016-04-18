extern crate syntex;
extern crate serde_codegen;

use std::env;
use std::path::Path;

pub fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let src_dir = Path::new("src/bin");
    let dst_dir = Path::new(&out_dir);

    for &(src, dst) in [
        ("adslbystat.rs.in", "adslbystat.rs"),
        ("lostfilm-check.rs.in", "lostfilm-check.rs"),
        ("trans-done-pb.rs.in", "trans-done-pb.rs"),
        ("vimb-queue-pocket.rs.in", "vimb-queue-pocket.rs"),
        ("yaddns.rs.in", "yaddns.rs"),
    ].iter() {
        let (src, dst) = (src_dir.join(src), dst_dir.join(dst));

        let mut registry = syntex::Registry::new();
        serde_codegen::register(&mut registry);
        registry.expand("", &src, &dst).unwrap();
    }
}
