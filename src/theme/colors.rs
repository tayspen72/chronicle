use ratatui::style::Color;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ColorError {
    #[error("Invalid hex color: {0}")]
    InvalidHex(String),
    #[error("Unknown color name: {0}")]
    UnknownName(String),
}

pub fn parse_color(value: &str) -> Result<Color, ColorError> {
    let value = value.trim();

    if value.starts_with('#') {
        parse_hex(value)
    } else {
        parse_named(value)
    }
}

fn parse_hex(hex: &str) -> Result<Color, ColorError> {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| ColorError::InvalidHex(hex.to_string()))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| ColorError::InvalidHex(hex.to_string()))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| ColorError::InvalidHex(hex.to_string()))?;
            Ok(Color::Rgb(r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)
                .map_err(|_| ColorError::InvalidHex(hex.to_string()))?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)
                .map_err(|_| ColorError::InvalidHex(hex.to_string()))?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)
                .map_err(|_| ColorError::InvalidHex(hex.to_string()))?;
            Ok(Color::Rgb(r, g, b))
        }
        _ => Err(ColorError::InvalidHex(hex.to_string())),
    }
}

fn parse_named(name: &str) -> Result<Color, ColorError> {
    match name.to_lowercase().as_str() {
        "black" => Ok(Color::Black),
        "white" => Ok(Color::White),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "darkgray" | "dark_gray" => Ok(Color::DarkGray),
        "gray" | "grey" => Ok(Color::Gray),
        "lightred" | "light_red" => Ok(Color::LightRed),
        "lightgreen" | "light_green" => Ok(Color::LightGreen),
        "lightyellow" | "light_yellow" => Ok(Color::LightYellow),
        "lightblue" | "light_blue" => Ok(Color::LightBlue),
        "lightmagenta" | "light_magenta" => Ok(Color::LightMagenta),
        "lightcyan" | "light_cyan" => Ok(Color::LightCyan),
        "reset" => Ok(Color::Reset),
        _ => Err(ColorError::UnknownName(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_full() {
        assert_eq!(parse_hex("#ffffff").unwrap(), Color::Rgb(255, 255, 255));
        assert_eq!(parse_hex("#000000").unwrap(), Color::Rgb(0, 0, 0));
        assert_eq!(parse_hex("#ff0000").unwrap(), Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_parse_hex_short() {
        assert_eq!(parse_hex("#fff").unwrap(), Color::Rgb(255, 255, 255));
        assert_eq!(parse_hex("#000").unwrap(), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_parse_named() {
        assert_eq!(parse_named("white").unwrap(), Color::White);
        assert_eq!(parse_named("dark_gray").unwrap(), Color::DarkGray);
        assert_eq!(parse_named("LightBlue").unwrap(), Color::LightBlue);
    }
}
