use std::{fs, path::Path};

use anyhow::{Context, Result, bail, ensure};
use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSImage, NSWorkspace, NSWorkspaceIconCreationOptions};
use objc2_foundation::{NSData, NSString};

pub fn load_icon(path: &Path) -> Result<Retained<NSImage>> {
    let signature: &[u8] = match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("png") => b"\x89PNG\r\n\x1a\n",
        Some(ext) if ext.eq_ignore_ascii_case("icns") => b"icns",
        _ => bail!("expected a .png or .icns icon at {}", path.display()),
    };
    let metadata =
        fs::metadata(path).with_context(|| format!("could not access {}", path.display()))?;
    ensure!(metadata.is_file(), "expected a file at {}", path.display());
    let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
    ensure!(
        bytes.starts_with(signature),
        "image contents do not match the extension of {}",
        path.display()
    );
    let data = NSData::with_bytes(&bytes);
    NSImage::initWithData(NSImage::alloc(), &data)
        .filter(|image| image.isValid())
        .with_context(|| format!("could not decode {}", path.display()))
}

pub fn set_icon(app: &Path, icon: Option<&NSImage>) -> Result<()> {
    let path = app.to_str().context("application path is not UTF-8")?;
    let options = NSWorkspaceIconCreationOptions::empty();
    ensure!(
        NSWorkspace::sharedWorkspace().setIcon_forFile_options(
            icon,
            &NSString::from_str(path),
            options,
        ),
        "macOS could not change the icon (check write access to the application)"
    );
    Ok(())
}
