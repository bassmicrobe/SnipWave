fn main() {
    // T6 で assets/icons/windows/icon.ico を追加したあとに exe アイコンを有効にする。
    #[cfg(target_os = "windows")]
    {
        const ICON_PATH: &str = "assets/icons/windows/icon.ico";
        println!("cargo:rerun-if-changed={ICON_PATH}");
        if std::path::Path::new(ICON_PATH).exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(ICON_PATH);
            if let Err(err) = res.compile() {
                panic!("Windows リソースのコンパイルに失敗しました: {err}");
            }
        }
    }
}
