#[cfg(windows)]
extern crate winresource;

#[cfg(windows)]
fn main() {
    // Only add the icon resource on Windows
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {
    // Do nothing on non-Windows platforms
}