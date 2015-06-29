#!/usr/local/bin/rustrun
//#![feature(plugin)]

//#![plugin(rot)]
//extern crate rot;

//use rot::*;

use std::process::Command;

//#[task]
fn build() {
    Command::new("cargo")
        .arg("build")
        .status()
        .unwrap();
}

//#[task(test)]
fn release() {
    test();
    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .status()
        .unwrap();
}

//#[task]
fn test() {
    Command::new("cargo")
        .arg("test")
        .status()
        .unwrap();
}

//#[task(release)]
fn install() {
    release();
    Command::new("find")
        .arg("./target/release")
        .arg("-maxdepth").arg("1")
        .arg("-type").arg("f")
        .arg("-executable")
        .arg("-exec").arg("install")
            .arg("-D").arg("-v").arg("-s")
            .arg("-o").arg("root")
            .arg("-g").arg("root")
            .arg("-m").arg("0755")
            .arg("-t").arg("/usr/local/bin")
            .arg("{}").arg("+")
        .status()
        .unwrap();
}

use std::env;
fn main() {
    let mut args = env::args();
    args.next(); // skip bin name

    let target = args.next().expect("target name expected");
    match &*target {
        "build" => build(),
        "release" => release(),
        "test" => test(),
        "install" => install(),
        _ => panic!("unknown target name: {}", target),
    }
}
