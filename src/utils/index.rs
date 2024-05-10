use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::BufRead,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::tipos_de_dato::{
    comando::Ejecutar, comandos::hash_object::HashObject, logger::Logger, objeto::Objeto,
};

use super::{io, path_buf::obtener_directorio_raiz};

const PATH_INDEX: &str = "./.gir/index";

#[derive(Debug, Clone)]
pub struct ObjetoIndex {
    pub merge: bool,
    pub objeto: Objeto,
    pub es_eliminado: bool,
}

pub fn crear_index() {
    if Path::new(PATH_INDEX).exists() {
        return;
    }
    let _ = fs::File::create(PATH_INDEX);
}

//Devuelve true si el index esta vacio y false en caso contrario.
//Si falla se presupone que es porque no existe y por lo tanto esta vacio
pub fn esta_vacio_el_index() -> Result<bool, String> {
    Ok(io::esta_vacio(PATH_INDEX))
}

/// Lee el archivo index y devuelve un vector de objetos index.
/// Por cada entrie que lee crea su respectivo objeto index.
/// Si el archivo index no existe, devuelve un vector vacio.
pub fn leer_index(logger: Arc<Logger>) -> Result<Vec<ObjetoIndex>, String> {
    if !PathBuf::from(PATH_INDEX).exists() {
        return Ok(Vec::new());
    }
    let file = match OpenOptions::new().read(true).open(PATH_INDEX) {
        Ok(file) => file,
        Err(_) => return Err("No se pudo abrir el archivo index".to_string()),
    };

    let mut objetos: Vec<ObjetoIndex> = Vec::new();

    for line in std::io::BufReader::new(file).lines() {
        if let Ok(line) = line.as_ref() {
            let (metadata, line) = line.split_at(4);
            let (simbolo_eliminado, merge) = metadata.split_at(2);
            let objeto = Objeto::from_index(line, logger.clone())?;
            let objeto_index = ObjetoIndex {
                merge: merge.trim() == "1",
                es_eliminado: simbolo_eliminado.trim() == "-",
                objeto,
            };
            objetos.push(objeto_index);
        }
    }
    Ok(objetos)
}

/// Devuelve un vector de objetos raiz a partir de un vector de objetos index.
/// Si un objeto index esta marcado como eliminado, no se agrega al vector de objetos raiz.
/// Cabe recalcar que estos objetos index quedan ordenados por su path.
pub fn generar_objetos_raiz(
    objetos_index: &Vec<ObjetoIndex>,
    logger: Arc<Logger>,
) -> Result<Vec<Objeto>, String> {
    let mut objetos_raiz: Vec<Objeto> = Vec::new();
    let mut directorios_raiz: HashSet<PathBuf> = HashSet::new();
    let mut directorios_a_tener_en_cuenta: Vec<PathBuf> = Vec::new();

    for objeto_index in objetos_index {
        if objeto_index.es_eliminado {
            continue;
        }

        match objeto_index.objeto {
            Objeto::Blob(ref blob) => {
                directorios_a_tener_en_cuenta.push(blob.ubicacion.clone());
                let padre = obtener_directorio_raiz(&blob.ubicacion)?;
                directorios_raiz.insert(PathBuf::from(padre));
            }
            Objeto::Tree(ref tree) => {
                if tree.es_vacio() {
                    continue;
                }
                directorios_a_tener_en_cuenta.extend(tree.obtener_paths_hijos());
                let padre = obtener_directorio_raiz(&tree.directorio)?;
                directorios_raiz.insert(PathBuf::from(padre));
            }
        }
    }

    for directorio in directorios_raiz {
        let objeto_conteniendo_al_blob = Objeto::from_directorio(
            directorio.clone(),
            Some(&directorios_a_tener_en_cuenta),
            logger.clone(),
        )?;

        objetos_raiz.push(objeto_conteniendo_al_blob);
    }

    objetos_raiz.sort_by_key(|x| match x {
        Objeto::Blob(blob) => blob.ubicacion.clone(),
        Objeto::Tree(tree) => PathBuf::from(&tree.directorio),
    });
    Ok(objetos_raiz)
}

/// Escribe los objetos index en el archivo index.
/// Los escribe siguiendo el siguiente formato:
/// [simbolo eliminado] [merge] [modo] [hash] [path]
/// Si el archivo index no existe, lo crea.
pub fn escribir_index(
    logger: Arc<Logger>,
    objetos_index: &mut Vec<ObjetoIndex>,
) -> Result<(), String> {
    let mut buffer = String::new();

    objetos_index.sort_by_key(|objeto_index| objeto_index.objeto.obtener_path());

    for objeto_index in objetos_index {
        let line = match objeto_index.objeto {
            Objeto::Blob(ref blob) => {
                if !objeto_index.es_eliminado {
                    HashObject {
                        logger: logger.clone(),
                        escribir: true,
                        ubicacion_archivo: blob.ubicacion.clone(),
                    }
                    .ejecutar()?;
                }
                let simbolo_eliminado = if objeto_index.es_eliminado { "-" } else { "+" };
                let merge = if objeto_index.merge { "1" } else { "0" };
                format!("{simbolo_eliminado} {merge} {blob}")
            }
            Objeto::Tree(_) => Err("No se puede escribir un arbol en el index".to_string())?,
        };

        buffer.push_str(&line);
    }

    io::escribir_bytes(PATH_INDEX, buffer)?;
    Ok(())
}

/// Limpia el contenido del archivo index.
pub fn limpiar_archivo_index() -> Result<(), String> {
    let _ = match OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("./.gir/index")
    {
        Ok(archivo) => archivo,
        Err(_) => return Err("No se pudo abrir el archivo index".to_string()),
    };
    Ok(())
}

pub fn hay_archivos_con_conflictos(logger: Arc<Logger>) -> bool {
    let objetos_index = match leer_index(logger.clone()) {
        Ok(objetos_index) => objetos_index,
        Err(_) => return false,
    };
    for objeto_index in objetos_index {
        if objeto_index.merge {
            return true;
        }
    }
    false
}
