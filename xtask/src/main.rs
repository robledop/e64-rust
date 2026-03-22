use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

const TARGET: &str = "x86_64-unknown-none";
const BUILD_DIR: &str = "build";
const ISO_ROOT: &str = "build/iso_root";
const ISO_IMAGE: &str = "build/e64-rust.iso";
const LIMINE_VERSION: &str = "10.3.2";

fn main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        args.push("run".to_string());
    }

    match args[0].as_str() {
        "iso" => build_iso()?,
        "run" => {
            let extra = args.split_off(1);
            build_iso()?;
            run_qemu(&extra)?;
        }
        "debug" => {
            build_iso()?;
            debug_qemu()?;
        }
        "clean" => clean()?,
        "limine" => build_limine()?,
        _ => print_usage(),
    }

    Ok(())
}

fn print_usage() {
    eprintln!("Usage: cargo run -p xtask -- <iso|run|debug|clean|limine> [qemu args]");
}

fn build_iso() -> Result<()> {
    build_kernel()?;
    build_limine()?;
    stage_iso_contents()?;
    make_iso()?;
    Ok(())
}

fn build_kernel() -> Result<()> {
    run(Command::new("cargo")
        .arg("build")
        .arg("--target")
        .arg(TARGET))
    .context("building kernel")
}

fn build_limine() -> Result<()> {
    download_limine()?;
    extract_limine()?;
    configure_limine()?;
    run(Command::new("make").current_dir(limine_dir())).context("building Limine artifacts")
}

fn download_limine() -> Result<()> {
    let tarball = limine_tarball();
    if tarball.exists() {
        return Ok(());
    }

    fs::create_dir_all(BUILD_DIR).context("creating build dir")?;
    run(Command::new("curl")
        .arg("-L")
        .arg(format!(
            "https://github.com/limine-bootloader/limine/releases/download/v{v}/limine-{v}.tar.xz",
            v = LIMINE_VERSION
        ))
        .arg("-o")
        .arg(&tarball))
    .context("downloading Limine release")?;
    Ok(())
}

fn extract_limine() -> Result<()> {
    let dir = limine_dir();
    if dir.exists() {
        return Ok(());
    }

    fs::create_dir_all(BUILD_DIR).context("creating build dir")?;
    run(Command::new("tar")
        .arg("-xJf")
        .arg(limine_tarball())
        .arg("-C")
        .arg(BUILD_DIR))
    .context("extracting Limine")?;
    Ok(())
}

fn configure_limine() -> Result<()> {
    let dir = limine_dir();
    if dir.join("config.status").exists() {
        return Ok(());
    }

    run(Command::new("./configure").current_dir(&dir).args([
        "--enable-bios",
        "--enable-bios-cd",
        "--enable-uefi-x86-64",
        "--enable-uefi-cd",
    ]))
    .context("configuring Limine")?;

    Ok(())
}

fn stage_iso_contents() -> Result<()> {
    let root = Path::new(ISO_ROOT);
    let limine_bin = limine_dir().join("bin");
    if root.exists() {
        fs::remove_dir_all(root).context("cleaning old ISO root")?;
    }
    fs::create_dir_all(root.join("EFI/BOOT")).context("creating ISO tree")?;

    let kernel = format!("target/{TARGET}/debug/e64-rust");
    fs::copy(&kernel, root.join("kernel.elf")).context("copying kernel")?;
    fs::copy("limine.conf", root.join("limine.conf")).context("copying limine.conf")?;

    for (src, dst) in [
        ("limine-bios.sys", "limine-bios.sys"),
        ("limine-bios-cd.bin", "limine-bios-cd.bin"),
        ("limine-uefi-cd.bin", "limine-uefi-cd.bin"),
        ("BOOTX64.EFI", "EFI/BOOT/BOOTX64.EFI"),
    ] {
        fs::copy(limine_bin.join(src), root.join(dst))
            .with_context(|| format!("copying {src} from Limine bin dir"))?;
    }

    Ok(())
}

fn make_iso() -> Result<()> {
    run(Command::new("xorriso").args([
        "-as",
        "mkisofs",
        "-R",
        "-b",
        "limine-bios-cd.bin",
        "-no-emul-boot",
        "-boot-load-size",
        "4",
        "-boot-info-table",
        "--efi-boot",
        "limine-uefi-cd.bin",
        "-efi-boot-part",
        "--efi-boot-image",
        "--protective-msdos-label",
        ISO_ROOT,
        "-o",
        ISO_IMAGE,
    ]))
    .context("creating ISO with xorriso")?;

    run(Command::new(limine_dir().join("bin/limine"))
        .arg("bios-install")
        .arg(ISO_IMAGE))
    .context("running limine bios-install")?;

    Ok(())
}

fn run_qemu(extra: &[String]) -> Result<()> {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-cdrom",
        ISO_IMAGE,
        "-m",
        "512M",
        "-serial",
        "stdio",
        "-no-reboot",
        "-no-shutdown",
    ]);
    cmd.args(extra);
    run(&mut cmd).context("running qemu")?;
    Ok(())
}

fn debug_qemu() -> Result<()> {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-cdrom",
        ISO_IMAGE,
        "-m",
        "512M",
        "-serial",
        "stdio",
        "-no-reboot",
        "-no-shutdown",
        "-s",
        "-S",
    ]);
    cmd.spawn().context("spawning qemu for debugging")?;
    Ok(())
}

fn clean() -> Result<()> {
    if Path::new(BUILD_DIR).exists() {
        fs::remove_dir_all(BUILD_DIR).context("removing build dir")?;
    }
    run(Command::new("cargo").arg("clean")).context("cargo clean")?;
    Ok(())
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status()?;
    if !status.success() {
        bail!("command failed: {:?}", cmd);
    }
    Ok(())
}

fn limine_tarball() -> PathBuf {
    PathBuf::from(format!("{BUILD_DIR}/limine-{LIMINE_VERSION}.tar.xz"))
}

fn limine_dir() -> PathBuf {
    PathBuf::from(format!("{BUILD_DIR}/limine-{LIMINE_VERSION}"))
}
