use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{io, strings};

pub fn obtener_refs_con_largo_hex(
    refs: &mut Vec<String>,
    refs_path: PathBuf,
    dir: &str,
) -> Result<(), String> {
    if !refs_path.exists() {
        return Ok(());
    }
    let head_dir = fs::read_dir(&refs_path).map_err(|e| e.to_string())?;
    for archivo in head_dir {
        match archivo {
            Ok(archivo) => {
                let mut path = archivo.path();

                let referencia = obtener_referencia(&mut path, dir)?;
                refs.push(strings::obtener_linea_con_largo_hex(&referencia));
            }
            Err(error) => {
                eprintln!("Error leyendo directorio: {}", error);
            }
        }
    }
    Ok(())
}

pub fn obtener_ref_head(path: PathBuf) -> Result<String, String> {
    if !path.exists() {
        return Err("No existe HEAD".to_string());
    }
    let contenido = io::leer_archivo(&mut path.clone())?;
    let head_ref = contenido.split_whitespace().collect::<Vec<&str>>()[1];
    if let Some(ruta) = path.clone().parent() {
        let cont = io::leer_archivo(&mut ruta.join(head_ref))? + " HEAD";
        Ok(strings::obtener_linea_con_largo_hex(&cont))
    } else {
        Err("Error al leer HEAD, verifique la ruta".to_string())
    }
}

pub fn obtener_refs(refs_path: PathBuf, dir: &str) -> Result<Vec<String>, String> {
    let mut refs: Vec<String> = Vec::new();
    if !refs_path.exists() {
        return Ok(refs);
        // io::Error::new(io::ErrorKind::NotFound, "No existe el repositorio");
    }

    if refs_path.ends_with("HEAD") {
        refs.push(obtener_ref_head(refs_path.to_path_buf())?);
    } else {
        let head_dir = fs::read_dir(&refs_path).map_err(|e| e.to_string())?;
        for archivo in head_dir {
            match archivo {
                Ok(archivo) => {
                    let mut path = archivo.path();
                    // let mut path = archivo.path().to_string_lossy().split("./.gir/").into_iter().next().unwrap().to_string();
                    refs.push(obtener_referencia(&mut path, dir)?);
                }
                Err(error) => {
                    eprintln!("Error leyendo directorio: {}", error);
                }
            }
        }
    }
    Ok(refs)
}

fn obtener_referencia(path: &mut Path, prefijo: &str) -> Result<String, String> {
    let mut contenido = io::leer_archivo(path)?;
    if contenido.is_empty() {
        contenido = "0".repeat(40);
    }
    let directorio_sin_prefijo = path.strip_prefix(prefijo).unwrap().to_path_buf();
    let referencia = format!(
        "{} {}",
        contenido.trim(),
        directorio_sin_prefijo.to_str().ok_or("No existe HEAD")?
    );
    Ok(referencia)
}
