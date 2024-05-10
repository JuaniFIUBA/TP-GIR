use crate::{
    tipos_de_dato::{
        comando::Ejecutar,
        comandos::{cat_file::conseguir_tamanio, hash_object::HashObject},
        logger::Logger,
    },
    utils::compresion::descomprimir_objeto,
    utils::path_buf::obtener_nombre,
};
use std::{fmt::Display, path::PathBuf, sync::Arc};

#[derive(Clone, Debug)]
pub struct Blob {
    /// Hash del objeto blob.
    pub hash: String,
    /// Ubicacion del objeto blob.
    pub ubicacion: PathBuf,
    /// Nombre del archivo que representa el blob.
    pub nombre: String,
    /// Logger para imprimir mensajes en el archivo log.
    pub logger: Arc<Logger>,
}

impl PartialEq for Blob {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Blob {}

impl Blob {
    /// Devuelve el hash del blob.
    pub fn obtener_hash(&self) -> String {
        self.hash.clone()
    }

    /// Devuelve el tamanio del blob.
    /// Para obtener el tamanio del blob, se descomprime el objeto y se lee el header.
    pub fn obtener_tamanio(&self) -> Result<usize, String> {
        let contenido_blob = descomprimir_objeto(&self.hash, ".gir/objects/")?;
        let header = contenido_blob
            .split('\0')
            .next()
            .ok_or("No se pudo leer el header")?;
        let tamanio_blob = conseguir_tamanio(header)?;

        match tamanio_blob.parse::<usize>() {
            Ok(tamanio) => Ok(tamanio),
            Err(_) => Err("No se pudo parsear el tamanio del blob".to_string()),
        }
    }

    /// Crea un objeto blob a partir de un archivo.
    pub fn from_directorio(directorio: PathBuf, logger: Arc<Logger>) -> Result<Blob, String> {
        if directorio.is_dir() {
            return Err("No se puede crear un blob a partir de un directorio".to_string());
        }
        let hash = HashObject {
            logger: logger.clone(),
            escribir: false,
            ubicacion_archivo: directorio.clone(),
        }
        .ejecutar()?;

        let nombre = obtener_nombre(&directorio)?;

        Ok(Blob {
            nombre,
            hash,
            ubicacion: directorio,
            logger,
        })
    }
}

impl Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = format!("100644 {} {}\n", self.hash, self.ubicacion.display());
        write!(f, "{}", string)
    }
}
