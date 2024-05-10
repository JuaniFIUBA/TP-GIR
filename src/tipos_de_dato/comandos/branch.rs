use std::{path::PathBuf, sync::Arc};

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger},
    utils::{io, path_buf::obtener_nombre, ramas},
};

const VERDE: &str = "\x1B[32m";
const RESET: &str = "\x1B[0m";

pub struct Branch {
    pub mostrar: bool,
    pub rama_nueva: Option<String>,
    pub logger: Arc<Logger>,
}

impl Branch {
    /// Devuelve un Branch con los parametros ingresados por el usuario.
    /// Si no se ingresa el nombre de la rama, devuelve un Branch con mostrar en true.
    /// Si se ingresa el nombre de la rama, devuelve un Branch con mostrar en false.
    pub fn from(args: &mut Vec<String>, logger: Arc<Logger>) -> Result<Branch, String> {
        if args.is_empty() {
            return Ok(Branch {
                mostrar: true,
                rama_nueva: None,
                logger,
            });
        }

        if args.len() > 1 {
            return Err("Demasiados argumentos\ngir branch [<nombre-rama-nueva>]".to_string());
        }

        let arg = args
            .pop()
            .ok_or(format!("No se pudo obtener el argumento de {:?}", args))?;

        Ok(Branch {
            mostrar: false,
            rama_nueva: Some(arg.to_string()),
            logger,
        })
    }

    /// Devuelve un vector con las ramas que existen en el repositorio
    pub fn obtener_ramas() -> Result<Vec<String>, String> {
        let directorio = ".gir/refs/heads";
        let entradas = std::fs::read_dir(directorio)
            .map_err(|e| format!("No se pudo leer el directorio:{}\n {}", directorio, e))?;

        let mut ramas: Vec<String> = Vec::new();

        for entrada in entradas {
            let entrada = entrada
                .map_err(|_| format!("Error al leer entrada el directorio {directorio:#?}"))?;
            let nombre = obtener_nombre(&entrada.path())?;
            ramas.push(nombre);
        }
        ramas.sort();
        Ok(ramas)
    }

    ///  Devuelve un string con la lista de ramas en el repo,
    /// marcando con un * y verde la rama actual
    pub fn mostrar_ramas() -> Result<String, String> {
        let rama_actual = ramas::obtener_rama_actual()?;

        let mut output = String::new();

        for rama in Self::obtener_ramas()? {
            if rama == rama_actual {
                output.push_str(&format!("* {}{}{}\n", VERDE, rama, RESET));
            } else {
                output.push_str(&format!("  {}\n", rama));
            }
        }

        Ok(output)
    }

    /// Crea una rama nueva con el nombre ingresado por el usuario.
    /// Si la rama ya existe, devuelve un error.
    /// Si no se pudo obtener el nombre de la rama, devuelve un error.
    /// Utiliza como base el hash del commit que apunta HEAD.
    fn crear_rama(&mut self) -> Result<String, String> {
        let rama_nueva = self
            .rama_nueva
            .take()
            .ok_or("No se pudo obtener el nombre de la rama")?;

        let direccion_rama_nueva = format!(".gir/refs/heads/{}", rama_nueva);

        if PathBuf::from(&direccion_rama_nueva).exists() {
            return Err(format!("La rama {} ya existe", rama_nueva));
        }
        let ultimo_commit = ramas::obtener_hash_commit_asociado_rama_actual()?;
        io::escribir_bytes(direccion_rama_nueva, ultimo_commit)?;
        Ok(format!("Se creó la rama {}", rama_nueva))
    }
}

impl Ejecutar for Branch {
    fn ejecutar(&mut self) -> Result<String, String> {
        if self.mostrar {
            return Self::mostrar_ramas();
        }
        self.crear_rama()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tipos_de_dato::comando::Ejecutar;
    use crate::tipos_de_dato::comandos::add::Add;
    use crate::tipos_de_dato::comandos::commit::Commit;
    use crate::tipos_de_dato::comandos::init::Init;
    use crate::tipos_de_dato::logger::Logger;
    use crate::utils;
    use crate::utils::gir_config::obtener_gir_config_path;
    use serial_test::serial;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn craer_archivo_config_default() {
        let config_path = obtener_gir_config_path().unwrap();
        let contenido = "nombre =aaaa\nmail =bbbb\n".to_string();
        std::fs::write(config_path, contenido).unwrap();
    }

    fn limpiar_archivo_gir() {
        if PathBuf::from("./.gir").exists() {
            io::rm_directorio(".gir").unwrap();
        }

        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_init")).unwrap());
        let mut init = Init {
            path: "./.gir".to_string(),
            logger,
        };
        init.ejecutar().unwrap();
        craer_archivo_config_default();
    }

    fn conseguir_arbol_commit(branch: &str) -> Result<String, String> {
        let hash_hijo = io::leer_a_string(format!(".gir/refs/heads/{}", branch))?;
        let contenido_hijo = utils::compresion::descomprimir_objeto_gir(&hash_hijo)?;
        let lineas_sin_null = contenido_hijo.replace('\0', "\n");
        let lineas = lineas_sin_null.split('\n').collect::<Vec<&str>>();
        let arbol_commit = lineas[1];
        let lineas = arbol_commit.split(' ').collect::<Vec<&str>>();
        let arbol_commit = lineas[1];
        Ok(arbol_commit.to_string())
    }

    fn addear_archivos_y_comittear(args: Vec<String>, logger: Arc<Logger>) {
        let mut add = Add::from(args, logger.clone()).unwrap();
        add.ejecutar().unwrap();
        let mut commit =
            Commit::from(&mut vec!["-m".to_string(), "mensaje".to_string()], logger).unwrap();
        commit.ejecutar().unwrap();
    }

    #[test]
    #[serial]
    fn test01_mostrar_ramas() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_test01")).unwrap());
        let mut branch = Branch {
            mostrar: true,
            rama_nueva: None,
            logger,
        };

        let resultado = branch.ejecutar();

        assert_eq!(resultado, Ok(format!("* {VERDE}master{RESET}\n")));
    }

    #[test]
    #[serial]
    fn test02_crear_rama() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_test02")).unwrap());
        let mut branch = Branch {
            mostrar: false,
            rama_nueva: Some("nueva_rama".to_string()),
            logger,
        };

        let resultado = branch.ejecutar();

        assert_eq!(resultado, Ok("Se creó la rama nueva_rama".to_string()));
    }

    #[test]
    #[serial]
    fn test03_crear_una_rama_y_mostrar_ramas() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_test03")).unwrap());
        Branch {
            mostrar: false,
            rama_nueva: Some("nueva_rama".to_string()),
            logger: logger.clone(),
        }
        .ejecutar()
        .unwrap();

        let resultado = Branch {
            mostrar: true,
            rama_nueva: None,
            logger,
        }
        .ejecutar();

        assert_eq!(
            resultado,
            Ok(format!("* {VERDE}master{RESET}\n  nueva_rama\n"))
        );
    }

    #[test]
    #[serial]
    fn test04_mostrar_from() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_test04")).unwrap());
        let mut branch = Branch::from(&mut vec![], logger).unwrap();

        let resultado = branch.ejecutar();

        assert_eq!(resultado, Ok(format!("* {VERDE}master{RESET}\n")));
    }

    #[test]
    #[serial]
    fn test05_crear_from() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_test05")).unwrap());
        let mut branch = Branch::from(&mut vec!["nueva_rama".to_string()], logger).unwrap();

        let resultado = branch.ejecutar();

        assert_eq!(resultado, Ok("Se creó la rama nueva_rama".to_string()));
    }

    #[test]
    #[serial]
    #[should_panic(expected = "Demasiados argumentos\\ngir branch [<nombre-rama-nueva>]")]
    fn test06_muchos_argumentos() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_test06")).unwrap());
        let mut branch = Branch::from(
            &mut vec!["nueva_rama".to_string(), "otra_nueva_rama".to_string()],
            logger,
        )
        .unwrap();

        branch.ejecutar().unwrap();
    }

    #[test]
    #[serial]
    fn test07_la_branch_se_crea_apuntando_al_ultimo_commit() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/branch_test07")).unwrap());
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());
        let mut branch = Branch {
            mostrar: false,
            rama_nueva: Some("nueva_rama".to_string()),
            logger: logger.clone(),
        };
        branch.ejecutar().unwrap();

        let hash_arbol = conseguir_arbol_commit("nueva_rama");
        let hash_arbol_git = "ce0ef9a25817847d31d12df1295248d24d07b309".to_string();

        assert_eq!(hash_arbol, Ok(hash_arbol_git));
    }
}
