use std::{error::Error, fmt, io, str::Utf8Error};

#[derive(Debug)]
pub enum ErrorDeComunicacion {
    Utf8Error(Utf8Error),
    IoError(io::Error),
    ErrorRepositorioNoExiste(String),
}

impl fmt::Display for ErrorDeComunicacion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorDeComunicacion::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
            ErrorDeComunicacion::IoError(e) => write!(f, "IO error: {}", e),
            ErrorDeComunicacion::ErrorRepositorioNoExiste(e) => {
                writeln!(f, "ERR El repositorio {} no existe", e)
            }
        }
    }
}

impl Error for ErrorDeComunicacion {}

impl From<Utf8Error> for ErrorDeComunicacion {
    fn from(error: Utf8Error) -> Self {
        ErrorDeComunicacion::Utf8Error(error)
    }
}

impl From<io::Error> for ErrorDeComunicacion {
    fn from(error: io::Error) -> Self {
        ErrorDeComunicacion::IoError(error)
    }
}
