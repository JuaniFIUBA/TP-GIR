use std::fmt::Debug;
use std::fs::{self, File, ReadDir};
use std::io::BufRead;
use std::path::Path;
use std::{env, str};

pub(crate) fn leer_archivo(path: &mut Path) -> Result<String, String> {
    let archivo = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut contenido = String::new();
    std::io::BufReader::new(archivo)
        .read_line(&mut contenido)
        .map_err(|e| e.to_string())?;
    Ok(contenido.trim().to_string())
}

//Devuelve true si la ubicacion esta vacia y false en caso contrario.
//Si falla se presupone que es porque no existe y por lo tanto esta vacio
pub fn esta_vacio(ubicacion: &str) -> bool {
    match fs::metadata(ubicacion) {
        Ok(metadata) => metadata.len() == 0,
        Err(_) => false,
    }
}

///Lee un directorio. Devuelve su iterador. Falla si no existe o si no es un directoro
pub fn leer_directorio<P>(directorio: &P) -> Result<ReadDir, String>
where
    P: AsRef<Path> + Debug + ?Sized,
{
    let metadada_dir =
        fs::metadata(directorio).map_err(|_| format!("Error no existe el dir {:?}", directorio))?;

    if !metadada_dir.is_dir() {
        return Err(format!("Error {:?} no es un dir", directorio));
    }

    fs::read_dir(directorio).map_err(|e| format!("Error al leer {:?}: {}", directorio, e))
}

pub fn cantidad_entradas_dir<P>(directorio: &P) -> Result<u64, String>
where
    P: AsRef<Path> + Debug + ?Sized,
{
    leer_directorio(directorio)?
        .count()
        .try_into()
        .map_err(|e| format!("Error al tratar de pasar de usize a u64: {e}"))
}

///Devuelve True si el directororio es un directorio o false en caso contrario o si no existe
pub fn es_dir<P: AsRef<Path> + Clone + Debug>(entrada: P) -> bool {
    match fs::metadata(entrada) {
        Ok(metadata_contenido) => metadata_contenido.is_dir(),
        Err(_) => false,
    }
}

///Crea un directorio
pub fn crear_directorio<P>(directorio: P) -> Result<(), String>
where
    P: AsRef<Path>,
{
    let dir = fs::metadata(&directorio);
    if dir.is_ok() {
        return Ok(());
    }
    match fs::create_dir_all(directorio) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error al crear el directorio: {}", e)),
    }
}
///Similar a `crear_directorio` pero puede fallar si la carpeta ya existe
pub fn crear_carpeta<P: AsRef<Path> + Clone>(carpeta: P) -> Result<(), String> {
    match fs::create_dir_all(carpeta) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error al crear la carpeta: {}", e)),
    }
}

pub fn cambiar_directorio<P: AsRef<Path> + Clone + Debug>(directorio: P) -> Result<(), String> {
    env::set_current_dir(&directorio)
        .map_err(|err| format!("Fallo al cambiar de directorio {:?}:{}", directorio, err))
}
pub fn crear_archivo<P: AsRef<Path> + Clone>(dir_directorio: P) -> Result<(), String> {
    si_no_existe_directorio_de_archivo_crearlo(&dir_directorio)?;
    if !dir_directorio.as_ref().exists() {
        File::create(dir_directorio.clone()).map_err(|err| format!("{}", err))?;
    }

    Ok(())
}

pub fn leer_a_string<P>(path: P) -> Result<String, String>
where
    P: AsRef<Path>,
{
    match fs::read_to_string(&path) {
        Ok(contenido) => Ok(contenido),
        Err(_) => Err(format!(
            "No se pudo leer el archivo {}",
            path.as_ref().display()
        )),
    }
}

pub fn escribir_bytes<P, C>(dir_archivo: P, contenido: C) -> Result<(), String>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    si_no_existe_directorio_de_archivo_crearlo(&dir_archivo)?;
    match fs::write(dir_archivo, contenido) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error al escribir el archivo: {}", e)),
    }
}

pub fn leer_bytes<P>(archivo: P) -> Result<Vec<u8>, String>
where
    P: AsRef<Path>,
{
    match fs::read(&archivo) {
        Ok(contenido) => Ok(contenido),
        Err(_) => Err(format!(
            "No se pudo leer el archivo leyendo bytes {}",
            archivo.as_ref().display()
        )),
    }
}

pub fn si_no_existe_directorio_de_archivo_crearlo<P>(dir_archivo: &P) -> Result<(), String>
where
    P: AsRef<Path>,
{
    let dir = dir_archivo.as_ref().parent();
    if let Some(parent_dir) = dir {
        let parent_str = parent_dir
            .to_str()
            .ok_or_else(|| String::from("Error al convertir el directorio a cadena"))?;

        crear_directorio(parent_str.to_owned() + "/")?;
    };
    Ok(())
}

pub fn rm_directorio<P>(directorio: P) -> Result<(), String>
where
    P: AsRef<Path>,
{
    let metadata = fs::metadata(&directorio).map_err(|e| {
        format!(
            "No se pudo obtener la metadata del directorio {}. {}",
            directorio.as_ref().display(),
            e
        )
    })?;

    if metadata.is_file() {
        return match fs::remove_file(&directorio) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!(
                "No se pudo borrar el archivo {}. {}",
                directorio.as_ref().display(),
                e
            )),
        };
    }

    if metadata.is_dir() {
        return match fs::remove_dir_all(&directorio) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!(
                "No se pudo borrar la carpeta {}. {}",
                directorio.as_ref().display(),
                e
            )),
        };
    }
    Err(format!(
        "No se pudo borrar el directorio {}",
        directorio.as_ref().display()
    ))
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::path::PathBuf;

    use crate::utils::io::{escribir_bytes, leer_a_string, rm_directorio};

    #[test]
    #[serial]
    fn test_escribir_archivo_pisa_contenido() {
        let dir = PathBuf::from("tmp/test_escribir_archivo_pisa_contenido.txt");
        escribir_bytes(&dir, "contenido 1").unwrap();
        escribir_bytes(&dir, "contenido 2").unwrap();
        assert_eq!(leer_a_string(&dir).unwrap(), "contenido 2");
        rm_directorio(dir).unwrap();
    }
}
