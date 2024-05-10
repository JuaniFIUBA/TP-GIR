use std::fmt::Display;

pub enum EstadoHttp {
    Ok,
    NotFound,
    NoContent,
    MethodNotAllowed,
    InternalServerError,
    BadRequest,
    Created,
    Conflict,
    ValidationFailed,
    Forbidden,
}

impl EstadoHttp {
    pub fn obtener_estado_y_mensaje(&self) -> (usize, String) {
        match self {
            EstadoHttp::Ok => (200, "OK".to_string()),
            EstadoHttp::NoContent => (204, "No Content".to_string()),
            EstadoHttp::MethodNotAllowed => (205, "Merge Not Allowed".to_string()),
            EstadoHttp::Conflict => (409, "Conflict".to_string()),
            EstadoHttp::NotFound => (404, "Not Found".to_string()),
            EstadoHttp::InternalServerError => (500, "Internal Server Error".to_string()),
            EstadoHttp::BadRequest => (400, "Bad Request".to_string()),
            EstadoHttp::Created => (201, "Created".to_string()),
            EstadoHttp::ValidationFailed => (422, "Validacion Failed".to_string()),
            EstadoHttp::Forbidden => (403, "Forbidden".to_string()),
        }
    }
}

impl Display for EstadoHttp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (estado, mensaje) = self.obtener_estado_y_mensaje();
        write!(f, "{} {}", estado, mensaje)
    }
}
