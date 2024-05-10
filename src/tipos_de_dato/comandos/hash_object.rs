use crate::tipos_de_dato::comando::Ejecutar;
use crate::tipos_de_dato::logger::Logger;
use crate::utils::compresion::comprimir_contenido_u8;
use crate::utils::io;
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use std::sync::Arc;

pub struct HashObject {
    /// Logger para imprimir mensajes en el archivo log.
    pub logger: Arc<Logger>,
    /// Si esta activado, escribe el objeto en el repositorio.
    pub escribir: bool,
    /// Ubicacion del archivo a hashear.
    pub ubicacion_archivo: PathBuf,
}

impl HashObject {
    /// Obtiene el nombre del archivo a hashear. Si no se especifica un archivo, devuelve un error.
    /// Se espera que el archivo este al final de los parametros pasados.
    fn obtener_nombre_archivo(args: &mut Vec<String>) -> Result<PathBuf, String> {
        let nombre_string = args
            .pop()
            .ok_or_else(|| "No se especifico un archivo".to_string());
        Ok(PathBuf::from(nombre_string?))
    }

    /// Crear un HashObject a partir de los argumentos pasados por linea de comandos.
    /// En caso de tener argumentos invalidos devuelve error.
    pub fn from(args: &mut Vec<String>, logger: Arc<Logger>) -> Result<HashObject, String> {
        let mut escribir = false;
        let nombre_archivo = Self::obtener_nombre_archivo(args)?;

        let iterador = args.iter();
        for arg in iterador {
            match arg.as_str() {
                "-w" => {
                    escribir = true;
                }
                _ => {
                    return Err(format!(
                        "Opcion desconocida {}\n gir hash-object [-w] <file>",
                        arg
                    ));
                }
            }
        }
        Ok(HashObject {
            logger,
            ubicacion_archivo: nombre_archivo,
            escribir,
        })
    }

    /// Construye el contenido del objeto blob a partir del archivo pasado por parametro.
    /// El contenido del objeto blob es el contenido del archivo con un header que indica
    /// el tipo de objeto y su tamaño.
    fn construir_contenido(&self) -> Result<Vec<u8>, String> {
        let contenido = io::leer_bytes(self.ubicacion_archivo.clone())?;
        let header = format!("blob {}\0", contenido.len());
        let contenido_total = [header.as_bytes(), &contenido].concat();

        Ok(contenido_total)
    }

    /// Hashea el contenido del objeto.
    /// Devuelve un hash de 40 caracteres en formato hexadecimal.
    pub fn hashear_contenido_objeto(contenido: &Vec<u8>) -> String {
        let mut hasher = Sha1::new();
        hasher.update(contenido);
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }
}

impl Ejecutar for HashObject {
    /// Ejecuta el comando hash-object.
    /// Devuelve el hash del objeto creado.
    /// Si la opcion -w esta activada, escribe el objeto en el repositorio.
    fn ejecutar(&mut self) -> Result<String, String> {
        let contenido = self.construir_contenido()?;
        let hash = Self::hashear_contenido_objeto(&contenido);

        if self.escribir {
            let ruta = format!(".gir/objects/{}/{}", &hash[..2], &hash[2..]);
            io::escribir_bytes(ruta, comprimir_contenido_u8(&contenido)?)?;
        }
        let mensaje = format!(
            "Objeto gir hasheado en {}",
            self.ubicacion_archivo.to_string_lossy()
        );
        self.logger.log(&mensaje);
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{io::Read, path::PathBuf, sync::Arc};

    use flate2::read::ZlibDecoder;

    use crate::{
        tipos_de_dato::{comando::Ejecutar, comandos::hash_object::HashObject, logger::Logger},
        utils::io,
    };

    #[test]
    #[serial]
    fn test01_hash_object_de_un_blob_devuelve_el_hash_correcto() {
        let mut args = vec!["test_dir/objetos/archivo.txt".to_string()];
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/hash_object_test01")).unwrap());
        let mut hash_object = HashObject::from(&mut args, logger).unwrap();
        let hash = hash_object.ejecutar().unwrap();
        assert_eq!(hash, "2b824e648965b94c6c6b3dd0702feb91f699ed62");
    }

    #[test]
    #[serial]
    fn test02_hash_object_de_un_blob_con_opcion_w_devuelve_el_hash_correcto_y_lo_escribe() {
        let mut args = vec!["-w".to_string(), "test_dir/objetos/archivo.txt".to_string()];
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/hash_object_test01")).unwrap());
        let mut hash_object = HashObject::from(&mut args, logger).unwrap();
        let hash = hash_object.ejecutar().unwrap();
        assert_eq!(hash, "2b824e648965b94c6c6b3dd0702feb91f699ed62");
        let contenido_leido =
            io::leer_bytes(".gir/objects/2b/824e648965b94c6c6b3dd0702feb91f699ed62").unwrap();
        let mut descompresor = ZlibDecoder::new(contenido_leido.as_slice());
        let mut contenido_descomprimido = String::new();
        descompresor
            .read_to_string(&mut contenido_descomprimido)
            .unwrap();
        assert_eq!(contenido_descomprimido, "blob 23\0contenido de un arxhivo");
    }
}
