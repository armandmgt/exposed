use std::{io, process::Command};

fn main() -> io::Result<()> {
    Command::new("yarn").args(["run", "build"]).status()?;
    println!("cargo:rerun-if-changed=tailwind.css");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=yarn.lock");
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=postcss.config.js");
    println!("cargo:rerun-if-changed=migrations");
    Ok(())
}
