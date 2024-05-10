use std::{path::PathBuf, sync::Arc};

use super::{
    logger::Logger,
    objetos::{blob::Blob, tree::Tree},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Objeto {
    Tree(Tree),
    Blob(Blob),
}

/// Devuelve true si el flag es un objeto valido.
pub fn flag_es_un_objeto_(flag: &str) -> bool {
    flag == "blob" || flag == "tree" || flag == "commit" || flag == "tag"
}

impl Objeto {
    /// Devuelve el path del objeto.
    pub fn obtener_path(&self) -> PathBuf {
        match self {
            Objeto::Tree(tree) => tree.directorio.clone(),
            Objeto::Blob(blob) => blob.ubicacion.clone(),
        }
    }

    /// Devuelve el hash del objeto.
    pub fn obtener_hash(&self) -> String {
        match self {
            Objeto::Tree(tree) => match tree.obtener_hash() {
                Ok(hash) => hash,
                Err(err) => err.to_string(),
            },
            Objeto::Blob(blob) => blob.obtener_hash(),
        }
    }

    /// Devuelve el tamanio del objeto.
    pub fn obtener_tamanio(&self) -> Result<usize, String> {
        match self {
            Objeto::Tree(tree) => Ok(tree.obtener_tamanio()?),
            Objeto::Blob(blob) => Ok(blob.obtener_tamanio()?),
        }
    }

    /// Dada una linea en formato del archivo index, devuelve una instancia del objeto.
    /// Si el modo es 100644 devuelve un Blob
    /// Si el modo es 40000 devuelve un Tree
    pub fn from_index(linea_index: &str, logger: Arc<Logger>) -> Result<Objeto, String> {
        let mut line = linea_index.split_whitespace();

        let modo = line.next().ok_or("Error al leer el modo")?;
        let hash = line.next().ok_or("Error al leer el hash")?;
        let ubicacion_string = line.next().ok_or("Error al leer la ubicacion")?;

        let ubicacion = PathBuf::from(ubicacion_string);
        let nombre = match ubicacion_string.split('/').last() {
            Some(nombre) => nombre,
            None => Err("Error al leer el nombre")?,
        };

        match modo {
            "100644" => Ok(Objeto::Blob(Blob {
                nombre: nombre.to_string(),
                ubicacion,
                hash: hash.to_string(),
                logger,
            })),
            "40000" => {
                let tree = Tree::from_hash(hash, ubicacion, logger)?;
                Ok(Objeto::Tree(tree))
            }
            _ => Err("Modo no soportado".to_string()),
        }
    }

    /// Devuelve una instancia del objeto en el directorio indicado
    /// Si el directorio es un archivo, devuelve un Blob
    /// Si el directorio es un directorio, devuelve un Tree
    pub fn from_directorio(
        mut directorio: PathBuf,
        hijos_especificados: Option<&Vec<PathBuf>>,
        logger: Arc<Logger>,
    ) -> Result<Objeto, String> {
        if directorio.starts_with("./") && directorio != PathBuf::from("./") {
            directorio = match directorio.strip_prefix("./") {
                Ok(dir) => dir.to_path_buf(),
                Err(_) => return Err(format!("No se pudo leer el directorio {directorio:#?}")),
            };
        }

        if directorio.is_dir() {
            let tree = Tree::from_directorio(directorio.clone(), hijos_especificados, logger)?;
            Ok(Objeto::Tree(tree))
        } else if directorio.is_file() {
            let blob = Blob::from_directorio(directorio.clone(), logger)?;
            Ok(Objeto::Blob(blob))
        } else {
            Err(format!("No se pudo leer el directorio {directorio:#?}"))
        }
    }
}

#[cfg(test)]

mod tests {
    use serial_test::serial;

    use crate::tipos_de_dato::{comando::Ejecutar, comandos::add::Add, logger};

    use super::*;

    #[test]
    #[serial]
    fn test01_blob_from_index() {
        let logger = Arc::new(logger::Logger::new(PathBuf::from("tmp/objeto_test01")).unwrap());
        let objeto = Objeto::from_index("100644 1234567890 ./hola.txt", logger.clone()).unwrap();
        assert_eq!(
            objeto,
            Objeto::Blob(Blob {
                nombre: "hola.txt".to_string(),
                hash: "1234567890".to_string(),
                ubicacion: PathBuf::from("./hola.txt"),
                logger: logger.clone()
            })
        );
    }

    #[test]
    #[serial]
    fn test02_blob_from_directorio() {
        let logger = Arc::new(logger::Logger::new(PathBuf::from("tmp/objeto_test02")).unwrap());
        let objeto = Objeto::from_directorio(
            PathBuf::from("test_dir/objetos/archivo.txt"),
            None,
            logger.clone(),
        )
        .unwrap();

        assert_eq!(
            objeto,
            Objeto::Blob(Blob {
                nombre: "archivo.txt".to_string(),
                hash: "2b824e648965b94c6c6b3dd0702feb91f699ed62".to_string(),
                ubicacion: PathBuf::from("test_dir/objetos/archivo.txt"),
                logger
            })
        );
    }

    #[test]
    #[serial]

    fn test03_tree_from_directorio() {
        let logger = Arc::new(logger::Logger::new(PathBuf::from("tmp/objeto_test03")).unwrap());
        let objeto =
            Objeto::from_directorio(PathBuf::from("test_dir/objetos"), None, logger.clone())
                .unwrap();

        let hijo = Objeto::Blob(Blob {
            nombre: "archivo.txt".to_string(),
            hash: "2b824e648965b94c6c6b3dd0702feb91f699ed62".to_string(),
            ubicacion: PathBuf::from("test_dir/objetos/archivo.txt"),
            logger: logger.clone(),
        });

        assert_eq!(
            objeto,
            Objeto::Tree(Tree {
                directorio: PathBuf::from("test_dir/objetos"),
                objetos: vec![hijo],
                logger
            })
        );
    }

    #[test]
    #[serial]
    fn test04_tree_from_index() {
        let logger = Arc::new(logger::Logger::new(PathBuf::from("tmp/objeto_test04")).unwrap());
        let objeto_a_escibir =
            Objeto::from_directorio(PathBuf::from("test_dir"), None, logger.clone()).unwrap();

        if let Objeto::Tree(ref tree) = objeto_a_escibir {
            tree.escribir_en_base().unwrap();
        } else {
            panic!("No se pudo leer el directorio");
        }

        let objeto = Objeto::from_index(
            &format!("40000 {} test_dir", objeto_a_escibir.obtener_hash()),
            logger.clone(),
        )
        .unwrap();

        let nieto_1 = Objeto::Blob(Blob {
            nombre: "archivo.txt".to_string(),
            hash: "2b824e648965b94c6c6b3dd0702feb91f699ed62".to_string(),
            ubicacion: PathBuf::from("test_dir/objetos/archivo.txt"),
            logger: logger.clone(),
        });

        let nieto_2 = Objeto::Blob(Blob {
            nombre: "archivo.txt".to_string(),
            hash: "ba1d9d6871ba93f7e070c8663e6739cc22f07d3f".to_string(),
            ubicacion: PathBuf::from("test_dir/muchos_objetos/archivo.txt"),
            logger: logger.clone(),
        });

        let nieto_3 = Objeto::Blob(Blob {
            nombre: "archivo_copy.txt".to_string(),
            hash: "2b824e648965b94c6c6b3dd0702feb91f699ed62".to_string(),
            ubicacion: PathBuf::from("test_dir/muchos_objetos/archivo_copy.txt"),
            logger: logger.clone(),
        });

        let hijo_1 = Objeto::Tree(Tree {
            directorio: PathBuf::from("test_dir/objetos"),
            objetos: vec![nieto_1],
            logger: logger.clone(),
        });

        let hijo_2 = Objeto::Tree(Tree {
            directorio: PathBuf::from("test_dir/muchos_objetos"),
            objetos: Tree::ordenar_objetos_alfabeticamente(&[nieto_2, nieto_3]),
            logger: logger.clone(),
        });

        assert_eq!(
            objeto,
            Objeto::Tree(Tree {
                directorio: PathBuf::from("test_dir"),
                objetos: Tree::ordenar_objetos_alfabeticamente(&[hijo_1, hijo_2]),
                logger: logger.clone(),
            })
        );
    }

    #[test]
    #[serial]
    fn test05_obtener_tamanio_blob() {
        let logger = Arc::new(logger::Logger::new(PathBuf::from("tmp/objeto_test05")).unwrap());
        let objeto = Objeto::from_directorio(
            PathBuf::from("test_dir/objetos/archivo.txt"),
            None,
            logger.clone(),
        )
        .unwrap();
        Add::from(
            vec!["test_dir/objetos/archivo.txt".to_string()],
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        assert_eq!(objeto.obtener_tamanio().unwrap(), 23);
    }
}
