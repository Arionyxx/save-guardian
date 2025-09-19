#[cfg(windows)]
extern crate winresource;

#[cfg(windows)]
fn main() {
    // Only add the icon resource on Windows
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "Save Guardian");
    res.set("FileDescription", "Save Guardian - Game Save Manager");
    res.set("ProductVersion", "1.0.0");
    res.set("FileVersion", "1.0.0");
    res.set("CompanyName", "Save Guardian");
    res.set("LegalCopyright", "© 2025 Save Guardian Contributors");
    
    if let Err(e) = res.compile() {
        println!("cargo:warning=Failed to compile Windows resources: {}", e);
    } else {
        println!("cargo:warning=Successfully embedded Windows resources and icon");
    }
}

#[cfg(not(windows))]
fn main() {
    // Do nothing on non-Windows platforms
}