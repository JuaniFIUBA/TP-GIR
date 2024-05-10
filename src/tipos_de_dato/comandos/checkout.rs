use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    tipos_de_dato::{
        comando::Ejecutar, comandos::branch::Branch, config::Config, info_ramas::RamasInfo,
        logger::Logger, objeto::Objeto, objetos::tree::Tree, tipo_de_rama::TipoRama,
    },
    utils::{self, io},
};

use super::{show_ref::ShowRef, write_tree::conseguir_arbol_en_directorio};

const PATH_HEAD: &str = "./.gir/HEAD";

pub struct Checkout {
    /// Si es true, se crea una nueva rama.
    crear_rama: bool,
    /// Nombre de la rama a cambiar.
    rama_a_cambiar: String,
    /// Logger para imprimir mensajes en un archivo log.
    logger: Arc<Logger>,
}

impl Checkout {
    /// Verifica si hay flags en los argumentos.
    fn hay_flags(args: &Vec<String>) -> bool {
        args.len() != 1
    }

    /// Verifica si la cantidad de argumentos son validos para el comando checkout.
    fn verificar_argumentos(args: &Vec<String>) -> Result<(), String> {
        if args.len() > 2 {
            return Err(
                "Argumentos desconocidos.\ngir checkcout [-b] <nombre-rama-cambiar>".to_string(),
            );
        }
        Ok(())
    }

    /// Crea una instancia de Checkout setteada para crear la branch.
    /// Si no se puede crear devuelve un error.
    fn crearse_con_flags(args: Vec<String>, logger: Arc<Logger>) -> Result<Checkout, String> {
        match (args[0].as_str(), args[1].clone()) {
            ("-b", rama) => Ok(Checkout {
                crear_rama: true,
                rama_a_cambiar: rama,
                logger,
            }),
            _ => Err("Argumentos invalidos.\ngir chekcout [-b] <nombre-rama-cambiar>".to_string()),
        }
    }

    /// Crea la instancia de checkout pertinente a los argumentos enviados.
    pub fn from(args: Vec<String>, logger: Arc<Logger>) -> Result<Checkout, String> {
        Self::verificar_argumentos(&args)?;

        if Self::hay_flags(&args) {
            return Self::crearse_con_flags(args, logger);
        }

        Ok(Checkout {
            crear_rama: false,
            rama_a_cambiar: args[0].to_string(),
            logger,
        })
    }

    /// Devuelve un vector con los nombres de las ramas existentes en el repositorio.
    pub fn obtener_ramas() -> Result<Vec<String>, String> {
        let directorio = ".gir/refs/heads";
        let entradas = std::fs::read_dir(directorio)
            .map_err(|e| format!("No se pudo leer el directorio:{}\n {}", directorio, e))?;

        let mut output = Vec::new();

        for entrada in entradas {
            let entrada = entrada
                .map_err(|_| format!("Error al leer entrada el directorio {directorio:#?}"))?;

            let nombre = utils::path_buf::obtener_nombre(&entrada.path())?;
            output.push(nombre)
        }

        Ok(output)
    }

    /// Devuelve un hashmap con las ramas remotas.
    pub fn obtener_ramas_remotas(&self) -> Result<HashMap<String, String>, String> {
        let show_ref = ShowRef::from(vec![], self.logger.clone())?;
        let ramas = show_ref.obtener_referencias(PathBuf::from(".gir/refs/remotes"))?;
        Ok(ramas)
    }
    /// Verifica si la rama a cambiar ya existe.
    fn verificar_si_la_rama_existe(&self) -> Result<TipoRama, String> {
        let ramas = Self::obtener_ramas()?;

        if ramas.contains(&self.rama_a_cambiar) {
            return Ok(TipoRama::Local);
        }

        // key: refs/remotes/<remote>/<branch>
        // value: <commit>
        let ramas_remotas = self.obtener_ramas_remotas()?;

        for (ruta, commit) in ramas_remotas {
            if ruta.ends_with(&self.rama_a_cambiar) {
                return Ok(TipoRama::Remota(ruta, commit));
            }
        }

        Err(format!("Fallo: No existe la rama {}", self.rama_a_cambiar))
    }

    /// Devuelve el nombre de la rama actual.
    /// O sea, la rama a la que apunta el archivo HEAD.
    fn conseguir_rama_actual(contenidio_head: &str) -> Result<String, String> {
        let partes: Vec<&str> = contenidio_head.split('/').collect();
        let rama_actual = partes
            .last()
            .ok_or_else(|| "Fallo en la lectura de HEAD".to_string())?
            .trim();
        Ok(rama_actual.to_string())
    }

    /// Cambia la referencia de la rama en el archivo HEAD.
    fn cambiar_ref_en_head(&self) -> Result<(), String> {
        let contenido_head = io::leer_a_string(PATH_HEAD)?;

        let rama_actual = Self::conseguir_rama_actual(&contenido_head)?;

        let nuevo_head = contenido_head.replace(&rama_actual, &self.rama_a_cambiar);

        io::escribir_bytes(PATH_HEAD, nuevo_head)?;

        Ok(())
    }

    /// Crea una nueva rama desde el remote.
    fn crear_rama_desde_remote(&self, commit: &str) -> Result<(), String> {
        io::escribir_bytes(format!(".gir/refs/heads/{}", self.rama_a_cambiar), commit)
    }

    /// Configura el remote para la rama actual.
    /// O sea, agrega la rama actual al archivo de configuracion.
    fn configurar_remoto_para_rama_actual(&self, ruta_remoto: &str) -> Result<(), String> {
        let mut config = Config::leer_config()?;
        let rama = RamasInfo {
            nombre: self.rama_a_cambiar.clone(),
            remote: ruta_remoto.split('/').last().unwrap().to_string(),
            merge: PathBuf::from(format!("refs/heads/{}", self.rama_a_cambiar)),
        };

        config.ramas.push(rama);
        config.guardar_config()?;

        Ok(())
    }

    /// Cambia la rama actual.
    /// Si la rama es remota, se crea una nueva rama local y se configura el remote.
    /// Si la rama es local, se cambia la referencia en el archivo HEAD.
    fn cambiar_rama(&self) -> Result<String, String> {
        match self.verificar_si_la_rama_existe()? {
            TipoRama::Remota(ruta, commit) => {
                self.crear_rama_desde_remote(&commit)?;
                self.configurar_remoto_para_rama_actual(&ruta)?
            }
            TipoRama::Local => {}
        };

        self.cambiar_ref_en_head()?;
        let msg = format!("Se cambio la rama actual a {}", self.rama_a_cambiar);
        self.logger.log(&msg);

        Ok(msg)
    }

    /// Crea una nueva rama con el nombre especificado.
    fn crear_rama(&self) -> Result<(), String> {
        let msg_branch = Branch::from(&mut vec![self.rama_a_cambiar.clone()], self.logger.clone())?
            .ejecutar()?;
        println!("{}", msg_branch);
        Ok(())
    }

    /// Verifica que el index no tenga contenido antes de cambiarse rama.
    fn comprobar_que_no_haya_contenido_index(&self) -> Result<(), String> {
        if !utils::index::esta_vacio_el_index()? {
            Err("Fallo, tiene contendio sin guardar. Por favor, haga commit para no perder los cambios".to_string())
        } else {
            Ok(())
        }
    }

    /// Devuelve el arbol del ultimo commit de la rama actual.
    pub fn obtener_arbol_commit_actual(logger: Arc<Logger>) -> Result<Tree, String> {
        let ref_actual = io::leer_a_string(PATH_HEAD)?;
        let rama_actual = Self::conseguir_rama_actual(&ref_actual)?;
        let head_commit = io::leer_a_string(format!(".gir/refs/heads/{}", rama_actual))?;
        let hash_tree_padre = conseguir_arbol_en_directorio(&head_commit, ".gir/objects/")?;
        Tree::from_hash(&hash_tree_padre, PathBuf::from("."), logger)
    }

    /// Elimina los archivos correspondientes a cada objeto que no se encuentre en el arbol futuro.
    fn eliminar_objetos(&self, objetos: &Vec<Objeto>) -> Result<(), String> {
        for objeto in objetos {
            match objeto {
                Objeto::Blob(blob) => {
                    io::rm_directorio(blob.ubicacion.clone())?;
                }
                Objeto::Tree(tree) => {
                    io::rm_directorio(tree.directorio.clone())?;
                }
            }
        }
        Ok(())
    }

    /// Devuelve un vector con los objetos que estaban en el tree viejo pero no en el nuevo.
    /// O sea, los objetos que se eliminaron.
    fn obtener_objetos_eliminados(tree_viejo: &Tree, tree_nuevo: &Tree) -> Vec<Objeto> {
        let mut objetos_eliminados: Vec<Objeto> = Vec::new();

        for objeto_viejo in tree_viejo.objetos.iter() {
            match objeto_viejo {
                Objeto::Blob(blob) => {
                    if !tree_nuevo.contiene_misma_version_hijo(&blob.hash, &blob.ubicacion) {
                        objetos_eliminados.push(objeto_viejo.clone());
                    }
                }
                Objeto::Tree(tree) => {
                    let mut hijos_eliminados = Self::obtener_objetos_eliminados(tree, tree_nuevo);
                    objetos_eliminados.append(&mut hijos_eliminados);
                }
            }
        }

        objetos_eliminados
    }
}

impl Ejecutar for Checkout {
    /// Ejecuta el comando checkout en su totalidad.
    /// Si se crea una nueva rama, se crea y se cambia a ella.
    /// Si se cambia de rama, se cambia y se actualiza el contenido.
    fn ejecutar(&mut self) -> Result<String, String> {
        self.comprobar_que_no_haya_contenido_index()?;

        if self.crear_rama {
            self.crear_rama()?;
            self.cambiar_rama()?;
            return Ok(format!("Cambiado a nueva rama {}", self.rama_a_cambiar));
        };

        if !self.crear_rama {
            let tree_viejo = Self::obtener_arbol_commit_actual(self.logger.clone())?;
            self.cambiar_rama()?;
            let tree_futuro = Self::obtener_arbol_commit_actual(self.logger.clone())?;
            let objetos_a_eliminar = Self::obtener_objetos_eliminados(&tree_viejo, &tree_futuro);
            self.eliminar_objetos(&objetos_a_eliminar)?;
            tree_futuro.escribir_en_directorio()?;
        };
        Ok(format!("Cambiado a rama {}", self.rama_a_cambiar))
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::branch::Branch,
            logger::Logger,
            objeto::Objeto,
            objetos::{blob::Blob, tree::Tree},
        },
        utils::{
            io,
            testing::{addear_archivos_y_comittear, limpiar_archivo_gir},
        },
    };

    use super::*;

    fn craer_archivo_config_default() {
        let home = std::env::var("HOME").unwrap();
        let config_path = format!("{home}/.girconfig");
        let contenido = "nombre = ejemplo_nombre\nmail = ejemplo_mail\n".to_string();
        io::escribir_bytes(config_path, contenido).unwrap();
    }

    fn tree_con_un_tree_y_un_objeto(logger: Arc<Logger>) -> Tree {
        let objeto_nieto = Objeto::Blob(Blob {
            hash: "hash_nieto".to_string(),
            ubicacion: PathBuf::from("./tree_hijo/nieto"),
            logger: logger.clone(),
            nombre: "nieto".to_string(),
        });
        let objeto_hijo = Objeto::Blob(Blob {
            hash: "hash_hijo".to_string(),
            ubicacion: PathBuf::from("./hijo"),
            logger: logger.clone(),
            nombre: "hijo".to_string(),
        });

        let un_tree_hijo = Objeto::Tree(Tree {
            directorio: PathBuf::from("./tree_hijo"),
            objetos: vec![objeto_nieto],
            logger: logger.clone(),
        });

        Tree {
            directorio: PathBuf::from("."),
            objetos: vec![un_tree_hijo, objeto_hijo],
            logger: logger.clone(),
        }
    }

    fn tree_con_un_objeto(logger: Arc<Logger>) -> Tree {
        let objeto_hijo = Objeto::Blob(Blob {
            hash: "hash_hijo".to_string(),
            ubicacion: PathBuf::from("./hijo"),
            logger: logger.clone(),
            nombre: "hijo".to_string(),
        });

        Tree {
            directorio: PathBuf::from("."),
            objetos: vec![objeto_hijo],
            logger: logger.clone(),
        }
    }

    #[test]
    #[serial]
    fn test01_checkout_cambia_de_rama() {
        craer_archivo_config_default();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/checkout_test02")).unwrap());
        limpiar_archivo_gir(logger.clone());
        let args = vec!["test_dir/objetos/archivo.txt".to_string()];
        addear_archivos_y_comittear(args, logger.clone());

        Branch::from(&mut vec!["una_rama".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        let mut checkout = Checkout::from(vec!["una_rama".to_string()], logger.clone()).unwrap();
        checkout.ejecutar().unwrap();

        let contenido_head = std::fs::read_to_string(".gir/HEAD").unwrap();
        assert_eq!(contenido_head, "ref: refs/heads/una_rama".to_string());
    }

    #[test]
    #[serial]
    fn test02_checkout_crea_y_cambia_de_rama() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/checkout_test02")).unwrap());
        limpiar_archivo_gir(logger.clone());
        let args = vec!["test_dir/objetos/archivo.txt".to_string()];
        addear_archivos_y_comittear(args, logger.clone());

        let mut checkout = Checkout::from(
            vec!["-b".to_string(), "una_rama".to_string()],
            logger.clone(),
        )
        .unwrap();
        checkout.ejecutar().unwrap();

        let contenido_head = std::fs::read_to_string(".gir/HEAD").unwrap();
        assert_eq!(contenido_head, "ref: refs/heads/una_rama".to_string());
    }

    #[test]
    #[serial]
    fn test03_al_hacer_checkout_actualiza_contenido() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/checkout_test03")).unwrap());
        limpiar_archivo_gir(logger.clone());
        io::escribir_bytes("tmp/checkout_test03_test", "contenido").unwrap();
        let args = vec!["tmp/checkout_test03_test".to_string()];
        addear_archivos_y_comittear(args, logger.clone());

        let mut checkout = Checkout::from(
            vec!["-b".to_string(), "una_rama".to_string()],
            logger.clone(),
        )
        .unwrap();
        checkout.ejecutar().unwrap();

        io::escribir_bytes("tmp/checkout_test03_test", "contenido 2").unwrap();
        let args = vec!["tmp/checkout_test03_test".to_string()];
        addear_archivos_y_comittear(args, logger.clone());

        let mut checkout = Checkout::from(vec!["master".to_string()], logger.clone()).unwrap();
        checkout.ejecutar().unwrap();

        let contenido_archivo = io::leer_a_string("tmp/checkout_test03_test").unwrap();

        assert_eq!(contenido_archivo, "contenido".to_string());
    }

    #[test]
    #[serial]
    fn test04_al_hacer_checkout_se_eliminan_no_trackeados() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/checkout_test03")).unwrap());
        limpiar_archivo_gir(logger.clone());
        io::escribir_bytes("tmp/checkout_test04_test", "contenido").unwrap();
        let args = vec!["tmp/checkout_test04_test".to_string()];
        addear_archivos_y_comittear(args, logger.clone());

        let mut checkout = Checkout::from(
            vec!["-b".to_string(), "una_rama".to_string()],
            logger.clone(),
        )
        .unwrap();
        checkout.ejecutar().unwrap();

        io::escribir_bytes("tmp/checkout_test04_test_2", "contenido 2").unwrap();
        let args = vec!["tmp/checkout_test04_test_2".to_string()];
        addear_archivos_y_comittear(args, logger.clone());

        let mut checkout = Checkout::from(vec!["master".to_string()], logger.clone()).unwrap();
        checkout.ejecutar().unwrap();

        assert!(!PathBuf::from("tmp/checkout_test04_test_2").exists());
        assert!(PathBuf::from("tmp/checkout_test04_test").exists());
    }

    #[test]
    #[serial]
    fn test05_obtener_objetos_eliminados() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/checkout_test04")).unwrap());

        let tree_viejo = tree_con_un_tree_y_un_objeto(logger.clone());
        let tree_nuevo = tree_con_un_objeto(logger.clone());

        let objetos_eliminados = Checkout::obtener_objetos_eliminados(&tree_viejo, &tree_nuevo);

        assert_eq!(objetos_eliminados.len(), 1);

        if let Objeto::Blob(blob) = &objetos_eliminados[0] {
            assert_eq!(blob.nombre, "nieto".to_string());
        } else {
            unreachable!();
        }
    }

    #[test]
    #[serial]
    fn test06_se_puede_checkoutear_a_una_rama_remota() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/checkout_test06")).unwrap());
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());
        let last_hash = io::leer_a_string(".gir/refs/heads/master").unwrap();
        io::crear_carpeta(".gir/refs/remotes/origin").unwrap();
        io::crear_archivo(".gir/refs/remotes/origin/remota").unwrap();
        io::escribir_bytes(".gir/refs/remotes/origin/remota", last_hash.clone()).unwrap();

        let mut checkout = Checkout::from(vec!["remota".to_string()], logger.clone()).unwrap();
        checkout.ejecutar().unwrap();

        let contenido_head = std::fs::read_to_string(".gir/HEAD").unwrap();
        assert_eq!(contenido_head, "ref: refs/heads/remota".to_string());
        assert_eq!(
            io::leer_a_string(".gir/refs/heads/remota").unwrap(),
            last_hash
        );
        assert!(PathBuf::from("test_file.txt").exists());
    }
}
