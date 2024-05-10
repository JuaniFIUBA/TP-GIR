use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger, objeto::Objeto},
    utils::{
        index::{crear_index, escribir_index, leer_index, ObjetoIndex},
        io::{self, rm_directorio},
    },
};

pub struct Remove {
    pub ubicaciones: Vec<PathBuf>,
    pub cached: bool,
    pub logger: Arc<Logger>,
    pub index: Vec<ObjetoIndex>,
}

impl Remove {
    /// Elimina los directorios vacios que quedaron remanentes luego de eliminar los
    /// archivos
    fn limpiar_directorios_vacios(&self) {
        let mut ubicaciones_a_corroborar: HashSet<PathBuf> = HashSet::new();

        for ubicacion in self.ubicaciones.clone() {
            let mut ubicacion_actual = ubicacion.clone();
            loop {
                ubicacion_actual = ubicacion_actual.parent().unwrap().to_path_buf();
                if ubicacion_actual == PathBuf::from("") {
                    break;
                }
                if ubicaciones_a_corroborar.contains(&ubicacion_actual) {
                    break;
                }
                ubicaciones_a_corroborar.insert(ubicacion_actual.clone());
            }
        }

        for ubicacion in ubicaciones_a_corroborar {
            if !ubicacion.is_dir() {
                continue;
            }

            let hijos = std::fs::read_dir(&ubicacion)
                .map_err(|_| "Error al obtener hijos de directorio".to_string())
                .unwrap();

            if hijos.count() == 0 {
                io::rm_directorio(ubicacion).unwrap();
            }
        }
    }

    /// Dada una lista de ubicaciones, devuelve una lista con las ubicaciones hoja
    /// (las ubicaciones hoja son los archivos que se encuentran en las ubicaciones)
    pub fn obtener_ubicaciones_hoja(
        ubicaciones: Vec<PathBuf>,
        recursivo: bool,
    ) -> Result<Vec<PathBuf>, String> {
        let mut ubicaciones_hoja: Vec<PathBuf> = Vec::new();
        for ubicacion in ubicaciones {
            if ubicacion.is_file() {
                ubicaciones_hoja.push(ubicacion);
            } else if ubicacion.is_dir() {
                if !recursivo {
                    Err("No se puede borrar un directorio sin la opcion -r".to_string())?;
                }
                let mut directorios = std::fs::read_dir(ubicacion)
                    .map_err(|_| "Error al obtener directorios hoja".to_string())?;
                while let Some(Ok(directorio)) = directorios.next() {
                    let path = directorio.path();
                    if path.is_file() {
                        ubicaciones_hoja.push(path);
                    } else if path.is_dir() {
                        ubicaciones_hoja
                            .append(&mut Self::obtener_ubicaciones_hoja(vec![path], true)?);
                    }
                }
            }
        }
        Ok(ubicaciones_hoja)
    }

    /// Crea un nuevo Remove a partir de los argumentos recibidos.
    /// Toma como flags validos --cached y -r.
    /// Luego toma todos los argumentos siguientes como una ubicacion a borrar.
    pub fn from(args: Vec<String>, logger: Arc<Logger>) -> Result<Remove, String> {
        crear_index();
        let index = leer_index(logger.clone())?;
        let mut ubicaciones_recibidas: Vec<PathBuf> = Vec::new();
        let mut cached = false;
        let mut recursivo = false;

        for arg in args.iter() {
            match arg.as_str() {
                "--cached" => {
                    cached = true;
                }
                "-r" => {
                    recursivo = true;
                }
                ubicacion => {
                    ubicaciones_recibidas.push(PathBuf::from(ubicacion));
                }
            }
        }

        let ubicaciones: Vec<PathBuf> =
            Self::obtener_ubicaciones_hoja(ubicaciones_recibidas, recursivo)?;

        Ok(Remove {
            logger: logger.clone(),
            ubicaciones,
            index,
            cached,
        })
    }
}

impl Ejecutar for Remove {
    /// Ejecuta el comando remove.
    /// Si cached es true, lo elimina del historial de commits pero lo conserva en el disco.
    /// Si cached es false, lo elimina del historial de commits y del disco.
    /// Si es recursive, elimina los archivos de los directorios recursivamente.
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Ejecutando remove");

        for ubicacion in self.ubicaciones.clone() {
            if ubicacion.is_dir() {
                Err("No se puede borrar un directorio sin la opcion -r".to_string())?;
            }
            let nuevo_objeto =
                Objeto::from_directorio(ubicacion.clone(), None, self.logger.clone())?;
            let nuevo_objeto_index = ObjetoIndex {
                merge: false,
                es_eliminado: true,
                objeto: nuevo_objeto.clone(),
            };

            let indice = self
                .index
                .iter()
                .position(|objeto_index| match objeto_index.objeto {
                    Objeto::Blob(ref blob) => blob.ubicacion == ubicacion,
                    Objeto::Tree(_) => false,
                });

            if indice.is_some() {
                Err("No se puede borrar un archivo en el index".to_string())?;
            } else {
                self.index.push(nuevo_objeto_index);
            }

            if !self.cached {
                rm_directorio(ubicacion)?;
            }
        }
        escribir_index(self.logger.clone(), &mut self.index)?;
        self.limpiar_directorios_vacios();

        Ok("".to_string())
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::tipos_de_dato::{
        comando::Ejecutar,
        comandos::{add::Add, commit::Commit},
    };

    use super::*;

    fn crear_test_file(contenido: &str) {
        io::escribir_bytes("tmp/rm_test.txt", contenido).unwrap();
    }

    fn existe_test_file() -> bool {
        PathBuf::from("tmp/rm_test.txt").exists()
    }

    fn clear_index() {
        let _ = std::fs::remove_file(".gir/index");
    }

    fn crear_archivo_en_dir(contenido: &str) {
        io::escribir_bytes("tmp/test_dir/testfile.txt", contenido).unwrap();
    }

    fn existe_archivo_en_dir() -> bool {
        PathBuf::from("tmp/test_dir/testfile.txt").exists()
    }

    fn existe_dir() -> bool {
        PathBuf::from("tmp/test_dir").exists()
    }

    #[test]
    #[serial]
    fn test01_remove_ejecutar() {
        clear_index();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/rm_test01")).unwrap());
        Add::from(vec!["test_file.txt".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        Commit::from(
            &mut vec!["-m".to_string(), "mensaje".to_string()],
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let args = vec!["--cached".to_string(), "test_file.txt".to_string()];
        Remove::from(args, logger).unwrap().ejecutar().unwrap();

        let index = io::leer_a_string(".gir/index").unwrap();
        assert_eq!(
            index,
            "- 0 100644 678e12dc5c03a7cf6e9f64e688868962ab5d8b65 test_file.txt\n"
        )
    }

    #[test]
    #[serial]
    fn test02_remove_recursivo() {
        clear_index();
        crear_archivo_en_dir("test02");
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/rm_test01")).unwrap());
        Add::from(vec!["tmp/test_dir".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        Commit::from(
            &mut vec!["-m".to_string(), "mensaje".to_string()],
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let args = vec![
            "--cached".to_string(),
            "-r".to_string(),
            "tmp/test_dir".to_string(),
        ];
        Remove::from(args, logger).unwrap().ejecutar().unwrap();

        let index = io::leer_a_string(".gir/index").unwrap();

        assert_eq!(
            index,
            "- 0 100644 5b88c81cf6242742a0920887170cff76a3267e50 tmp/test_dir/testfile.txt\n"
        )
    }

    #[test]
    #[serial]
    fn test03_remove_sin_cached() {
        clear_index();
        crear_test_file("test03");
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/rm_test01")).unwrap());
        Add::from(vec!["tmp/rm_test.txt".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        Commit::from(
            &mut vec!["-m".to_string(), "mensaje".to_string()],
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let args = vec!["tmp/rm_test.txt".to_string()];
        let mut remove = Remove::from(args, logger).unwrap();
        remove.ejecutar().unwrap();

        let index = io::leer_a_string(".gir/index").unwrap();

        assert_eq!(
            index,
            "- 0 100644 bd64bee30820f16fa35105d7cd589131812c4d67 tmp/rm_test.txt\n"
        );
        assert!(!existe_test_file());
    }

    #[test]
    #[serial]
    fn test04_remove_recursivo_sin_cached() {
        clear_index();
        crear_archivo_en_dir("test04");

        let logger = Arc::new(Logger::new(PathBuf::from("tmp/rm_test01")).unwrap());
        Add::from(vec!["tmp/test_dir".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        Commit::from(
            &mut vec!["-m".to_string(), "mensaje".to_string()],
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let args = vec!["-r".to_string(), "tmp/test_dir".to_string()];
        Remove::from(args, logger).unwrap().ejecutar().unwrap();

        let index = io::leer_a_string(".gir/index").unwrap();

        assert_eq!(
            index,
            "- 0 100644 46bc7488b30b93a1c3e6f2e3f6f3fd200c662ad2 tmp/test_dir/testfile.txt\n"
        );

        assert!(!existe_archivo_en_dir());
        assert!(!existe_dir());
    }

    #[test]
    #[serial]
    #[should_panic(expected = "No se puede borrar un directorio sin la opcion -r")]
    fn test05_remove_directorio_no_recursivo_falla() {
        clear_index();
        crear_archivo_en_dir("test05");

        let logger = Arc::new(Logger::new(PathBuf::from("tmp/rm_test01")).unwrap());
        Add::from(vec!["tmp/test_dir".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        Commit::from(
            &mut vec!["-m".to_string(), "mensaje".to_string()],
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let args = vec!["tmp/test_dir".to_string()];
        Remove::from(args, logger).unwrap().ejecutar().unwrap();
    }
}
