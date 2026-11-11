//! Viewer-only backend selection from `[renderer] backend` in local.toml.

use std::{error::Error, fmt, fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backend {
    #[default]
    Auto,
    Vulkan,
    OpenGl,
}

#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for ConfigError {}

impl Backend {
    /// Auto keeps the exact defaults of `new_with_display_handle`.
    pub fn apply(self, descriptor: &mut wgpu::InstanceDescriptor) {
        match self {
            Self::Auto => {}
            Self::Vulkan => descriptor.backends = wgpu::Backends::VULKAN,
            Self::OpenGl => descriptor.backends = wgpu::Backends::GL,
        }
    }
}

/// Read the existing local config. Missing renderer section/key means auto.
/// Accepts one quoted string and an optional trailing comment; malformed or
/// duplicate backend entries are errors rather than silent fallback.
pub fn from_file(path: &Path) -> Result<Backend, Box<dyn Error>> {
    Ok(parse(&fs::read_to_string(path)?)?)
}

fn parse(text: &str) -> Result<Backend, ConfigError> {
    let mut renderer = false;
    let mut result = None;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            let section = line.split('#').next().unwrap_or("").trim();
            renderer = section == "[renderer]";
            continue;
        }
        if !renderer || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "backend" {
            continue;
        }
        if result.is_some() {
            return Err(ConfigError("duplicate renderer.backend".into()));
        }
        let value = value.trim();
        let Some(rest) = value.strip_prefix('"') else {
            return Err(ConfigError(format!(
                "renderer.backend must be a quoted string on line {}",
                index + 1
            )));
        };
        let Some((value, tail)) = rest.split_once('"') else {
            return Err(ConfigError(format!(
                "unterminated renderer.backend on line {}",
                index + 1
            )));
        };
        let tail = tail.trim();
        if !tail.is_empty() && !tail.starts_with('#') {
            return Err(ConfigError(format!(
                "invalid renderer.backend on line {}",
                index + 1
            )));
        }
        result = Some(match value {
            "auto" => Backend::Auto,
            "vulkan" => Backend::Vulkan,
            "opengl" => Backend::OpenGl,
            _ => {
                return Err(ConfigError(format!(
                    "unsupported renderer.backend {value:?}; expected auto, vulkan, or opengl"
                )));
            }
        });
    }
    Ok(result.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_config_defaults_and_selects_explicit_backends() {
        assert_eq!(
            parse("[paths]\nbackend = \"vulkan\"\n").unwrap(),
            Backend::Auto
        );
        assert_eq!(
            parse("[renderer] # comment\nbackend = \"auto\"\n").unwrap(),
            Backend::Auto
        );
        assert_eq!(
            parse("[renderer]\nbackend = \"vulkan\" # comment\n").unwrap(),
            Backend::Vulkan
        );
        assert_eq!(
            parse("[renderer]\nbackend = \"opengl\"\n").unwrap(),
            Backend::OpenGl
        );
        for text in [
            "[renderer]\nbackend = vulkan",
            "[renderer]\nbackend = \"bad\"",
            "[renderer]\nbackend = \"vulkan\" extra",
            "[renderer]\nbackend = \"auto\"\nbackend = \"opengl\"",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn auto_preserves_descriptor_and_explicit_choices_restrict_it() {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        let default = descriptor.backends;
        Backend::Auto.apply(&mut descriptor);
        assert_eq!(descriptor.backends, default);
        Backend::Vulkan.apply(&mut descriptor);
        assert_eq!(descriptor.backends, wgpu::Backends::VULKAN);
        Backend::OpenGl.apply(&mut descriptor);
        assert_eq!(descriptor.backends, wgpu::Backends::GL);
    }
}
