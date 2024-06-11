/// The errors for the display
#[derive(Debug)]
pub enum Error {
    DisplayOccupied,
    CannotLoosen,
    AlreadyLoose,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayOccupied => {
                writeln!(f, "The display is currently in control by another process")
            }
            Self::CannotLoosen => {
                writeln!(f, "Cannot loosen control over display as there is not process in control in the first place.")
            }
            Self::AlreadyLoose => {
                writeln!(
                    f,
                    "Cannot loosen control over display as the control is already loose"
                )
            }
        }
    }
}
