use std::path::{Path, PathBuf};

/// Dado un directorio y un conjunto de directorios habilitados
/// devuelve si el directorio esta habilitado, es decir, si es un subdirectorio de alguno de los directorios habilitados.
pub fn esta_directorio_habilitado(
    directorio: &Path,
    directorios_habilitados: &Vec<PathBuf>,
) -> bool {
    for directorio_habilitado in directorios_habilitados {
        if directorio.starts_with(directorio_habilitado)
            || directorio_habilitado.starts_with(directorio)
        {
            return true;
        }
    }
    false
}

/// Dado el path de un directorio, devuelve el path del directorio raiz.
/// O sea, si el directorio es /gir/objects/obj, devuelve /gir.
pub fn obtener_directorio_raiz(directorio: &Path) -> Result<String, String> {
    let directorio_split = directorio
        .iter()
        .next()
        .ok_or("Error al obtener el directorio raiz")?
        .to_str()
        .ok_or("Error al obtener el directorio raiz")?;

    Ok(directorio_split.to_string())
}

/// Dado el path de un directorio, devuelve el nombre del directorio.
/// O sea, si el directorio es /gir/objects/obj, devuelve obj.
pub fn obtener_nombre(directorio: &Path) -> Result<String, String> {
    let directorio_split = directorio
        .file_name()
        .ok_or("Error al obtener el nombre")?
        .to_str()
        .ok_or("Error al obtener el nombre")?;

    Ok(directorio_split.to_string())
}
