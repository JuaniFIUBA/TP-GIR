/// Representa los posibles flags que puede tener el comando cat-file.
pub enum Visualizaciones {
    /// Muestra el tipo de objeto.
    TipoObjeto,
    /// Muestra el tamanio del objeto.
    Tamanio,
    /// Muestra el contenido del objeto.
    Contenido,
}

impl Visualizaciones {
    /// Crea un cierto tipo de visualizacion a partir de un parametro.
    pub fn from(parametro: &str) -> Result<Visualizaciones, String> {
        match parametro {
            "-t" => Ok(Visualizaciones::TipoObjeto),
            "-s" => Ok(Visualizaciones::Tamanio),
            "-p" => Ok(Visualizaciones::Contenido),
            _ => Err(format!(
                "Parametro desconocido {}, parametros esperados: (-t | -s | -p)",
                parametro
            )),
        }
    }
}
