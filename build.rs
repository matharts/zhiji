/// 跟踪 Windows 资源变化，并仅在 Windows 目标构建时嵌入应用图标。
fn main() {
    println!("cargo:rerun-if-changed=windows.rc");
    println!("cargo:rerun-if-changed=assets/icons/app.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("windows.rc", embed_resource::NONE)
            .manifest_required()
            .expect("embed the Windows application icon");
    }
}
