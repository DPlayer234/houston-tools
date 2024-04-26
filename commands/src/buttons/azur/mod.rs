pub mod ship;

#[derive(Debug, Clone)]
pub struct ShipParseError;

impl std::error::Error for ShipParseError {}

impl std::fmt::Display for ShipParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown ship name.")
    }
}