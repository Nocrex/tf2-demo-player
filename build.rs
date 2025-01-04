use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=data");
    Command::new("glib-compile-resources")
        .arg(format!(
            "--target={}/demoplayer.gresource",
            std::env::var("OUT_DIR").unwrap()
        ))
        .arg("demoplayer.gresource.xml")
        .current_dir("data")
        .output()
        .inspect(|o| assert!(o.status.success()))
        .expect("Resource compilation failed");
}
