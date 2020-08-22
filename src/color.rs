pub enum Color {
    Purple,
    Green,
    Red,
    Blue
}

impl std::str::FromStr for Color {
    type Err = std::string::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "purple" => Ok(Color::Purple),
            "green" => Ok(Color::Green),
            "red" => Ok(Color::Red),
            "blue" => Ok(Color::Blue),
            _ => Ok(Color::Purple),
        }
    }
}

impl Color {
    pub fn hex_color(&self) -> u32 {
        match self {
            Color::Purple => 0xaf12e8,
            Color::Blue => 0x0000ff,
            Color::Green => 0x008000,
            Color::Red => 0xff0000,
        }
    }
}