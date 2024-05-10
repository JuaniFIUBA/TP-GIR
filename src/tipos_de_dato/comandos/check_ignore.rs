use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger},
    utils::{self, io, path_buf::esta_directorio_habilitado},
};

pub struct CheckIgnore {
    /// Logger para imprimir los mensajes en el archivo log.
    logger: Arc<Logger>,
    /// Paths de los archivos a verificar si son ignorados.
    paths: Vec<String>,
}

impl CheckIgnore {
    /// Dada una ubicacion de un directorio/archivo, devuele un booleano indicando
    /// si la ubicacion esta dentro de los archivos a ignorar.
    pub fn es_directorio_a_ignorar(ubicacion: &Path, logger: Arc<Logger>) -> Result<bool, String> {
        if esta_directorio_habilitado(ubicacion, &vec![PathBuf::from(".gir")]) {
            return Ok(true);
        }

        let path = ubicacion
            .to_str()
            .ok_or_else(|| "Path invalido".to_string())?;

        let path_a_verificar = vec![path.to_string()];
        let mut check_ignore = CheckIgnore::from(path_a_verificar, logger)?;
        let archivos_ignorados = check_ignore.ejecutar()?;
        if !archivos_ignorados.is_empty() {
            return Ok(true);
        }
        Ok(false)
    }

    /// Devueve un CheckIgnore con los paths de los archivos a verificar.
    pub fn from(args: Vec<String>, logger: Arc<Logger>) -> Result<CheckIgnore, String> {
        if args.is_empty() {
            return Err("Ingrese la ruta del archivo buscado como parametro".to_string());
        }
        let paths = args;
        Ok(CheckIgnore { logger, paths })
    }
}

/// Ejecuta el comando check-ignore.
/// Devuelve un string con todos los archivos consultados que resultaron estar ignorados.
/// Si no hay archivos ignorados, devuelve un string vacio.
impl Ejecutar for CheckIgnore {
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Buscando archivos ignorados");
        let archivos_ignorados = match io::leer_a_string(".girignore") {
            Ok(archivos_ignorados) => archivos_ignorados.trim().to_string(),
            Err(_) => return Ok("".to_string()),
        };
        if archivos_ignorados.is_empty() {
            return Ok("".to_string());
        }

        let archivos_ignorados_separados: Vec<PathBuf> = archivos_ignorados
            .split('\n')
            .filter(|x| !x.is_empty())
            .map(PathBuf::from)
            .collect();

        let mut archivos_encontrados: Vec<String> = Vec::new();

        for path in &self.paths {
            if utils::path_buf::esta_directorio_habilitado(
                &PathBuf::from(path),
                &archivos_ignorados_separados,
            ) {
                archivos_encontrados.push(path.to_string());
            }
        }

        self.logger.log("Check ignore finalizado");

        Ok(archivos_encontrados.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::{add::Add, check_ignore::CheckIgnore, init::Init, status::Status},
            logger::Logger,
        },
        utils::{self, io},
    };

    fn settupear_girignore_para_tests() {
        let archivos_a_ignorar = ".log\n.gitignore\n.vscode\n.girignore\n.log.txt\ntes_dir/";
        io::crear_archivo(".girignore").unwrap();
        io::escribir_bytes(".girignore", archivos_a_ignorar).unwrap();
    }

    //para evitar el target al obtener el status
    fn girignore_original() {
        let archivos_a_ignorar = ".log\n.gitignore\n.vscode\n.girignore\n.log.txt\ntes_dir/\ndiagrama.png\ntarget/\n.DS_Store\n.gir/\n.git/";
        io::crear_archivo(".girignore").unwrap();
        io::escribir_bytes(".girignore", archivos_a_ignorar).unwrap();
    }

    fn limpiar_archivo_gir() {
        io::rm_directorio(".gir").unwrap();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/check_ignore_init")).unwrap());
        let mut init = Init {
            path: "./.gir".to_string(),
            logger,
        };
        init.ejecutar().unwrap();
    }

    #[test]
    #[serial]
    fn test01_check_ignore_ignora_un_solo_archivo() {
        settupear_girignore_para_tests();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/check_ignore_test01")).unwrap());
        let mut check_ignore = CheckIgnore::from(vec![".log".to_string()], logger).unwrap();
        let resultado = check_ignore.ejecutar().unwrap();
        assert_eq!(resultado, ".log");
    }

    #[test]
    #[serial]
    fn test02_check_ignore_ignora_varios_archivos() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/check_ignore_test02")).unwrap());
        let mut check_ignore = CheckIgnore::from(
            vec![
                ".log".to_string(),
                ".girignore".to_string(),
                "tes_dir/".to_string(),
            ],
            logger,
        )
        .unwrap();
        let resultado = check_ignore.ejecutar().unwrap();
        assert_eq!(resultado, ".log\n.girignore\ntes_dir/");
    }

    #[test]
    #[serial]
    fn test03_al_addear_archivos_ignorados_estos_no_se_addean() {
        limpiar_archivo_gir();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/check_ignore_test03")).unwrap());
        let mut add = Add::from(
            vec![
                ".log".to_string(),
                ".girignore".to_string(),
                "tes_dir/".to_string(),
            ],
            logger.clone(),
        )
        .unwrap();
        add.ejecutar().unwrap();
        let index = utils::index::leer_index(logger.clone()).unwrap();
        assert_eq!(index.len(), 0);
    }

    #[test]
    #[serial]
    fn test04_obtener_untrackeados_del_status_ignora_los_archivos_ignorados() {
        girignore_original();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/check_ignore_test04")).unwrap());
        let status = Status::from(logger).unwrap();
        let untrackeados = status.obtener_untrackeados().unwrap();
        assert!(!untrackeados.iter().any(|x| x == ".log"));
        assert!(!untrackeados.iter().any(|x| x == ".girignore"));
        assert!(!untrackeados.iter().any(|x| x == "tes_dir"));
    }

    #[test]
    #[serial]
    fn test05_ignora_files_dentro_de_directorios_ignorados() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/check_ignore_test05")).unwrap());
        let mut check_ignore =
            CheckIgnore::from(vec!["tes_dir/archivo_dir.txt".to_string()], logger).unwrap();
        let resultado = check_ignore.ejecutar().unwrap();
        assert_eq!(resultado, "tes_dir/archivo_dir.txt");
    }

    #[test]
    #[serial]
    fn test06_si_tengo_un_archivo_y_directorio_con_nombres_parecidos_solo_ignora_al_indicado() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/check_ignore_test06")).unwrap());
        io::escribir_bytes(".girignore", "test_file/").unwrap();
        let mut check_ignore =
            CheckIgnore::from(vec!["test_file.txt".to_string()], logger).unwrap();
        io::crear_directorio("test_file").unwrap();
        let resultado = check_ignore.ejecutar().unwrap();
        io::rm_directorio("test_file").unwrap();
        girignore_original();
        assert!(resultado.is_empty());
    }
}
