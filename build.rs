use std::path::Path;

use gvdb::gresource;

fn main() {
    println!("cargo::rerun-if-changed=data");
    let xml =
        gresource::XmlManifest::from_file(&Path::new("data").join("demoplayer.gresource.xml"))
            .unwrap();
    let builder = gresource::BundleBuilder::from_xml(xml).unwrap();
    std::fs::write(
        Path::new(&std::env::var("OUT_DIR").unwrap()).join("demoplayer.gresource"),
        builder.build().unwrap(),
    )
    .unwrap();

    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("data/logo.ico")
        .compile()
        .expect("Failed compiling windows resource");
}
