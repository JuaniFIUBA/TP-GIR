/// Tipo de rama que se puede cambiar
pub enum TipoRama {
    Local,
    /// El primer string representa la ruta del remote
    /// El segundo string representa el hash del commit al que apunta
    Remota(String, String),
}
