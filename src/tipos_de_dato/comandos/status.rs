use std::{
    path::{self, PathBuf},
    sync::Arc,
};

const ROJO: &str = "\x1B[31m";
const VERDE: &str = "\x1B[32m";
const RESET: &str = "\x1B[0m";

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger, objeto::Objeto, objetos::tree::Tree},
    utils::{
        index::{leer_index, ObjetoIndex},
        io, ramas,
    },
};

use super::{check_ignore::CheckIgnore, write_tree::conseguir_arbol_en_directorio};

pub struct Status {
    /// Logger para registrar los eventos ocurridos durante la ejecucion del comando.
    logger: Arc<Logger>,
    /// Vector de objetos que estan presentes en el archivo index.
    index: Vec<ObjetoIndex>,
    /// Arbol del commit al que apunta la rama actual.
    tree_commit_head: Option<Tree>,
    /// Arbol del directorio actual.
    tree_directorio_actual: Tree,
}

/// Obtiene el arbol del commit al que apunta la rama actual.
/// En caso de no haber un commit devuelve None.
pub fn obtener_arbol_del_commit_head(logger: Arc<Logger>) -> Option<Tree> {
    let ruta = match ramas::obtener_gir_dir_rama_actual() {
        Ok(ruta) => format!("{}", ruta.display()),
        Err(_) => return None,
    };
    let padre_commit = io::leer_a_string(path::Path::new(&ruta)).unwrap_or_else(|_| "".to_string());
    if padre_commit.is_empty() {
        None
    } else {
        let hash_arbol_commit =
            conseguir_arbol_en_directorio(&padre_commit, ".gir/objects/").ok()?;
        let tree =
            Tree::from_hash(&hash_arbol_commit, PathBuf::from("./"), logger.clone()).unwrap();
        Some(tree)
    }
}

impl Status {
    /// Crea un comando status a partir de los argumentos pasados por linea de comandos.
    pub fn from(logger: Arc<Logger>) -> Result<Status, String> {
        let index = leer_index(logger.clone())?;
        let tree_commit_head = obtener_arbol_del_commit_head(logger.clone());
        let tree_directorio_actual =
            Tree::from_directorio(PathBuf::from("./"), None, logger.clone())?;
        Ok(Status {
            logger,
            index,
            tree_commit_head,
            tree_directorio_actual,
        })
    }

    /// Obtiene los cambios que se encuentran en el index.
    /// Devuelve un vector con los cambios formateados segun su respectivo tipo de cambio.
    /// Si el archivo no se encuentra en el commit anterior, se considera un nuevo archivo.
    pub fn obtener_staging(&self) -> Result<Vec<String>, String> {
        let mut staging: Vec<String> = Vec::new();
        for objeto_index in &self.index {
            match self.tree_commit_head {
                Some(ref tree) => {
                    let tipo_cambio =
                        if tree.contiene_hijo_por_ubicacion(objeto_index.objeto.obtener_path()) {
                            match objeto_index.es_eliminado {
                                true => "eliminado",
                                false => "modificado",
                            }
                        } else {
                            "nuevo archivo"
                        };
                    let linea_formateada = format!(
                        "{}: {}",
                        tipo_cambio,
                        objeto_index.objeto.obtener_path().display()
                    );
                    staging.push(linea_formateada);
                }
                None => {
                    let linea_formateada = format!(
                        "nuevo archivo: {}",
                        objeto_index.objeto.obtener_path().display()
                    );
                    staging.push(linea_formateada);
                    continue;
                }
            }
        }
        Ok(staging)
    }

    /// Obtiene los archivos que quedaron con conflictos luego de realizar un merge.
    /// Devuelve un vector con los mismos formateados indicando que quedaron con conflictos.
    fn obtener_archivos_unmergeados(&self) -> Result<Vec<String>, String> {
        let mut unmergeados = Vec::new();
        for objeto_index in &self.index {
            if objeto_index.merge {
                let linea_formateada = format!(
                    "unmergeado: {}",
                    objeto_index.objeto.obtener_path().display()
                );
                unmergeados.push(linea_formateada);
            }
        }
        Ok(unmergeados)
    }

    /// Obtiene los archivos que fueron commiteados anteriormente, fueron modificados en el directorio actual y no estan en el index.
    /// Devuelve un vector con los cambios formateados indicando que fueron modificados.
    /// Si el archivo no se encuentra en el commit anterior, no se considera un cambio a trackear.
    /// Si el archivo se encuentra en el index, no se considera un cambio a trackear.
    pub fn obtener_trackeados(&self) -> Result<Vec<String>, String> {
        let mut trackeados = Vec::new();
        let tree_head = match self.tree_commit_head {
            Some(ref tree) => tree,
            None => return Ok(trackeados),
        };
        for objeto in self.tree_directorio_actual.obtener_objetos_hoja() {
            if tree_head.contiene_hijo_por_ubicacion(objeto.obtener_path())
                && !tree_head
                    .contiene_misma_version_hijo(&objeto.obtener_hash(), &objeto.obtener_path())
                && !self.index_contiene_objeto(&objeto)
            {
                trackeados.push(format!("modificado: {}", objeto.obtener_path().display()));
            }
        }
        Ok(trackeados)
    }

    /// Obtiene los archivos hijos de u tree que nunca fueron commiteados anteriormente y no estan en el index.
    /// Devuelve un vector con las ubicaciones de los archivos.
    fn obtener_hijos_untrackeados(
        &self,
        tree: &Tree,
        tree_head: &Tree,
    ) -> Result<Vec<String>, String> {
        let mut untrackeados = Vec::new();

        for objeto in tree.objetos.iter() {
            if self.index_contiene_objeto(objeto) {
                continue;
            }
            if CheckIgnore::es_directorio_a_ignorar(&objeto.obtener_path(), self.logger.clone())? {
                continue;
            }
            match objeto {
                Objeto::Blob(_) => {
                    if !tree_head.contiene_hijo_por_ubicacion(objeto.obtener_path()) {
                        untrackeados.push(format!("{}", objeto.obtener_path().display()));
                    }
                }
                Objeto::Tree(ref tree) => {
                    if !tree_head.contiene_directorio(&objeto.obtener_path()) {
                        untrackeados.push(format!("{}/", objeto.obtener_path().display()));
                    } else {
                        let mut untrackeados_hijos =
                            self.obtener_hijos_untrackeados(tree, tree_head)?;

                        untrackeados.append(&mut untrackeados_hijos);
                    }
                }
            }
        }
        Ok(untrackeados)
    }

    /// Devuelve true si el index contiene el objeto pasado por parametro.
    fn index_contiene_objeto(&self, objeto: &Objeto) -> bool {
        let bool = self.index.iter().any(|objeto_index| match objeto {
            Objeto::Blob(ref blob) => blob.obtener_hash() == objeto_index.objeto.obtener_hash(),
            Objeto::Tree(ref tree) => tree.contiene_misma_version_hijo(
                &objeto_index.objeto.obtener_hash(),
                &objeto_index.objeto.obtener_path(),
            ),
        });
        bool
    }

    /// Obtiene los archivos que nunca fueron commiteados anteriormente y no estan en el index.
    /// En el caso de no haber commit anterior, se considera que todos los archivos son untrackeados.
    /// Devuelve un vector con las ubicaciones de los archivos untrackeados.
    pub fn obtener_untrackeados(&self) -> Result<Vec<String>, String> {
        let tree_head = match self.tree_commit_head {
            Some(ref tree) => tree,
            None => {
                let mut untrackeados = Vec::new();
                for objeto in self.tree_directorio_actual.objetos.iter() {
                    if self.index_contiene_objeto(objeto) {
                        continue;
                    }
                    if CheckIgnore::es_directorio_a_ignorar(
                        &objeto.obtener_path(),
                        self.logger.clone(),
                    )? {
                        continue;
                    }
                    if let Objeto::Tree(_) = objeto {
                        untrackeados.push(format!("{}/", objeto.obtener_path().display()));
                    } else {
                        untrackeados.push(format!("{}", objeto.obtener_path().display()));
                    };
                }
                return Ok(untrackeados);
            }
        };

        let untrackeados =
            self.obtener_hijos_untrackeados(&self.tree_directorio_actual, tree_head)?;

        Ok(untrackeados)
    }
}

impl Ejecutar for Status {
    /// Ejecuta el comando status.
    /// Devuelve un string con los cambios a ser commiteados, los cambios no en zona de preparacion y los cambios no trackeados.
    fn ejecutar(&mut self) -> Result<String, String> {
        let staging = self.obtener_staging()?;
        let unmergeados = self.obtener_archivos_unmergeados()?;
        let trackeados = self.obtener_trackeados()?;
        let untrackeados = self.obtener_untrackeados()?;

        let mut mensaje = String::new();
        mensaje.push_str("Cambios a ser commiteados:\n");
        for cambio in staging {
            mensaje.push_str(&format!("         {}{}{}\n", VERDE, cambio, RESET));
        }
        mensaje.push_str("\nArchivos unmergeados:\n");
        for cambio in unmergeados {
            mensaje.push_str(&format!("         {}{}{}\n", ROJO, cambio, RESET));
        }
        mensaje.push_str("\nCambios no en zona de preparacion:\n");
        for cambio in trackeados {
            mensaje.push_str(&format!("         {}{}{}\n", ROJO, cambio, RESET));
        }
        mensaje.push_str("\nCambios no trackeados:\n");
        for cambio in untrackeados {
            mensaje.push_str(&format!("         {}{}{}\n", ROJO, cambio, RESET));
        }
        self.logger.log("Status terminado");
        Ok(mensaje)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{io::Write, path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::{add::Add, commit::Commit, init::Init, status::Status},
            logger::Logger,
        },
        utils::io,
    };

    fn addear_archivos(args: Vec<String>, logger: Arc<Logger>) {
        let mut add = Add::from(args, logger.clone()).unwrap();
        add.ejecutar().unwrap();
    }

    fn addear_archivos_y_comittear(args: Vec<String>, logger: Arc<Logger>) {
        let mut add = Add::from(args, logger.clone()).unwrap();
        add.ejecutar().unwrap();
        let mut commit =
            Commit::from(&mut vec!["-m".to_string(), "mensaje".to_string()], logger).unwrap();
        commit.ejecutar().unwrap();
    }

    fn limpiar_archivo_gir() {
        io::rm_directorio(".gir").unwrap();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_init")).unwrap());
        let mut init = Init {
            path: "./.gir".to_string(),
            logger,
        };
        init.ejecutar().unwrap();
    }

    fn crear_test_file() {
        let mut file = std::fs::File::create("test_file.txt").unwrap();
        let _ = file.write_all(b"test file");
    }

    fn modicar_test_file() {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open("test_file.txt")
            .unwrap();
        let _ = file.write_all(b"test file modified");
    }

    fn nombre_esta_en_vector(vector: Vec<String>, nombre: &str) -> bool {
        vector.iter().any(|x| x == nombre)
    }

    #[test]
    #[serial]
    fn test01_obtener_staging() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/status_test01")).unwrap());
        addear_archivos(
            vec![
                "add".to_string(),
                "test_dir/muchos_objetos/archivo.txt".to_string(),
            ],
            logger.clone(),
        );
        let status = Status::from(logger.clone()).unwrap();
        let staging = status.obtener_staging().unwrap();
        assert_eq!(staging.len(), 1);
        assert_eq!(
            staging[0],
            "nuevo archivo: test_dir/muchos_objetos/archivo.txt"
        );
    }

    #[test]
    #[serial]
    fn test02_obtener_staging_con_archivos_multiples() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/status_test02")).unwrap());
        addear_archivos(
            vec!["add".to_string(), "test_dir/".to_string()],
            logger.clone(),
        );

        addear_archivos(
            vec!["add".to_string(), "test_file.txt".to_string()],
            logger.clone(),
        );
        let status = Status::from(logger.clone()).unwrap();
        let staging = status.obtener_staging().unwrap();
        assert_eq!(staging.len(), 4);
        assert_eq!(
            staging[0],
            "nuevo archivo: test_dir/muchos_objetos/archivo.txt"
        );
        assert_eq!(
            staging[1],
            "nuevo archivo: test_dir/muchos_objetos/archivo_copy.txt"
        );
        assert_eq!(staging[2], "nuevo archivo: test_dir/objetos/archivo.txt");
        assert_eq!(staging[3], "nuevo archivo: test_file.txt");
    }

    #[test]
    #[serial]
    fn test03_obtener_trackeados() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/status_test03")).unwrap());
        crear_test_file();
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());
        modicar_test_file();
        let status = Status::from(logger).unwrap();
        let trackeados = status.obtener_trackeados().unwrap();
        assert_eq!(trackeados.len(), 1);
        assert_eq!(trackeados[0], "modificado: test_file.txt");
    }

    #[test]
    #[serial]
    fn test04_obtener_trackeados_con_varios_archivos() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/status_test04")).unwrap());
        crear_test_file();
        addear_archivos(vec!["test_file2.txt".to_string()], logger.clone());
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());
        modicar_test_file();
        io::escribir_bytes("test_file2.txt", "nuevo_mensaje").unwrap();
        let status = Status::from(logger).unwrap();
        let trackeados = status.obtener_trackeados().unwrap();
        let staging = status.obtener_staging().unwrap();
        io::escribir_bytes("test_file2.txt", "test file").unwrap();
        assert_eq!(staging.len(), 0);
        assert_eq!(trackeados.len(), 2);
        assert!(trackeados.contains(&"modificado: test_file.txt".to_string()));
        assert!(trackeados.contains(&"modificado: test_file2.txt".to_string()));
    }

    #[test]
    #[serial]
    fn test05_addear_archivo_lo_elimina_de_untrackeados() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/status_test05")).unwrap());
        addear_archivos(vec!["test_file.txt".to_string()], logger.clone());
        let status = Status::from(logger).unwrap();
        let untrackeados = status.obtener_untrackeados().unwrap();
        assert!(!nombre_esta_en_vector(untrackeados, "test_file.txt"));
    }

    #[test]
    #[serial]
    fn test06_addear_y_luego_modificar_aparece_en_staging_y_trackeados() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/status_test06")).unwrap());
        crear_test_file();
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());
        modicar_test_file();
        addear_archivos(vec!["test_file.txt".to_string()], logger.clone());
        io::escribir_bytes("test_file.txt", "nuevo_mensaje").unwrap();
        let status = Status::from(logger).unwrap();
        let staging = status.obtener_staging().unwrap();
        let trackeados = status.obtener_trackeados().unwrap();
        let untrackeados = status.obtener_untrackeados().unwrap();
        modicar_test_file();
        assert_eq!(staging.len(), 1);
        assert_eq!(trackeados.len(), 1);
        assert_eq!(staging[0], "modificado: test_file.txt");
        assert_eq!(trackeados[0], "modificado: test_file.txt");
        assert!(!nombre_esta_en_vector(untrackeados, "test_file.txt"));
    }
}
