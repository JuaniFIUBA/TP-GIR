#[derive(Debug, Clone)]
/// Representa el tipo de cambio que se le hizo a un archivo.
pub enum TipoDiff {
    /// La linea fue agregada.
    Added(String),
    /// La linea fue removida.
    Removed(String),
    /// La linea no tiene cambios.
    Unchanged(String),
}
