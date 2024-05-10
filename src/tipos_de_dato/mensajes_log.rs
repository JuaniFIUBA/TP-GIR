#[derive(Debug, Default)]
/// Representa un mensaje de log.
pub enum Log {
    /// Un mensaje de log.
    Message(String),
    /// El fin del logger.
    #[default]
    End,
}
