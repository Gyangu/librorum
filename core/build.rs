use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "../shared/src/proto/node.proto",
                "../shared/src/proto/file.proto", 
                "../shared/src/proto/log.proto",
            ],
            &["../shared/src/proto"],
        )?;
    
    // Copy the compiled backend binary to the Swift project if it exists
    if let Ok(target_dir) = env::var("OUT_DIR") {
        let mut target_path = PathBuf::from(target_dir);
        // Navigate to the target directory
        while target_path.file_name() != Some(std::ffi::OsStr::new("target")) {
            if !target_path.pop() {
                break;
            }
        }
        
        // Get the profile (debug or release)
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        let binary_path = target_path.join(profile).join("librorum-daemon");
        
        // Copy to Swift client if exists
        let swift_resources = PathBuf::from("../client/librorum/Resources");
        if swift_resources.exists() {
            let dest_path = swift_resources.join("librorum_backend");
            if binary_path.exists() {
                let _ = std::fs::copy(&binary_path, &dest_path);
                
                // Set executable permissions on Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&dest_path) {
                        let mut permissions = metadata.permissions();
                        permissions.set_mode(0o755);
                        let _ = std::fs::set_permissions(&dest_path, permissions);
                    }
                }
            }
        }
    }
    
    Ok(())
}