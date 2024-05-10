/// Respuesta de pedido
pub enum RespuestaDePedido {
    /// Un mensaje recibido de un cliente.
    Mensaje(String),
    /// Un mensaje para terminar la conexion con un cliente.
    Terminate,
}
