use std::process::{Command, Stdio};

pub fn install_tailscale() {
    let output = Command::new("curl")
        .arg("-fsSL")
        .arg("https://tailscale.com/install.sh")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().unwrap()
        .wait_with_output().unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        println!("Installation script executed successfully");
    } else {
        println!("Installation script failed with exit code: {}", output.status);
    }
    
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_tailscale() {
        install_tailscale();
        if cfg!(target_os = "linux") {
            if std::fs::File::open("/usr/bin/tailscale").is_ok() {
                assert!(true);
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    }
}