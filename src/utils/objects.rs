use std::fs;
use std::{io, path::PathBuf};

use crate::err_comunicacion::ErrorDeComunicacion;

use super::path_buf;
use super::{io as gir_io, strings};

///Devuelve todos los objetos dentro de objetcs (sus hash)
pub fn obtener_objetos_del_dir(dir: &PathBuf) -> Result<Vec<String>, String> {
    let dir_abierto = gir_io::leer_directorio(dir)?;

    let mut objetos: Vec<String> = Vec::new();

    for entrada in dir_abierto {
        match entrada {
            Ok(entrada) => {
                if gir_io::es_dir(entrada.path())
                    && entrada.file_name().to_string_lossy() != "info"
                    && entrada.file_name().to_string_lossy() != "pack"
                    && !entrada.path().to_string_lossy().contains("log.txt")
                {
                    objetos.append(&mut obtener_objetos_con_nombre_carpeta(entrada.path())?);
                }
            }
            Err(error) => {
                return Err(format!("Error leyendo directorio: {}", error));
            }
        }
    }
    Ok(objetos)
}

///Obitiene todos los objetos asosiados a una carpeta dentro objetcs. Dado una carpeta, devuelve
/// todo los objtetos asosiados a este
///
/// ## Ejemplo
/// - recive: jk/
/// - devuleve: jksfsfsffafasfas...fdfdf, kjsfsfaftyhththht, jkiodf235453535355fs, ...
///
/// ## Error
/// -Si no existe dir
/// -Si no tiene conti8dio
fn obtener_objetos_con_nombre_carpeta(dir: PathBuf) -> Result<Vec<String>, String> {
    let directorio = gir_io::leer_directorio(&dir)?;

    let mut objetos = Vec::new();
    let nombre_directorio = path_buf::obtener_nombre(&dir)?;

    for archivo in directorio {
        match archivo {
            Ok(archivo) => {
                objetos.push(
                    nombre_directorio.clone() + archivo.file_name().to_string_lossy().as_ref(),
                );
            }
            Err(error) => {
                return Err(format!("Error leyendo directorio: {}", error));
            }
        }
    }

    if objetos.is_empty() {
        return Err(format!(
            "Error el directorio {} no tiene cotenido",
            nombre_directorio
        ));
    }

    Ok(objetos)
}

// dado un vector con nombres de archivos de vuelve aquellos que no estan en el directorio
pub fn obtener_archivos_faltantes(nombres_archivos: Vec<String>, dir: &str) -> Vec<String> {
    // DESHARDCODEAR EL NOMBRE DEL DIRECTORIO (.gir)
    let objetcts_contained =
        obtener_objetos_del_dir(&PathBuf::from(dir.to_string() + "objects/")).unwrap();
    let mut archivos_faltantes: Vec<String> = Vec::new();
    for nombre in &objetcts_contained {
        if nombres_archivos.contains(nombre) {
        } else {
            archivos_faltantes.push(nombre.clone());
        }
    }
    archivos_faltantes
}

// dado un directorio devuelve el nombre del archivo contenido (solo caso de objectos de git)
pub fn obtener_objetos(dir: PathBuf) -> Result<String, ErrorDeComunicacion> {
    let mut directorio = fs::read_dir(dir.clone())?;
    if let Some(archivo) = directorio.next() {
        match archivo {
            Ok(archivo) => {
                return Ok(archivo.file_name().to_string_lossy().to_string());
            }
            Err(error) => {
                eprintln!("Error leyendo directorio: {}", error);
            }
        }
    }
    Err(ErrorDeComunicacion::IoError(io::Error::new(
        io::ErrorKind::NotFound,
        "Hubo un error al obtener el objeto",
    )))
}

// aca depende de si esta multi_ack y esas cosas, esta es para cuando no hay multi_ack ni multi_ack_mode
pub fn obtener_objetos_en_comun(nombres_archivos: Vec<String>, dir: &str) -> Vec<String> {
    let mut ack = Vec::new();
    for nombre in nombres_archivos {
        let dir_archivo = format!("{}{}/{}", dir, &nombre[..2], &nombre[2..]);
        if PathBuf::from(dir_archivo.clone()).exists() {
            ack.push(strings::obtener_linea_con_largo_hex(
                ("ACK ".to_string() + &nombre + "\n").as_str(),
            ));
            break;
        }
    }
    if ack.is_empty() {
        ack.push(strings::obtener_linea_con_largo_hex("NAK\n"));
    }
    ack
}
