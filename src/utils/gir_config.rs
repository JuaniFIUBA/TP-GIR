use std::path::PathBuf;

use super::io;

/// Devuelve el path del archivo de configuración de gir.
pub fn obtener_gir_config_path() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "Error al obtener el directorio home")?;
    let config_path = format!("{home}/.girconfig");
    Ok(config_path)
}

/// Devuelve el nombre y el mail del usuario guardados en el archivo de configuración.
pub fn conseguir_nombre_y_mail_del_config() -> Result<(String, String), String> {
    let nombre =
        conseguir_nombre_config().ok_or("Error al extraer el nombre del config".to_string())?;
    let mail = conseguir_mail_config().ok_or("Error al extraer el mail del config".to_string())?;

    Ok((nombre, mail))
}

///extrae el nombre seteada en el archivo config.
///Busca una entrada que sea nombre:
pub fn conseguir_nombre_config() -> Option<String> {
    buscar_en_config_el_valor_de("nombre")
}

///extrae el mail seteada en el archivo config.
///Busca una entrada que sea mail=
pub fn conseguir_mail_config() -> Option<String> {
    buscar_en_config_el_valor_de("mail")
}

//extrae la ubiacion del archivo log seteada en el archivo config. En caso de error
// devuelve una direccion default = .log
pub fn conseguir_ubicacion_log_config() -> Result<PathBuf, String> {
    let mut dir_archivo_log = PathBuf::from(".log");
    let ubicacion_config = obtener_gir_config_path()?;
    let contenido_config = match io::leer_a_string(ubicacion_config) {
        Ok(contenido_config) => contenido_config,
        Err(_) => return Ok(dir_archivo_log),
    };

    for linea_config in contenido_config.lines() {
        if linea_config.trim().starts_with("log") {
            if let Some(dir_archivo_log_config) = linea_config.split('=').nth(1) {
                dir_archivo_log = PathBuf::from(dir_archivo_log_config.trim());
                break;
            }
        }
    }

    Ok(dir_archivo_log)
}
///extrae el server_url seteada en el archivo config.
///Busca una entrada que sea 'server_url='
pub fn conseguir_url_servidor() -> Option<String> {
    buscar_en_config_el_valor_de("server_url")
}

///extrae el remoto seteada en el archivo config.
///Busca una entrada que sea 'remoto='
pub fn conseguir_direccion_nombre_remoto() -> Option<String> {
    buscar_en_config_el_valor_de("remoto")
}

pub fn conseguir_puerto_http() -> Option<String> {
    buscar_en_config_el_valor_de("puerto_http")
}

///extrae el repositorio seteada en el archivo config.
///Busca una entrada que sea 'repositorio='
pub fn conseguir_direccion_nombre_repositorio() -> Option<String> {
    buscar_en_config_el_valor_de("repositorio")
}

fn buscar_en_config_el_valor_de(parametro_a_buscar: &str) -> Option<String> {
    let config_path = obtener_gir_config_path().ok()?;
    let contenido_config = io::leer_a_string(config_path).ok()?;

    for linea_config in contenido_config.lines() {
        if linea_config.trim().starts_with(parametro_a_buscar) {
            if let Some(repositorio) = linea_config.split('=').nth(1) {
                return Some(repositorio.trim().to_string());
            }
        }
    }

    None
}

/// Devuelve si el archivo config esta vacio.
fn archivo_config_esta_vacio() -> Result<bool, String> {
    let config_path = obtener_gir_config_path()?;

    let contenido = match io::leer_a_string(config_path) {
        Ok(contenido) => contenido,
        Err(_) => return Ok(true),
    };
    if contenido.is_empty() {
        return Ok(true);
    }
    Ok(false)
}

/// Arma el archivo de configuración con el nombre y el mail del usuario.
/// Si el archivo ya tiene información, no hace nada.
/// Si el archivo no tiene información, pide al usuario que ingrese su nombre y su mail.
/// Si no se puede leer el nombre o el mail, devuelve un error.
pub fn armar_config_con_mail_y_nombre() -> Result<(), String> {
    if !archivo_config_esta_vacio()? {
        return Ok(());
    }
    let mut nombre = String::new();
    let mut mail = String::new();

    println!("Por favor, ingrese su nombre:");
    match std::io::stdin().read_line(&mut nombre) {
        Ok(_) => (),
        Err(_) => return Err("No se pudo leer el nombre ingresado".to_string()),
    };

    println!("Por favor, ingrese su correo electrónico:");
    match std::io::stdin().read_line(&mut mail) {
        Ok(_) => (),
        Err(_) => return Err("No se pudo leer el mail ingresado".to_string()),
    };

    nombre = nombre.trim().to_string();
    mail = mail.trim().to_string();

    let config_path = obtener_gir_config_path()?;
    let contenido = format!("nombre ={}\nmail ={}\n", nombre, mail);
    io::escribir_bytes(config_path, contenido)?;
    println!("Información de usuario guardada en ~/.girconfig.");
    Ok(())
}
