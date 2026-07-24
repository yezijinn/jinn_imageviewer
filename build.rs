use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("app_icon.ico");
        res.compile().expect("Failed to compile Windows resources");
    }

    // 生成编译日期版本号
    let build_date = chrono::Local::now().format("%Y%m%d").to_string();
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = PathBuf::from(&out_dir).join("build_date.rs");
    fs::write(
        &dest_path,
        format!("pub const BUILD_DATE: &str = \"{}\";\n", build_date),
    )
    .expect("Failed to write build_date.rs");
}
