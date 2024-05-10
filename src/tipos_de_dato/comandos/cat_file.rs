use crate::{
    tipos_de_dato::{
        comando::Ejecutar, logger::Logger, objeto::flag_es_un_objeto_, objetos::tree::Tree,
        visualizaciones::Visualizaciones,
    },
    utils::compresion::descomprimir_objeto,
};
use std::sync::Arc;

pub struct CatFile {
    /// Logger para imprimir los mensajes en el archivo log.
    pub logger: Arc<Logger>,
    /// Opcion de visualizacion del objeto.
    pub visualizacion: Visualizaciones,
    /// Hash del objeto a visualizar.
    pub hash_objeto: String,
}

/// Obtiene el tipo de objeto ubicado en cierto directorio a partir de su hash.
/// En caso de no encontrar el objeto devuelve error.
/// hash_objeto - Hash del objeto a obtener.
/// dir - Directorio donde se encuentra el objeto.
pub fn obtener_tipo_objeto_de(hash_objeto: &str, dir: &str) -> Result<String, String> {
    let (header, _) = obtener_contenido_objeto_de(hash_objeto, dir)?;
    let tipo_objeto = conseguir_tipo_objeto(&header)?;
    Ok(tipo_objeto)
}

/// Obtiene el contenido de un objeto ubicado en cierto directorio a partir de su hash.
/// En caso de no encontrar el objeto devuelve error.
fn obtener_contenido_objeto_de(hash: &str, dir: &str) -> Result<(String, String), String> {
    // sacar el hardcode de esto
    let objeto = descomprimir_objeto(hash, dir)?;
    match objeto.split_once('\0') {
        Some((header, contenido)) => Ok((header.to_string(), contenido.to_string())),
        None => Err("Objeto invalido".to_string()),
    }
}

/// Obtiene el contenido de un objeto ubicado en el directorio de objetos del .gir a partir de su hash.
/// En caso de no encontrar el objeto devuelve error.
pub fn obtener_contenido_objeto(hash: &str) -> Result<(String, String), String> {
    // sacar el hardcode de esto
    let objeto = descomprimir_objeto(hash, ".gir/objects/")?;
    match objeto.split_once('\0') {
        Some((header, contenido)) => Ok((header.to_string(), contenido.to_string())),
        None => Err("Objeto invalido".to_string()),
    }
}

/// Obtiene el tipo de objeto a partir de su header.
/// El header tiene el siguiente formato: <tipo_objeto> <tamanio_objeto>
/// En el caso de tener un formato invalido devuelve error.
pub fn conseguir_tipo_objeto(header: &str) -> Result<String, String> {
    let tipo_objeto = match header.split_once(' ') {
        Some((tipo, _)) => tipo,
        None => return Err("Objeto invalido".to_string()),
    };
    Ok(tipo_objeto.to_string())
}

/// Obtiene el contenido de un objeto en formato pretty print.
/// El contenido del objeto se muestra en formato pretty print dependiendo de su tipo.
/// En caso de ser un blob o un commit devuelve el contenido sin modificar.
/// En caso de ser un tree devuelve el contenido formatteado a pretty print.
/// En caso de no ser un objeto valido devuelve error.
pub fn conseguir_contenido_pretty(header: &str, contenido: &str) -> Result<String, String> {
    let tipo = conseguir_tipo_objeto(header)?;
    match tipo.as_str() {
        "blob" | "commit" => Ok(contenido.to_string()),
        "tree" => {
            let mut pretty_print = String::new();
            let contenido_parseado = Tree::rearmar_contenido_descomprimido(contenido)?;
            let lineas = contenido_parseado.split('\n').collect::<Vec<&str>>();
            for linea in lineas {
                let atributos_objeto = linea.split(' ').collect::<Vec<&str>>();
                let mut modo = atributos_objeto[0].to_string();
                let tipo = match modo.as_str() {
                    "100644" => "blob".to_string(),
                    "40000" => "tree".to_string(),
                    _ => return Err("Objeto invalido".to_string()),
                };
                if modo == "40000" {
                    modo = "040000".to_string();
                }
                let nombre = atributos_objeto[1].to_string();
                let hash = atributos_objeto[2].to_string();
                let linea = format!("{} {} {}   {}\n", modo, tipo, hash, nombre);
                pretty_print.push_str(&linea);
            }
            Ok(pretty_print)
        }
        _ => Err("Objeto invalido".to_string()),
    }
}

/// Obtiene el tamanio de un objeto a partir de su header.
/// El header tiene el siguiente formato: <tipo_objeto> <tamanio_objeto>
/// En el caso de tener un formato invalido devuelve error.
pub fn conseguir_tamanio(header: &str) -> Result<String, String> {
    let size = match header.split_once(' ') {
        Some((_, size)) => size,
        None => return Err("Objeto invalido".to_string()),
    };
    Ok(size.to_string())
}

impl CatFile {
    /// Crea un CatFile a partir de los argumentos pasados por linea de comandos.
    /// En caso de que no se asigne una opcion de visualizacion, se asume que se quiere visualizar el contenido del objeto.
    /// En caso de tener argumentos invalidos devuelve error.
    /// args - Argumentos pasados por linea de comandos.
    /// logger - Logger para imprimir los mensajes de error.
    pub fn from(args: &mut Vec<String>, logger: Arc<Logger>) -> Result<CatFile, String> {
        let objeto = args
            .pop()
            .ok_or_else(|| "No se especifico un objeto".to_string())?;
        let segundo_argumento = args.pop().ok_or_else(|| {
            "No se especifico una opcion de visualizacion (-t | -s | -p)".to_string()
        })?;
        let visualizacion = match flag_es_un_objeto_(&segundo_argumento) {
            true => Visualizaciones::from("-p")?,
            false => Visualizaciones::from(&segundo_argumento)?,
        };
        Ok(CatFile {
            logger,
            visualizacion,
            hash_objeto: objeto,
        })
    }

    pub fn ejecutar_de(&self, dir: &str) -> Result<String, String> {
        let (header, contenido) = obtener_contenido_objeto_de(&self.hash_objeto, dir)?;
        let mensaje = match self.visualizacion {
            Visualizaciones::TipoObjeto => conseguir_tipo_objeto(&header)?,
            Visualizaciones::Tamanio => conseguir_tamanio(&header)?,
            Visualizaciones::Contenido => conseguir_contenido_pretty(&header, &contenido)?,
        };
        self.logger.log(&mensaje);
        Ok(mensaje)
    }
}

impl Ejecutar for CatFile {
    /// Ejecuta el comando cat-file.
    /// En caso de no encontrar el objeto devuelve error.
    /// En caso de no poder parsear el contenido del objeto devuelve error.
    fn ejecutar(&mut self) -> Result<String, String> {
        let (header, contenido) = obtener_contenido_objeto(&self.hash_objeto)?;
        let mensaje = match self.visualizacion {
            Visualizaciones::TipoObjeto => conseguir_tipo_objeto(&header)?,
            Visualizaciones::Tamanio => conseguir_tamanio(&header)?,
            Visualizaciones::Contenido => conseguir_contenido_pretty(&header, &contenido)?,
        };
        self.logger.log(&mensaje);
        Ok(mensaje)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::{
                cat_file::{
                    conseguir_contenido_pretty, conseguir_tamanio, conseguir_tipo_objeto, CatFile,
                },
                hash_object::HashObject,
            },
            logger::Logger,
            visualizaciones::Visualizaciones,
        },
        utils::io,
    };
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    #[test]
    #[serial]
    fn test01_cat_file_blob_para_visualizar_muestra_el_contenido_correcto() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/cat_file_test01")).unwrap());
        let mut hash_object = HashObject::from(
            &mut vec!["-w".to_string(), "test_dir/objetos/archivo.txt".to_string()],
            logger.clone(),
        )
        .unwrap();
        let hash = hash_object.ejecutar().unwrap();
        let mut cat_file = CatFile {
            logger,
            visualizacion: Visualizaciones::Contenido,
            hash_objeto: hash.to_string(),
        };

        let contenido = cat_file.ejecutar().unwrap();
        let contenido_esperado = io::leer_a_string("test_dir/objetos/archivo.txt")
            .unwrap()
            .trim()
            .to_string();
        assert_eq!(contenido, contenido_esperado);
    }

    #[test]
    #[serial]
    fn test02_cat_file_blob_muestra_el_tamanio_correcto() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/cat_file_test02")).unwrap());
        let mut hash_object = HashObject::from(
            &mut vec!["-w".to_string(), "test_dir/objetos/archivo.txt".to_string()],
            logger.clone(),
        )
        .unwrap();
        let hash = hash_object.ejecutar().unwrap();
        let mut cat_file = CatFile {
            logger,
            visualizacion: Visualizaciones::Tamanio,
            hash_objeto: hash.to_string(),
        };
        let tamanio = cat_file.ejecutar().unwrap();
        let tamanio_esperado = io::leer_a_string("test_dir/objetos/archivo.txt")
            .unwrap()
            .trim()
            .len()
            .to_string();
        assert_eq!(tamanio, tamanio_esperado);
    }

    #[test]
    #[serial]
    fn test03_cat_file_blob_muestra_el_tipo_de_objeto_correcto() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/cat_file_test03")).unwrap());
        let mut hash_object = HashObject::from(
            &mut vec!["-w".to_string(), "test_dir/objetos/archivo.txt".to_string()],
            logger.clone(),
        )
        .unwrap();
        let hash = hash_object.ejecutar().unwrap();
        let mut cat_file = CatFile {
            logger,
            visualizacion: Visualizaciones::TipoObjeto,
            hash_objeto: hash.to_string(),
        };
        let tipo_objeto = cat_file.ejecutar().unwrap();
        assert_eq!(tipo_objeto, "blob");
    }

    #[test]
    #[serial]
    fn test04_pretty_print_tree_muestra_el_contenido_correcto() {
        let contenido = "40000 test_dir\0d1bd5884df89a9734e3b0a4e7721a4802d85cce8100644 test_file.txt\0678e12dc5c03a7cf6e9f64e688868962ab5d8b65".to_string();
        let pretty_print = conseguir_contenido_pretty("tree 109", &contenido).unwrap();
        assert_eq!(pretty_print, "040000 tree d1bd5884df89a9734e3b0a4e7721a4802d85cce8   test_dir\n100644 blob 678e12dc5c03a7cf6e9f64e688868962ab5d8b65   test_file.txt\n");
    }

    #[test]
    #[serial]
    fn test05_conseguir_tipo_tree_muestra_el_tipo_de_objeto_correcto() {
        let tipo_objeto = conseguir_tipo_objeto("tree 109").unwrap();
        assert_eq!(tipo_objeto, "tree");
    }

    #[test]
    #[serial]
    fn test06_conseguir_tamanio_tree_muestra_el_tamanio_correcto() {
        let tamanio = conseguir_tamanio("tree 109").unwrap();
        assert_eq!(tamanio, "109");
    }

    #[test]
    #[serial]
    fn test07_pretty_print_commit_muestra_el_contenido_correcto() {
        let contenido = "tree c475b36be7b222b7ff1469b44b15cdc0f754ef44\n
        parent b557332b86888546cecbe81933cf22adb1f3fed1\n
        author aaaa <bbbb> 1698535611 -0300\n
        committer aaaa <bbbb> 1698535611 -0300'n";
        let pretty_print = conseguir_contenido_pretty("commit 29", contenido).unwrap();
        assert_eq!(pretty_print, contenido);
    }

    #[test]
    #[serial]
    fn test08_conseguir_tipo_commit_muestra_el_tipo_de_objeto_correcto() {
        let tipo_objeto = conseguir_tipo_objeto("commit 109").unwrap();
        assert_eq!(tipo_objeto, "commit");
    }

    #[test]
    #[serial]
    fn test09_conseguir_tamanio_commit_muestra_el_tamanio_correcto() {
        let tamanio = conseguir_tamanio("commit 29").unwrap();
        assert_eq!(tamanio, "29");
    }
}
