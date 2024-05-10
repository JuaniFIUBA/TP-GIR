use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::tipos_de_dato::logger::Logger;
use crate::tipos_de_dato::objeto::Objeto;
use crate::tipos_de_dato::objetos::tree::Tree;
use crate::utils::compresion::descomprimir_objeto;
use crate::utils::index::{generar_objetos_raiz, leer_index, ObjetoIndex};

/// Dado un hash de un commit y una ubicacion de donde buscar el objeto.
/// Devuelve el hash del arbol de ese commit.
pub fn conseguir_arbol_en_directorio(hash_commit_padre: &str, dir: &str) -> Result<String, String> {
    let contenido = descomprimir_objeto(hash_commit_padre, dir)?;
    let lineas_sin_null = contenido.replace('\0', "\n");
    let lineas = lineas_sin_null.split('\n').collect::<Vec<&str>>();
    let arbol_commit = lineas[1];
    let lineas = arbol_commit.split(' ').collect::<Vec<&str>>();
    let arbol_commit = lineas[1];
    Ok(arbol_commit.to_string())
}

/// Dado un hash de un commit devuelve el hash del arbol de ese commit.
/// Se espera que el contenido del commit tenga el formato correcto.
pub fn conseguir_arbol(hash_commit_padre: &str) -> Result<String, String> {
    conseguir_arbol_en_directorio(hash_commit_padre, ".gir/objects/")
}

/// Devuelve el arbol mergeado entre el arbol padre y los cambios trackeados en el index.
/// Si un archivo esta en el arbol padre se agrega al nuevo arbol.
/// Si un archivo esta en el index se agrega al nuevo arbol, salvo que este en el index por haber sido removido.
/// Si un archivo esta en el arbol padre y en el index, pisa la version anterior y mantiene solo la del index.
fn aplicar_index_a_arbol(arbol_index: &[ObjetoIndex], arbol_padre: &[Objeto]) -> Vec<ObjetoIndex> {
    let mut arbol_mergeado: HashMap<PathBuf, ObjetoIndex> = HashMap::new();

    for objeto_padre in arbol_padre {
        let objeto_index = ObjetoIndex {
            es_eliminado: false,
            merge: false,
            objeto: objeto_padre.clone(),
        };
        arbol_mergeado.insert(objeto_padre.obtener_path(), objeto_index);
    }
    for objeto_index in arbol_index {
        if objeto_index.es_eliminado {
            arbol_mergeado.remove(&objeto_index.objeto.obtener_path());
            continue;
        }
        arbol_mergeado.insert(objeto_index.objeto.obtener_path(), objeto_index.clone());
    }
    arbol_mergeado
        .values()
        .cloned()
        .collect::<Vec<ObjetoIndex>>()
}

/// Crea un arbol de commit a partir del index y su commit padre
/// Commit_padre es un option ya que puede ser None en caso de que sea el primer commit
/// Escribe tanto el arbol como todos sus componentes en .gir/objects.
/// Devuelve el hash del arbol de commit creado.
/// En caso de no haber archivos trackeados devuelve un mensaje y corta la ejecucion.
pub fn crear_arbol_commit(
    commit_padre: Option<String>,
    logger: Arc<Logger>,
) -> Result<String, String> {
    let objetos_index = leer_index(logger.clone())?;
    if objetos_index.is_empty() {
        return Err("No hay archivos trackeados para commitear".to_string());
    }

    let objetos_a_utilizar = if let Some(hash) = commit_padre {
        let hash_arbol_padre = conseguir_arbol_en_directorio(&hash, ".gir/objects/")?;
        let arbol_padre = Tree::from_hash(&hash_arbol_padre, PathBuf::from("./"), logger.clone())?;
        let objetos_arbol_nuevo_commit =
            aplicar_index_a_arbol(&objetos_index, &arbol_padre.objetos);
        generar_objetos_raiz(&objetos_arbol_nuevo_commit, logger.clone())?
    } else {
        generar_objetos_raiz(&objetos_index, logger.clone())?
    };

    let arbol_commit = Tree {
        directorio: PathBuf::from("./"),
        objetos: objetos_a_utilizar,
        logger,
    };

    arbol_commit.escribir_en_base()?;
    arbol_commit.obtener_hash()
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{
            comando::Ejecutar, comandos::add::Add, comandos::init::Init, logger::Logger,
        },
        utils::{compresion::descomprimir_objeto_gir, io},
    };

    use super::crear_arbol_commit;

    fn limpiar_archivo_gir() {
        io::rm_directorio(".gir").unwrap();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_init")).unwrap());
        let mut init = Init {
            path: "./.gir".to_string(),
            logger,
        };
        init.ejecutar().unwrap();
    }

    #[test]
    #[serial]

    fn test01_se_escribe_arbol_con_un_hijo() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test01")).unwrap());
        Add::from(vec!["test_file.txt".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        let arbol_commit = crear_arbol_commit(None, logger.clone()).unwrap();
        let contenido_commit = descomprimir_objeto_gir(&arbol_commit).unwrap();

        assert_eq!(
            contenido_commit,
            "tree 41\0100644 test_file.txt\0678e12dc5c03a7cf6e9f64e688868962ab5d8b65"
        );
    }

    #[test]
    #[serial]
    fn test02_se_escribe_arbol_con_carpeta() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test01")).unwrap());
        Add::from(vec!["test_dir/objetos".to_string()], logger.clone())
            .unwrap()
            .ejecutar()
            .unwrap();

        let arbol_commit = crear_arbol_commit(None, logger.clone()).unwrap();

        assert_eq!(arbol_commit, "01c6c27fe31e9a4c3e64d3ab3489a2d3716a2b49");

        let contenido_commit = descomprimir_objeto_gir(&arbol_commit).unwrap();

        assert_eq!(
            contenido_commit,
            "tree 35\040000 test_dir\01f67151c34d6b33ec1a98fdafef8b021068395a0"
        );
    }
}
