use anyhow::{Context, Result, anyhow};
use iced::{Size, window};
use image::ImageFormat;

const ICON_PNG: &[u8] = include_bytes!("../../icons/128x128@2x.png");

pub fn window_settings() -> Result<window::Settings> {
    let settings = window::Settings {
        size: Size::new(1360.0, 860.0),
        min_size: Some(Size::new(980.0, 640.0)),
        icon: Some(application_icon()?),
        ..window::Settings::default()
    };

    #[cfg(target_os = "linux")]
    let settings = window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: "meridian".to_owned(),
            ..window::settings::PlatformSpecific::default()
        },
        ..settings
    };

    Ok(settings)
}

fn application_icon() -> Result<window::Icon> {
    let image = image::load_from_memory_with_format(ICON_PNG, ImageFormat::Png)
        .context("failed to decode the embedded Meridian application icon")?
        .into_rgba8();
    let (width, height) = image.dimensions();
    window::icon::from_rgba(image.into_raw(), width, height)
        .map_err(|error| anyhow!("invalid Meridian application icon: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_is_valid_rgba() -> Result<()> {
        let icon = application_icon()?;
        let (rgba, size) = icon.into_raw();

        assert_eq!(size, Size::new(256, 256));
        assert_eq!(rgba.len(), 256 * 256 * 4);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_application_id_matches_desktop_file() -> Result<()> {
        assert_eq!(
            window_settings()?.platform_specific.application_id,
            "meridian"
        );
        Ok(())
    }
}
