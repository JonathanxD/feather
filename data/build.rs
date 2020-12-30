use anyhow::Context;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{copy, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    }
}

fn run() -> anyhow::Result<()> {
    let path = format!("{}/minecraft", env::var("OUT_DIR")?);
    let path_1_15 = format!("{}/minecraft-1.15", env::var("OUT_DIR")?);

    download_version("https://launcher.mojang.com/v1/objects/3737db93722a9e39eeada7c27e7aca28b144ffa7/server.jar", &path, true).context("failed to download 1.13 data")?;
    download_version("https://launcher.mojang.com/v1/objects/bb2b6b1aefcd70dfd1892149ac3a215f6c636b07/server.jar", &path_1_15, false).context("failed to download 1.15 data")?;

    clone_minecraft_data().context("failed to clone PrismarineJS/minecraft-data")?;

    println!(
        "cargo:rerun-if-changed={}",
        concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs")
    );
    Ok(())
}

fn download_version(url: &str, path: &str, do_generate: bool) -> anyhow::Result<()> {
    let path = Path::new(&path);
    let path_server = path.join("server.jar");

    if data_exists(path).unwrap_or(false) {
        println!("cargo:rerun-if-changed={}", &path.display());
        println!(
            "cargo:rerun-if-changed={}",
            concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs")
        );
        return Ok(());
    }

    let _ = fs::remove_dir_all(path);
    fs::create_dir_all(path).context("failed to create target directory for downloaded data")?;

    download(url, &path_server).context("failed to download vanilla server JAR")?;

    println!(
        "after download: {:?}",
        std::fs::read_dir(path)?.collect::<Vec<_>>()
    );

    if do_generate {
        generate(path).context("failed to generate vanilla server reports.")?;
    }

    extract(path).context("failed to extract vanilla assets.")?;
    println!(
        "after extract: {:?}",
        std::fs::read_dir(path)?.collect::<Vec<_>>()
    );

    Ok(())
}

fn data_exists(path: &Path) -> anyhow::Result<bool> {
    Ok(File::open(path.join("server.jar")).is_ok()
        && File::open(path.join("assets")).is_ok()
        && File::open(path.join("data")).is_ok()
        && File::open(path.join("generated")).is_ok())
}

fn download<P: AsRef<Path>>(url: &str, server: P) -> anyhow::Result<()> {
    let mut response = reqwest::blocking::get(url)?;
    let mut dest = File::create(server)
        .context("failed to create destination file for server JAR download")?;
    copy(&mut response, &mut dest)?;
    dest.flush()?;
    Ok(())
}

fn java_home_bin_path() -> Option<PathBuf> {
    env::var("JAVA_HOME")
        .map(|java_home| PathBuf::from(java_home).join("bin").join("java"))
        .ok()
}

fn find_java() -> anyhow::Result<PathBuf> {
    let java_paths = [
        "/bin/java",
        "/usr/bin/java",
        "/usr/local/bin/java",
        "/usr/sbin/java",
    ];

    for java_path_str in &java_paths {
        let path = Path::new(java_path_str);

        if path.exists() {
            return Ok(path.to_path_buf());
        }
    }

    let java_from_java_home = java_home_bin_path();

    match java_from_java_home {
        Some(java_bin) => {
            if !java_bin.exists() {
                let mut ancestors = java_bin.ancestors();
                ancestors.next();
                ancestors.next();
                anyhow::bail!(
                    "JAVA_HOME is defined as {}, but Java binary could not be find in the binary path: {}.",
                    ancestors.next().unwrap().display(),
                    java_bin.display()
                )
            } else {
                Ok(java_bin)
            }
        }
        None => anyhow::bail!(
            "Please install Java and define env var 'JAVA_HOME' pointing to installation root path. Java could not be found in any of known paths: {}.",
            java_paths.join(", ")
        )
    }
}

fn generate<P: AsRef<Path>>(working: P) -> anyhow::Result<()> {
    let java_bin = find_java()?;

    let status = Command::new(java_bin)
        .current_dir(working.as_ref())
        .args(&["-cp", "server.jar", "net.minecraft.data.Main", "--reports"])
        .status()?;
    if !status.success() {
        anyhow::bail!(
            "process to generate server reports was not successful (exit status {}, JAR path {})",
            status,
            working.as_ref().display(),
        )
    }
    Ok(())
}

fn extract<P: AsRef<Path>>(working: P) -> anyhow::Result<()> {
    println!(
        "{:?}",
        std::fs::read_dir(working.as_ref())?.collect::<Vec<_>>()
    );
    let server_jar = working.as_ref().join("server.jar");
    let mut archive = ZipArchive::new(std::fs::File::open(server_jar)?)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if !(file.name().starts_with("assets/") || file.name().starts_with("data/")) {
            continue;
        }

        let outpath_name = file.name().replace("..", ".");
        let outpath = working.as_ref().join(outpath_name);

        if file.is_dir() {
            println!("Directory \"{}\" was created", outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!("Writing to \"{}\"", outpath.display(),);
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    Ok(())
}

fn clone_minecraft_data() -> anyhow::Result<()> {
    let path = format!("{}/minecraft-data", env::var("OUT_DIR")?);
    if Path::new(&path).exists() {
        // Already cloned - no need to do so again
        return Ok(());
    }

    if !Command::new("git")
        .arg("clone")
        .arg("https://github.com/PrismarineJS/minecraft-data.git")
        .arg(&path)
        .status()?
        .success()
    {
        Err(anyhow::anyhow!(
            "failed to clone minecraft-data repository: please ensure git is installed"
        ))
    } else {
        Ok(())
    }
}
