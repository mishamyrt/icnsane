use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use objc2::{AnyThread, rc::autoreleasepool};
use objc2_app_kit::{NSBitmapImageRep, NSColorSpace, NSWorkspace};
use objc2_foundation::NSString;
use tempfile::tempdir;

fn bundle(root: &Path, name: &str) -> PathBuf {
    let app = root.join(name);
    fs::create_dir_all(app.join("Contents")).unwrap();
    fs::write(
        app.join("Contents/Info.plist"),
        r#"<plist version="1.0"><dict>
        <key>CFBundlePackageType</key><string>APPL</string>
        </dict></plist>"#,
    )
    .unwrap();
    app
}

fn image(path: &Path, format: &str, rgb: [u8; 3]) {
    let ppm = path.with_extension("ppm");
    let mut bytes = b"P6\n512 512\n255\n".to_vec();
    bytes.extend(std::iter::repeat_n(rgb, 512 * 512).flatten());
    fs::write(&ppm, bytes).unwrap();
    let result = Command::new("/usr/bin/sips")
        .args(["-s", "format", format])
        .arg(ppm)
        .arg("--out")
        .arg(path)
        .output()
        .unwrap();
    assert_status(&result, true);
}

fn assert_status(output: &Output, success: bool) {
    assert_eq!(
        output.status.success(),
        success,
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn icon_color(app: &Path) -> (f64, f64) {
    let app = fs::canonicalize(app).unwrap();
    autoreleasepool(|_| {
        let path = NSString::from_str(app.to_str().unwrap());
        let icon = NSWorkspace::sharedWorkspace().iconForFile(&path);
        let data = icon.TIFFRepresentation().unwrap();
        let bitmap = NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &data).unwrap();
        let color = bitmap
            .colorAtX_y(bitmap.pixelsWide() / 2, bitmap.pixelsHigh() / 2)
            .unwrap()
            .colorUsingColorSpace(&NSColorSpace::deviceRGBColorSpace())
            .unwrap();
        (color.redComponent(), color.greenComponent())
    })
}

#[test]
fn apply_and_reset_a_batch() {
    let temp = tempdir().unwrap();
    let home = temp.path();
    let config_dir = home.join("config");
    let icons = config_dir.join("icons");
    fs::create_dir_all(&icons).unwrap();
    let config = config_dir.join("settings.toml");
    let png_app = bundle(home, "PNG пример.app");
    let icns_app = bundle(home, "ICNS.app");
    image(&icons.join("pink.png"), "png", [255, 8, 150]);
    image(&icons.join("green.icns"), "icns", [10, 230, 8]);

    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_icnsane"))
            .current_dir(home)
            .env("HOME", home)
            .arg("--config")
            .arg(&config)
            .args(args)
            .output()
            .unwrap()
    };
    let valid = r#"
icons_dir = "./icons"
[apps]
"~/PNG пример.app" = "pink.png"
"~/ICNS.app" = "green.icns"
"#;
    fs::write(&config, valid).unwrap();
    let dry_run = run(&["apply", "--dry-run"]);
    assert_status(&dry_run, true);
    assert_eq!(
        String::from_utf8_lossy(&dry_run.stdout)
            .matches("would apply")
            .count(),
        2
    );
    assert!(!png_app.join("Icon\r").exists());
    assert!(!icns_app.join("Icon\r").exists());

    fs::write(&config, format!(r#"{valid}"~/Missing.app" = "pink.png""#)).unwrap();
    let partial = run(&["apply"]);
    assert_status(&partial, false);
    assert!(String::from_utf8_lossy(&partial.stderr).contains("Missing.app"));
    let (red, green) = icon_color(&png_app);
    assert!(
        red - green > 0.5,
        "expected pink icon, got ({red}, {green})"
    );
    let (red, green) = icon_color(&icns_app);
    assert!(
        green - red > 0.5,
        "expected green icon, got ({red}, {green})"
    );

    fs::write(&config, valid.replace("pink.png", "green.icns")).unwrap();
    for _ in 0..2 {
        assert_status(&run(&["apply"]), true);
        let (red, green) = icon_color(&png_app);
        assert!(
            green - red > 0.5,
            "expected green icon, got ({red}, {green})"
        );
    }

    assert_status(&run(&["reset", "~/PNG пример.app"]), true);
    assert!(!png_app.join("Icon\r").exists());
    assert!(icns_app.join("Icon\r").exists());
    fs::remove_file(icons.join("green.icns")).unwrap();
    assert_status(&run(&["reset", "--all"]), true);
    assert!(!icns_app.join("Icon\r").exists());
}

#[test]
fn conflicting_paths_and_invalid_images_do_not_write_icons() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    let app = bundle(root, "Example.app");
    let invalid = bundle(root, "Invalid.app");
    symlink(&app, root.join("Alias.app")).unwrap();
    image(&root.join("valid.png"), "png", [30, 70, 240]);
    fs::write(root.join("broken.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    fs::write(
        root.join("icnsane.toml"),
        r#"
icons_dir = "."
[apps]
"~/Example.app" = "valid.png"
"~/Alias.app" = "valid.png"
"~/Invalid.app" = "broken.png"
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_icnsane"))
        .current_dir(root)
        .env("HOME", root)
        .arg("apply")
        .output()
        .unwrap();
    assert_status(&output, false);
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        errors.contains("multiple configuration entries"),
        "{errors}"
    );
    assert!(errors.contains("could not decode"), "{errors}");
    assert!(!app.join("Icon\r").exists());
    assert!(!invalid.join("Icon\r").exists());
}
