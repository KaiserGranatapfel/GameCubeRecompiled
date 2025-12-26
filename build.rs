// Build script for cross-platform compilation
fn main() {
    // Set up platform-specific configurations
    let target = std::env::var("TARGET").unwrap();
    
    println!("cargo:warning=Building for target: {}", target);
    
    // Add platform-specific linker flags
    if target.contains("windows") {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:CONSOLE");
    }
    
    if target.contains("apple") {
        // macOS specific settings
        println!("cargo:rustc-link-arg=-framework");
        println!("cargo:rustc-link-arg=CoreFoundation");
    }
}

