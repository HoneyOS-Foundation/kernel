/// The errors for the display
#[derive(Debug)]
pub enum Error {
    DisplayOccupied,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayOccupied => {
                writeln!(f, "The display is currently in control by another process")
            }
        }
    }
}
