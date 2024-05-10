use std::path::PathBuf;

/// Tipo de dato para la fase de descubrimiento
/// El String representa el hash del commit
/// El PathBuf representa la direccion de la referencia asociada a ese commit en el servidor.
/// La direccion de la referencia puede pertenecer a una rama o a un tag.
pub type ReferenciaCommit = Vec<(String, PathBuf)>;
