use std::{path::PathBuf, sync::Arc};

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger, objeto::Objeto},
    utils::index::{crear_index, escribir_index, leer_index, ObjetoIndex},
};

use super::{check_ignore::CheckIgnore, status::obtener_arbol_del_commit_head};

pub struct Add {
    /// Logger para imprimir los mensajes en el archivo log.
    logger: Arc<Logger>,
    /// Ubicaciones de los archivos a agregar.
    ubicaciones: Vec<PathBuf>,
    /// Objetos que ya estan en el index del repositorio.
    index: Vec<ObjetoIndex>,
}

impl Add {
    /// Devuelve un vector con las ubicaciones de cada archivo dentro de las ubicaciones que se le pasaron.
    /// Si se le pasa un archivo, devuelve el path de ese archivo.
    /// Si se le pasa un directorio, devuelve el path de todos los archivos que se encuentran dentro de ese directorio.
    pub fn obtener_ubicaciones_hoja(ubicaciones: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
        let mut ubicaciones_hoja: Vec<PathBuf> = Vec::new();
        for ubicacion in ubicaciones {
            if ubicacion.is_file() {
                ubicaciones_hoja.push(ubicacion);
            } else if ubicacion.is_dir() {
                let mut directorios = std::fs::read_dir(ubicacion)
                    .map_err(|_| "Error al obtener directorios hoja".to_string())?;
                while let Some(Ok(directorio)) = directorios.next() {
                    let path = directorio.path();
                    if path.is_file() {
                        ubicaciones_hoja.push(path);
                    } else if path.is_dir() {
                        ubicaciones_hoja.append(&mut Self::obtener_ubicaciones_hoja(vec![path])?);
                    }
                }
            }
        }
        Ok(ubicaciones_hoja)
    }

    /// Crea un comando add a partir de los argumentos pasados por linea de comandos.
    pub fn from(args: Vec<String>, logger: Arc<Logger>) -> Result<Add, String> {
        crear_index();
        let index = leer_index(logger.clone())?;
        let ubicaciones_recibidas = args.iter().map(PathBuf::from).collect::<Vec<PathBuf>>();
        let ubicaciones: Vec<PathBuf> = Self::obtener_ubicaciones_hoja(ubicaciones_recibidas)?;
        Ok(Add {
            logger,
            ubicaciones,
            index,
        })
    }

    /// Crea un objeto index a partir de una ubicacion.
    fn crear_objeto_index_from_ubicacion(
        ubicacion: PathBuf,
        logger: Arc<Logger>,
    ) -> Result<ObjetoIndex, String> {
        let nuevo_objeto = Objeto::from_directorio(ubicacion.clone(), None, logger.clone())?;

        Ok(ObjetoIndex {
            merge: false,
            es_eliminado: false,
            objeto: nuevo_objeto.clone(),
        })
    }

    /// Dada una ubicacion, devuelve el indice del objeto index que contiene esa ubicacion.
    /// Si no se encuentra, devuelve None.
    fn obtener_indice_objeto_index(&self, ubicacion: PathBuf) -> Option<usize> {
        self.index
            .iter()
            .position(|objeto_index| match objeto_index.objeto {
                Objeto::Blob(ref blob) => blob.ubicacion == ubicacion,
                Objeto::Tree(_) => false,
            })
    }

    /// Agrega un objeto index al index.
    /// Si el objeto ya se encuentra en el index, actualiza el objeto.
    /// Si el objeto contiene la misma version que en el commit anterior, no lo agrega.
    /// Si el objeto tiene modificaciones, lo agrega.
    fn aniadir_ubicacion_pedida_al_index(&mut self, ubicacion: PathBuf) -> Result<(), String> {
        let nuevo_objeto_index =
            Self::crear_objeto_index_from_ubicacion(ubicacion.clone(), self.logger.clone())?;

        let indice = self.obtener_indice_objeto_index(ubicacion);

        if let Some(i) = indice {
            self.index[i] = nuevo_objeto_index;
        } else {
            let tree_head = obtener_arbol_del_commit_head(self.logger.clone());
            if let Some(tree_head) = tree_head {
                if tree_head.contiene_misma_version_hijo(
                    &nuevo_objeto_index.objeto.obtener_hash(),
                    &nuevo_objeto_index.objeto.obtener_path(),
                ) {
                    return Ok(());
                }
            }
            self.index.push(nuevo_objeto_index);
        }
        Ok(())
    }
}

impl Ejecutar for Add {
    /// Ejecuta el comando add.
    /// Agrega los archivos pasados por parametro al index.
    /// Si el archivo ya se encuentra en el index, actualiza el objeto.
    /// Si el archivo contiene la misma version que en el commit anterior, no lo agrega.
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Ejecutando add");

        for ubicacion in self.ubicaciones.clone() {
            if CheckIgnore::es_directorio_a_ignorar(&ubicacion, self.logger.clone())? {
                continue;
            }

            self.logger.log(&format!(
                "Agregando {} al index",
                ubicacion
                    .to_str()
                    .ok_or_else(|| "Path invalido".to_string())?,
            ));
            if ubicacion.is_dir() {
                Err("No se puede agregar un directorio")?;
            }
            self.aniadir_ubicacion_pedida_al_index(ubicacion)?;
        }
        escribir_index(self.logger.clone(), &mut self.index)?;
        Ok("".to_string())
    }
}
#[cfg(test)]

mod tests {
    use serial_test::serial;
    use std::{io::Write, path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::{add::Add, init::Init},
            logger::Logger,
            objeto::Objeto,
        },
        utils::io,
    };

    fn create_test_file() {
        let mut file = std::fs::File::create("test_file.txt").unwrap();
        let _ = file.write_all(b"test file");
    }

    fn modify_test_file() {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open("test_file.txt")
            .unwrap();
        let _ = file.write_all(b"test file modified");
    }

    fn limpiar_archivo_gir() {
        let _ = io::rm_directorio(".gir");
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_init")).unwrap());
        let mut init = Init {
            path: "./.gir".to_string(),
            logger,
        };
        init.ejecutar().unwrap();
    }

    #[test]
    #[serial]
    fn test01_archivo_vacio_se_llena_con_objeto_agregado() {
        limpiar_archivo_gir();
        create_test_file();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/add_test01")).unwrap());
        let ubicacion = "test_file.txt".to_string();
        let mut add = Add::from(vec![ubicacion], logger.clone()).unwrap();

        add.ejecutar().unwrap();

        assert_eq!(add.index.len(), 1);

        let file = io::leer_a_string("./.gir/index").unwrap();
        assert_eq!(
            file,
            "+ 0 100644 bdf08de0f3095da5030fecd9bafc0b00c1aced7c test_file.txt\n"
        );
    }

    #[test]
    #[serial]
    fn test02_archivo_con_objeto_actualiza_el_objeto() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/add_test02")).unwrap());

        create_test_file();
        let ubicacion = "test_file.txt".to_string();
        let mut add = Add::from(vec![ubicacion], logger.clone()).unwrap();

        add.ejecutar().unwrap();

        modify_test_file();
        let ubicacion = "test_file.txt".to_string();
        let mut add = Add::from(vec![ubicacion], logger.clone()).unwrap();

        add.ejecutar().unwrap();

        assert_eq!(add.index.len(), 1);

        let objeto = &add.index[0].objeto;
        if let Objeto::Blob(blob) = objeto {
            assert_eq!(blob.nombre, "test_file.txt");
            assert_eq!(blob.hash, "678e12dc5c03a7cf6e9f64e688868962ab5d8b65");
        }

        let file = io::leer_a_string("./.gir/index").unwrap();
        assert_eq!(
            file,
            "+ 0 100644 678e12dc5c03a7cf6e9f64e688868962ab5d8b65 test_file.txt\n"
        );
    }

    #[test]
    #[serial]
    fn test03_agregar_un_objeto_en_un_directorio() {
        limpiar_archivo_gir();

        let logger = Arc::new(Logger::new(PathBuf::from("tmp/add_test03")).unwrap());

        let path = "test_dir/objetos/archivo.txt".to_string();
        let mut add = Add::from(vec![path], logger.clone()).unwrap();
        add.ejecutar().unwrap();

        let file = io::leer_a_string("./.gir/index").unwrap();

        assert_eq!(
            file,
            "+ 0 100644 2b824e648965b94c6c6b3dd0702feb91f699ed62 test_dir/objetos/archivo.txt\n"
        );
    }

    #[test]
    #[serial]
    fn test04_archivo_con_objetos_agrega_nuevos_objetos() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/add_test04")).unwrap());
        let ubicacion = "test_file.txt".to_string();

        let mut add = Add::from(vec![ubicacion], logger.clone()).unwrap();
        add.ejecutar().unwrap();

        let ubicacion = "test_dir/objetos/archivo.txt".to_string();

        let mut add = Add::from(vec![ubicacion], logger.clone()).unwrap();
        add.ejecutar().unwrap();

        assert_eq!(add.index.len(), 2);

        if let Objeto::Blob(blob) = &add.index[1].objeto {
            assert_eq!(blob.nombre, "test_file.txt");
            assert_eq!(blob.hash, "678e12dc5c03a7cf6e9f64e688868962ab5d8b65");
        }

        if let Objeto::Blob(blob) = &add.index[0].objeto {
            assert_eq!(blob.nombre, "archivo.txt");
            assert_eq!(blob.hash, "2b824e648965b94c6c6b3dd0702feb91f699ed62");
        }

        let file = io::leer_a_string("./.gir/index").unwrap();

        assert_eq!(
            file,
            "+ 0 100644 2b824e648965b94c6c6b3dd0702feb91f699ed62 test_dir/objetos/archivo.txt\n+ 0 100644 678e12dc5c03a7cf6e9f64e688868962ab5d8b65 test_file.txt\n"
        );
    }

    #[test]
    #[serial]
    fn test05_agregar_un_directorio_al_index() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/add_test05")).unwrap());

        let path = "test_dir/muchos_objetos".to_string();
        let mut add = Add::from(vec![path], logger.clone()).unwrap();
        add.ejecutar().unwrap();

        let file = io::leer_a_string("./.gir/index").unwrap();

        assert_eq!(
            file,
            "+ 0 100644 ba1d9d6871ba93f7e070c8663e6739cc22f07d3f test_dir/muchos_objetos/archivo.txt\n+ 0 100644 2b824e648965b94c6c6b3dd0702feb91f699ed62 test_dir/muchos_objetos/archivo_copy.txt\n"
        );
    }

    #[test]
    #[serial]
    fn test06_agregar_dos_archivos_de_una() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/add_test06")).unwrap());
        let ubicacion = "test_file.txt".to_string();

        let ubicacion2 = "test_dir/objetos/archivo.txt".to_string();

        let mut add = Add::from(vec![ubicacion, ubicacion2], logger.clone()).unwrap();
        add.ejecutar().unwrap();

        assert_eq!(add.index.len(), 2);

        if let Objeto::Blob(blob) = &add.index[0].objeto {
            assert_eq!(blob.nombre, "archivo.txt");
            assert_eq!(blob.hash, "2b824e648965b94c6c6b3dd0702feb91f699ed62");
        }

        if let Objeto::Blob(blob) = &add.index[1].objeto {
            assert_eq!(blob.nombre, "test_file.txt");
            assert_eq!(blob.hash, "678e12dc5c03a7cf6e9f64e688868962ab5d8b65");
        }

        let file = io::leer_a_string("./.gir/index").unwrap();

        assert_eq!(
            file,
            "+ 0 100644 2b824e648965b94c6c6b3dd0702feb91f699ed62 test_dir/objetos/archivo.txt\n+ 0 100644 678e12dc5c03a7cf6e9f64e688868962ab5d8b65 test_file.txt\n"
        );
    }
}
