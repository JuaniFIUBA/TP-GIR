use super::estado::EstadoHttp;

#[derive(Debug)]
pub enum ErrorHttp {
    NotFound(String),
    InternalServerError(String),
    ValidationFailed(String),
    Forbidden(String),
    BadRequest(String),
    Conflict(String),
    NotImplemented(String),
}

impl ToString for ErrorHttp {
    fn to_string(&self) -> String {
        match self {
            Self::NotFound(mensaje) => format!("404 Not Found: {}", mensaje),
            Self::InternalServerError(mensaje) => format!("500 Internal Server Error: {}", mensaje),
            Self::ValidationFailed(mensaje) => format!("422 Validation Failed: {}", mensaje),
            Self::Forbidden(mensaje) => format!("403 Forbidden: {}", mensaje),
            Self::BadRequest(mensaje) => format!("400 Bad Request: {}", mensaje),
            Self::Conflict(mensaje) => format!("409 Conflict: {}", mensaje),
            Self::NotImplemented(mensaje) => format!("501 Not Implemented: {}", mensaje),
        }
    }
}

impl ErrorHttp {
    pub fn obtener_estado(&self) -> EstadoHttp {
        match self {
            Self::NotFound(_) => EstadoHttp::NotFound,
            Self::InternalServerError(_) => EstadoHttp::InternalServerError,
            Self::ValidationFailed(_) => EstadoHttp::ValidationFailed,
            Self::Forbidden(_) => EstadoHttp::Forbidden,
            Self::BadRequest(_) => EstadoHttp::BadRequest,
            Self::Conflict(_) => EstadoHttp::Conflict,
            Self::NotImplemented(_) => EstadoHttp::MethodNotAllowed,
        }
    }

    pub fn obtener_mensaje(&self) -> String {
        match self {
            Self::NotFound(mensaje) => mensaje.to_string(),
            Self::InternalServerError(mensaje) => mensaje.to_string(),
            Self::ValidationFailed(mensaje) => mensaje.to_string(),
            Self::Forbidden(mensaje) => mensaje.to_string(),
            Self::BadRequest(mensaje) => mensaje.to_string(),
            Self::Conflict(mensaje) => mensaje.to_string(),
            Self::NotImplemented(mensaje) => mensaje.to_string(),
        }
    }
}
