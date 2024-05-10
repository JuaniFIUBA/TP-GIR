use std::path::PathBuf;

#[derive(Debug, Clone)]

/// Informacion de una rama.
pub struct RamasInfo {
    /// Nombre de la rama.
    pub nombre: String,
    /// Nombre del remote.
    pub remote: String,
    /// Path a donde se debe realizar el merge a la hora de pullear.
    pub merge: PathBuf,
}
