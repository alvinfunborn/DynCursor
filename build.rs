fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/logo.ico"); // 图标路径
        res.compile().unwrap();
    }
}