/// Variante de comando remote
pub enum ComandoRemote {
    /// Muestra todos los remotes
    Mostrar,
    /// Agrega un remote
    Agregar,
    /// Elimina un remote
    Eliminar,
    /// Cambia la url de un remote
    CambiarUrl,
    /// Muestra la url de un remote
    MostrarUrl,
}
