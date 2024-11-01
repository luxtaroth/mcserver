use std::{
    fs::{self},
    io,
    path::{Path, PathBuf},
    process::Command,
};

use reqwest::blocking::get;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::ConfigToml;

pub fn build() {
    // TODO Build this
    let config = read_config().unwrap();
    println!("config: {:?}", config);
    let version = config.mcserver.unwrap().java.unwrap();
    println!("version: {version}");
    let cache = check_jdk_in_cache(version);
    println!("in 23 cache:{}", cache.unwrap());

    let cache = check_jdk_in_cache("22".to_string());
    println!("in 22 cache:{}", cache.unwrap());
}

pub fn check_project_structure() -> bool {
    let required_file = "config.toml";
    let required_dirs = vec!["server", "cache", "log", "java"];

    if !Path::new(required_file).exists() {
        return false;
    }

    for dir in required_dirs {
        if !Path::new(dir).is_dir() {
            return false;
        }
    }

    true
}

pub fn read_config() -> std::result::Result<ConfigToml, Box<dyn std::error::Error>> {
    let config_file = "config.toml";
    let path = Path::new(config_file);

    if !path.exists() {
        return Err("Config file does not exist".into());
    }

    fs::metadata(config_file).map_err(|e| format!("Cannot read config file: {}", e))?;

    let content = fs::read_to_string(path)?;
    let toml: ConfigToml = toml::from_str(&content)?;
    Ok(toml)
}

pub fn check_jdk_in_cache(version: String) -> io::Result<bool> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None => {
            eprintln!("Could not determine home directory");
            return Ok(false);
        }
    };
    println!("home: {:?}", home);

    let cache_dir = home.join(".cache");
    if !cache_dir.exists() {
        eprintln!("Cache directory does not exist");
        return Ok(false);
    }
    println!("cache: {:?}", cache_dir);

    let jdk = format!("jdk-{}.tar.gz", version);
    let jdk_path = cache_dir.join(jdk);
    println!("jdk: {:?}", jdk_path);

    Ok(jdk_path.exists())
}

pub fn download_jdk(java_version: u8) -> io::Result<()> {
    #[derive(Deserialize)]
    struct AvailableReleases {
        available_releases: Vec<u8>,
    }

    let response =
        match reqwest::blocking::get("https://api.adoptium.net/v3/info/available_releases") {
            Ok(res) => res,
            Err(_) => {
                println!("Failed to make the API request, to Adoptium");
                std::process::exit(1);
            }
        };

    let releases: AvailableReleases = match response.json() {
        Ok(json) => json,
        Err(_) => {
            println!("Failed to parse the JSON response, from Adoptium");
            std::process::exit(1);
        }
    };

    if !releases.available_releases.contains(&java_version) {
        println!(
            "Given Java Version : {} is not available by the sources. ",
            java_version
        );
        std::process::exit(0);
    }

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/linux/x64/jdk/hotspot/normal/eclipse",
        java_version
    );

    let output_path = dirs::home_dir().map(|p| p.join(".cache")).unwrap();
    let output_file = output_path.join(format!("jdk-{}.tar.gz", java_version));

    let status = Command::new("curl")
        .arg("-L")
        .arg("--progress-bar")
        .arg(&url)
        .arg("-o")
        .arg(&output_file)
        .status()
        .expect("failed to execute curl");

    if status.success() {
        println!("jdk downloaded successfully at {}", output_file.display());
        Ok(())
    } else {
        eprintln!("Failed to download the jdk");
        std::process::exit(1);
    }
}

fn extract_tar_gz(tar_path: &Path, extract_path: &Path) -> io::Result<()> {
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(tar_path)
        .arg("-C")
        .arg(extract_path)
        .status()
        .expect("Failed to execute tar command");

    if !status.success() {
        println!("Failed to extract tar.gz file");
    }
    Ok(())
}

pub fn checksum_check(jdk_path: PathBuf, version: String) -> io::Result<bool> {
    let url = format!(
        "https://api.adoptium.net/v3/assets/latest/{}/hotspot",
        version
    );

    let response = match get(url) {
        Ok(res) => {
            if !res.status().is_success() {
                eprintln!("API request failed with status: {}", res.status());
                return Ok(false);
            }
            match res.text() {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("Failed to read response text: {}", e);
                    return Ok(false);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to make API request: {}", e);
            return Ok(false);
        }
    };

    let jdk_content = match fs::read(jdk_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to read JDK file: {}", e);
            return Ok(false);
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(&jdk_content);
    let local_hash = format!("{:x}", hasher.finalize());

    let response: serde_json::Value = match serde_json::from_str(&response) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to parse JSON response: {}", e);
            eprintln!("Response content: {}", response);
            return Ok(false);
        }
    };

    let releases = match response.as_array() {
        Some(arr) => arr,
        None => {
            eprintln!("Expected JSON array in response");
            return Ok(false);
        }
    };

    for release in releases {
        if let Some(binaries) = release.get("binaries").and_then(|b| b.as_array()) {
            for binary in binaries {
                if let Some(checksum) = binary
                    .get("package")
                    .and_then(|pkg| pkg.get("checksum"))
                    .and_then(|c| c.as_str())
                {
                    if checksum == local_hash {
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}
