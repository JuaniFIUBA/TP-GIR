/// Elimina el prefijo de las lineas. Por ejemplo, si el prefijo es "remote = ", y la linea es "remote = origin", devuelve "origin".
pub fn eliminar_prefijos(lineas: &Vec<String>) -> Vec<String> {
    let mut lineas_sin_prefijo: Vec<String> = Vec::new();
    for linea in lineas {
        lineas_sin_prefijo.push(linea.split_whitespace().collect::<Vec<&str>>()[1].to_string())
    }
    lineas_sin_prefijo
}

///Obtiene de la url el ip puerto y el repositorio
///
/// ## Ejemplo
/// - recibe: ip:puerto/repositorio/
/// - devuelve: (ip:puerto, /respositorio/)
pub fn obtener_ip_puerto_y_repositorio(url: &str) -> Result<(String, String), String> {
    let (ip_puerto_str, repositorio) = url
        .split_once('/')
        .ok_or_else(|| format!("Fallo en obtener el ip:puerto y repo de {}", url))?;

    Ok((ip_puerto_str.to_string(), "/".to_string() + repositorio))
}

//le calcula en hexa el largo a una linea
pub fn calcular_largo_hex(line: &str) -> String {
    let largo = line.len() + 4; // el + 4 es por los 4 bytes que indican el largo
    let largo_hex = format!("{:x}", largo);
    format!("{:0>4}", largo_hex)
}

///le agrega a la linea al principio el largo en hexa
pub fn obtener_linea_con_largo_hex(line: &str) -> String {
    let largo_hex = calcular_largo_hex(line);
    format!("{}{}", largo_hex, line)
}
