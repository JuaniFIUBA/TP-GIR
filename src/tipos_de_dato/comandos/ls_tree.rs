use std::{path::PathBuf, sync::Arc};

use crate::tipos_de_dato::{
    comando::Ejecutar, logger::Logger, objeto::Objeto, objetos::tree::Tree,
    visualizaciones::Visualizaciones,
};

use super::cat_file::CatFile;

pub struct LsTree {
    /// Logger para imprimir los mensajes en el archivo log.
    logger: Arc<Logger>,
    /// Indica si se debe mostrar el contenido de los subdirectorios.
    recursivo: bool,
    /// Indica si se deben mostrar solo los arboles.
    solo_arboles: bool,
    /// Indica si se debe mostrar el tamaño de los archivos.
    con_size: bool,
    /// Hash del arbol a mostrar.
    arbol: String,
}

impl LsTree {
    /// Devuelve un LsTree con los parametros ingresados por el usuario.
    /// Si no se ingresa el hash del arbol correctamente, devuelve un error.
    pub fn from(logger: Arc<Logger>, args: &mut Vec<String>) -> Result<LsTree, String> {
        let hash_arbol = args.pop().ok_or("No se pudo obtener el hash del arbol")?;
        if hash_arbol.len() != 40 {
            return Err(format!("El hash del arbol no es valido: {}", hash_arbol));
        }
        let mut recursivo = false;
        let mut solo_arboles = false;
        let mut con_size = false;

        for arg in args {
            match arg.as_str() {
                "-r" => recursivo = true,
                "-d" => solo_arboles = true,
                "-l" => con_size = true,
                _ => {
                    return Err(format!("Argumento no valido: {}", arg));
                }
            }
        }
        Ok(LsTree {
            logger,
            recursivo,
            solo_arboles,
            con_size,
            arbol: hash_arbol,
        })
    }

    /// Dado un objeto blob, devuelve un string con el formato de salida de ls-tree.
    fn obtener_string_blob(blob: &Objeto) -> String {
        format!(
            "100644 blob {}    {}\n",
            blob.obtener_hash(),
            blob.obtener_path().display()
        )
    }

    /// Dado un arbol, devuelve un vector con los objetos que se deben mostrar.
    /// Si el arbol es recursivo, se muestran todos los objetos hoja.
    /// Si el arbol es recursivo y solo_arboles es true, se muestran todos los arboles.
    /// Si el arbol no es recursivo, se muestran todos los objetos en el segundo nivel del arbol.
    fn obtener_objetos_a_mostrar(&self, arbol: &Tree) -> Vec<Objeto> {
        let mut objetos_a_mostrar = Vec::new();
        if self.recursivo && self.solo_arboles {
            let hijos_totales = arbol.obtener_objetos();
            for hijo in hijos_totales {
                if let Objeto::Tree(tree) = hijo {
                    objetos_a_mostrar.push(Objeto::Tree(tree));
                }
            }
        } else if self.recursivo {
            objetos_a_mostrar = arbol.obtener_objetos_hoja();
        } else if self.solo_arboles {
            let hijos_arbol = arbol.objetos.clone();
            for hijo in hijos_arbol {
                if let Objeto::Tree(tree) = hijo {
                    objetos_a_mostrar.push(Objeto::Tree(tree));
                }
            }
        } else {
            objetos_a_mostrar = arbol.objetos.clone();
        }
        Tree::ordenar_objetos_alfabeticamente(&objetos_a_mostrar)
    }
}

impl Ejecutar for LsTree {
    /// Ejecuta el comando ls-tree.
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Corriendo ls-tree");
        let arbol = Tree::from_hash(&self.arbol, PathBuf::from("."), self.logger.clone())?;
        let objetos_a_mostrar = self.obtener_objetos_a_mostrar(&arbol);

        let mut string_resultante = String::new();
        for objeto in objetos_a_mostrar {
            match objeto {
                Objeto::Blob(ref blob) => {
                    if self.con_size {
                        let tamanio = CatFile {
                            hash_objeto: blob.obtener_hash(),
                            logger: self.logger.clone(),
                            visualizacion: Visualizaciones::Tamanio,
                        }
                        .ejecutar()?;

                        string_resultante.push_str(&format!(
                            "100644 blob {} {: >7}    {}\n",
                            blob.obtener_hash(),
                            tamanio,
                            blob.ubicacion.display()
                        ));
                    } else {
                        string_resultante.push_str(&Self::obtener_string_blob(&objeto));
                    }
                }
                Objeto::Tree(ref tree) => {
                    if self.con_size {
                        string_resultante.push_str(&format!(
                            "040000 tree {}       -    {}\n",
                            tree.obtener_hash()?,
                            tree.directorio.display()
                        ));
                    } else {
                        string_resultante.push_str(&format!(
                            "040000 tree {}    {}\n",
                            tree.obtener_hash()?,
                            tree.directorio.display()
                        ));
                    }
                }
            }
        }
        Ok(string_resultante)
    }
}

#[cfg(test)]

mod tests {
    use std::{path::PathBuf, sync::Arc};

    use serial_test::serial;

    use super::*;
    use crate::{
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::{add::Add, commit::Commit, init::Init, write_tree},
            logger::Logger,
        },
        utils::{io, ramas},
    };

    fn add_y_commit_anindado(logger: Arc<Logger>) {
        let mut add = Add::from(
            vec!["test_dir/objetos/archivo.txt".to_string()],
            logger.clone(),
        )
        .unwrap();
        add.ejecutar().unwrap();
        let mut commit = Commit::from(
            &mut vec!["-m".to_string(), "mensaje".to_string()],
            logger.clone(),
        )
        .unwrap();
        commit.ejecutar().unwrap();
    }

    fn add_y_commit_en_root(logger: Arc<Logger>) {
        let mut add = Add::from(vec!["test_file.txt".to_string()], logger.clone()).unwrap();
        add.ejecutar().unwrap();
        let mut commit = Commit::from(
            &mut vec!["-m".to_string(), "mensaje".to_string()],
            logger.clone(),
        )
        .unwrap();
        commit.ejecutar().unwrap();
    }

    fn limpiar_archivo_gir(logger: Arc<Logger>) {
        if PathBuf::from("./.gir").exists() {
            io::rm_directorio(".gir").unwrap();
        }

        let mut init = Init {
            path: "./.gir".to_string(),
            logger,
        };
        init.ejecutar().unwrap();
    }

    #[test]
    #[serial]

    fn test01_sin_flags() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/ls_tree_test01")).unwrap());
        limpiar_archivo_gir(logger.clone());
        add_y_commit_anindado(logger.clone());

        let hash_commit = ramas::obtener_hash_commit_asociado_rama_actual().unwrap();
        let arbol = write_tree::conseguir_arbol(&hash_commit).unwrap();
        let mut ls_tree = LsTree::from(logger.clone(), &mut vec![arbol]).unwrap();
        let resultado = ls_tree.ejecutar().unwrap();
        assert_eq!(
            resultado,
            "040000 tree 1f67151c34d6b33ec1a98fdafef8b021068395a0    test_dir\n"
        );
    }

    #[test]
    #[serial]

    fn test02_recursivo() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/ls_tree_test02")).unwrap());
        limpiar_archivo_gir(logger.clone());
        add_y_commit_anindado(logger.clone());

        let hash_commit = ramas::obtener_hash_commit_asociado_rama_actual().unwrap();
        let arbol = write_tree::conseguir_arbol(&hash_commit).unwrap();
        let mut ls_tree =
            LsTree::from(logger.clone(), &mut vec!["-r".to_string(), arbol.clone()]).unwrap();
        let resultado = ls_tree.ejecutar().unwrap();
        assert_eq!(
            resultado,
            "100644 blob 2b824e648965b94c6c6b3dd0702feb91f699ed62    test_dir/objetos/archivo.txt\n"
        );
    }

    #[test]
    #[serial]

    fn test03_arboles() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/ls_tree_test01")).unwrap());
        limpiar_archivo_gir(logger.clone());
        add_y_commit_anindado(logger.clone());
        add_y_commit_en_root(logger.clone());

        let hash_commit = ramas::obtener_hash_commit_asociado_rama_actual().unwrap();
        let arbol = write_tree::conseguir_arbol(&hash_commit).unwrap();
        let mut ls_tree = LsTree::from(logger.clone(), &mut vec!["-d".to_string(), arbol]).unwrap();
        let resultado = ls_tree.ejecutar().unwrap();
        assert_eq!(
            resultado,
            "040000 tree 1f67151c34d6b33ec1a98fdafef8b021068395a0    test_dir\n"
        );
    }

    #[test]
    #[serial]
    fn test03_tamanio() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/ls_tree_test01")).unwrap());
        limpiar_archivo_gir(logger.clone());
        add_y_commit_en_root(logger.clone());

        let hash_commit = ramas::obtener_hash_commit_asociado_rama_actual().unwrap();
        let arbol = write_tree::conseguir_arbol(&hash_commit).unwrap();
        let mut ls_tree = LsTree::from(logger.clone(), &mut vec!["-l".to_string(), arbol]).unwrap();
        let resultado = ls_tree.ejecutar().unwrap();
        assert_eq!(
            resultado,
            "100644 blob 678e12dc5c03a7cf6e9f64e688868962ab5d8b65      18    test_file.txt\n"
        );
    }

    #[test]
    #[serial]
    fn test04_tamanio_arbol() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/ls_tree_test01")).unwrap());
        limpiar_archivo_gir(logger.clone());
        add_y_commit_anindado(logger.clone());

        let hash_commit = ramas::obtener_hash_commit_asociado_rama_actual().unwrap();
        let arbol = write_tree::conseguir_arbol(&hash_commit).unwrap();
        let mut ls_tree = LsTree::from(logger.clone(), &mut vec!["-l".to_string(), arbol]).unwrap();
        let resultado = ls_tree.ejecutar().unwrap();
        assert_eq!(
            resultado,
            "040000 tree 1f67151c34d6b33ec1a98fdafef8b021068395a0       -    test_dir\n"
        );
    }
}
