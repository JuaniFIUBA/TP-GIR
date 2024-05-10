use std::{path, sync::Arc};

use chrono::TimeZone;

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger},
    utils::{
        compresion::comprimir_contenido,
        gir_config::{armar_config_con_mail_y_nombre, conseguir_nombre_y_mail_del_config},
        index::limpiar_archivo_index,
        io, ramas,
    },
};

use super::{hash_object::HashObject, merge::Merge, write_tree};

pub struct Commit {
    /// Logger para imprimir mensajes en el archivo log.
    pub logger: Arc<Logger>,
    /// Mensaje del commit.
    pub mensaje: String,
    pub rama_actual: String,
}

/// Arma el timestamp del commit en formato unix.
/// En caso de no poder obtener la zona horaria o la fecha y hora actual devuelve un error.
/// El formato del timestamp es: <timestamp> <offset>
/// Donde timestamp es la cantidad de segundos desde el 1 de enero de 1970 y offset es la diferencia
/// en horas y minutos con respecto a UTC. Se asumio que el offset es -0300.
/// Ejemplo: 1614550000 -0300
fn armar_timestamp_commit() -> Result<String, String> {
    let zona_horaria = match chrono::FixedOffset::west_opt(3 * 3600) {
        Some(zona_horaria) => zona_horaria,
        None => return Err("No se pudo obtener la zona horaria".to_string()),
    };
    let now = match zona_horaria.from_local_datetime(&chrono::Local::now().naive_local()) {
        chrono::LocalResult::Single(now) => now,
        _ => return Err("No se pudo obtener la fecha y hora actual".to_string()),
    };
    let timestamp = now.timestamp();
    let offset_horas = -3;
    let offset_minutos = 0;

    let offset_format = format!("{:-03}{:02}", offset_horas, offset_minutos);
    Ok(format!("{} {}", timestamp, offset_format))
}

impl Commit {
    /// Crea un commit a partir de los argumentos pasados por linea de comandos.
    /// En caso de no tener argumentos y haber un merge en curso, crea un commit de merge.
    /// Se espera que contenga el flag -m y un mensaje.
    /// En caso de tener argumentos invalidos devuelve error.
    pub fn from(args: &mut Vec<String>, logger: Arc<Logger>) -> Result<Commit, String> {
        if args.is_empty() && Merge::hay_merge_en_curso()? {
            if Merge::hay_archivos_sin_mergear(logger.clone())? {
                return Err("Hay archivos sin mergear".to_string());
            }
            let rama_actual = ramas::obtener_rama_actual()?;
            return Commit::from_merge(logger, &rama_actual);
        }

        if args.len() != 2 {
            return Err("La cantidad de argumentos es invalida, -m esperado".to_string());
        }
        let mensaje = args
            .pop()
            .ok_or_else(|| "No se especifico un mensaje luego del flag -m".to_string())?;
        let flag = args
            .pop()
            .ok_or_else(|| "No se especifico un flag".to_string())?;
        if flag != "-m" {
            return Err(format!("Flag desconocido {}", flag));
        }

        let rama_actual = ramas::obtener_rama_actual()?;
        Ok(Commit {
            mensaje,
            logger,
            rama_actual,
        })
    }

    /// Crea un commit a partir del mensaje en el archivo COMMIT_EDITMSG.
    pub fn from_merge(logger: Arc<Logger>, rama_actual: &str) -> Result<Commit, String> {
        let mensaje = io::leer_a_string(path::Path::new(".gir/COMMIT_EDITMSG"))?;
        Ok(Commit {
            mensaje,
            logger,
            rama_actual: rama_actual.to_string(),
        })
    }

    /// Formatea el contenido del commit.
    /// Devuelve el contenido del commit en formato git.
    /// Toma el nombre y mail del archivo de configuracion.
    fn formatear_contenido_commit(
        &self,
        hash_arbol: &str,
        hash_padre_commit: &str,
    ) -> Result<String, String> {
        let mut contenido_commit = String::new();
        contenido_commit.push_str(&format!("tree {}\n", hash_arbol));
        if !hash_padre_commit.is_empty() {
            contenido_commit.push_str(&format!("parent {}\n", hash_padre_commit));
        }
        if Merge::hay_merge_en_curso()? {
            let padre_mergeado = io::leer_a_string(path::Path::new(".gir/MERGE_HEAD"))?;
            contenido_commit.push_str(&format!("parent {}\n", padre_mergeado));
        }
        let (nombre, mail) = conseguir_nombre_y_mail_del_config()?;
        let linea_autor = format!("{} {}", nombre, mail);
        let timestamp = armar_timestamp_commit()?;
        contenido_commit.push_str(&format!(
            "author {} {}\ncommitter {} {}\n\n{}",
            linea_autor, timestamp, linea_autor, timestamp, self.mensaje
        ));
        Ok(contenido_commit)
    }

    /// Crea el contenido del commit.
    /// Devuelve el hash del arbol y el contenido total del commit.
    fn crear_contenido_commit(&self) -> Result<(String, String), String> {
        let hash_commit = Merge::obtener_commit_de_branch(&self.rama_actual)?;
        let hash_arbol = match hash_commit.as_str() {
            "" => write_tree::crear_arbol_commit(None, self.logger.clone())?,
            _ => {
                write_tree::crear_arbol_commit(Some(hash_commit.to_string()), self.logger.clone())?
            }
        };
        let contenido_commit = self.formatear_contenido_commit(&hash_arbol, &hash_commit)?;
        let header = format!("commit {}\0", contenido_commit.len());
        let contenido_total = format!("{}{}", header, contenido_commit);
        Ok((hash_arbol, contenido_total))
    }

    /// Escribe el objeto commit en el repositorio.
    /// El objeto commit se escribe en .gir/objects/.
    fn escribir_objeto_commit(hash: &str, contenido_comprimido: Vec<u8>) -> Result<(), String> {
        let ruta = format!(".gir/objects/{}/{}", &hash[..2], &hash[2..]);
        io::escribir_bytes(ruta, contenido_comprimido)?;
        Ok(())
    }

    /// Actualiza el archivo head/ref de la branch actual con el hash del commit creado.
    /// En caso de no poder abrir o escribir en el archivo devuelve un error.
    fn updatear_ref_head(&self, hash: &str) -> Result<(), String> {
        let ruta = format!(".gir/refs/heads/{}", self.rama_actual);
        io::escribir_bytes(ruta, hash)?;
        Ok(())
    }

    /// Ejecuta el comando commit.
    /// Devuelve un mensaje indicando si se pudo crear el commit o no.
    fn ejecutar_wrapper(&self, contenido_total: &str) -> Result<(), String> {
        let contenido_comprimido = comprimir_contenido(contenido_total)?;
        let hash = HashObject::hashear_contenido_objeto(&contenido_total.as_bytes().to_vec());
        Self::updatear_ref_head(self, &hash)?;
        Self::escribir_objeto_commit(&hash, contenido_comprimido)?;
        self.logger.log(&format!(
            "commit {}\n Author: {}\n{} ",
            hash, "", self.mensaje
        ));
        limpiar_archivo_index()?;
        Ok(())
    }
}

impl Ejecutar for Commit {
    /// Ejecuta el comando commit en su totalidad.
    /// Utiliza un ejecutar wrapper para que en caso de error limpiar los archivos creados.
    fn ejecutar(&mut self) -> Result<String, String> {
        armar_config_con_mail_y_nombre()?;
        let (hash_arbol, contenido_total) = self.crear_contenido_commit()?;
        match self.ejecutar_wrapper(&contenido_total) {
            Ok(_) => (),
            Err(_) => {
                io::rm_directorio(format!(
                    ".gir/objects/{}/{}",
                    &hash_arbol[..2],
                    &hash_arbol[2..]
                ))?;
                return Err("No se pudo ejecutar el commit".to_string());
            }
        };
        Merge::limpiar_merge_post_commit()?;
        Ok("Commit creado".to_string())
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::{add::Add, hash_object::HashObject},
            logger::Logger,
        },
        utils::{
            compresion::{descomprimir_objeto, descomprimir_objeto_gir},
            io,
            testing::{addear_archivos_y_comittear, limpiar_archivo_gir},
        },
    };

    use super::Commit;

    fn craer_archivo_config_default() {
        let home = std::env::var("HOME").unwrap();
        let config_path = format!("{home}/.girconfig");
        let contenido = "nombre = ejemplo_nombre\nmail = ejemplo_mail\n".to_string();
        io::escribir_bytes(config_path, contenido).unwrap();
    }

    fn conseguir_hash_padre(branch: &str) -> Result<String, String> {
        let hash = io::leer_a_string(format!(".gir/refs/heads/{}", branch))?;
        let contenido = descomprimir_objeto(&hash, ".gir/objects/")?;
        let lineas_sin_null = contenido.replace('\0', "\n");
        let lineas = lineas_sin_null.split('\n').collect::<Vec<&str>>();
        let linea_supuesto_padre = lineas[2].split(' ').collect::<Vec<&str>>();
        let hash_padre = match linea_supuesto_padre[0] {
            "parent" => linea_supuesto_padre[1],
            _ => "",
        };
        Ok(hash_padre.to_string())
    }

    fn conseguir_arbol_commit(branch: &str) -> Result<String, String> {
        let hash_hijo = io::leer_a_string(format!(".gir/refs/heads/{}", branch)).unwrap();
        let contenido_hijo = descomprimir_objeto(&hash_hijo, ".gir/objects/").unwrap();
        let lineas_sin_null = contenido_hijo.replace('\0', "\n");
        let lineas = lineas_sin_null.split('\n').collect::<Vec<&str>>();
        let arbol_commit = lineas[1];
        let lineas = arbol_commit.split(' ').collect::<Vec<&str>>();
        let arbol_commit = lineas[1];
        Ok(arbol_commit.to_string())
    }

    #[test]
    #[serial]
    fn test01_se_actualiza_el_head_ref_correspondiente_con_el_hash_del_commit() {
        craer_archivo_config_default();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test01")).unwrap());
        limpiar_archivo_gir(logger.clone());
        let mut add = Add::from(vec!["test_file.txt".to_string()], logger.clone()).unwrap();
        add.ejecutar().unwrap();
        let mut commit =
            Commit::from(&mut vec!["-m".to_string(), "mensaje".to_string()], logger).unwrap();
        commit.ejecutar().unwrap();
        let arbol_last_commit = conseguir_arbol_commit("master");
        assert_eq!(
            arbol_last_commit,
            Ok("ce0ef9a25817847d31d12df1295248d24d07b309".to_string())
        );
    }

    #[test]
    #[serial]
    fn test02_al_hacer_dos_commits_el_primero_es_padre_del_segundo() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test02")).unwrap());
        limpiar_archivo_gir(logger.clone());

        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());

        let hash_padre = io::leer_a_string(".gir/refs/heads/master").unwrap();
        addear_archivos_y_comittear(vec!["test_file2.txt".to_string()], logger.clone());

        let hash_padre_desde_hijo = conseguir_hash_padre("master");
        assert_eq!(hash_padre_desde_hijo, Ok(hash_padre.to_string()));
    }

    #[test]
    #[serial]
    fn test03_al_hacer_commit_apunta_al_arbol_correcto() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test03")).unwrap());
        limpiar_archivo_gir(logger.clone());
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger);

        let hash_arbol = conseguir_arbol_commit("master").unwrap();
        let contenido_arbol = descomprimir_objeto(&hash_arbol, ".gir/objects/").unwrap();

        assert_eq!(
            contenido_arbol,
            "tree 41\0100644 test_file.txt\0678e12dc5c03a7cf6e9f64e688868962ab5d8b65".to_string()
        );
    }

    #[test]
    #[serial]
    fn test04_al_hacer_commit_de_un_archivo_y_luego_hacer_otro_commit_de_ese_archivo_modificado_el_hash_tree_es_correcto(
    ) {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test04")).unwrap());
        limpiar_archivo_gir(logger.clone());
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());

        io::escribir_bytes("test_file.txt", "hola").unwrap();
        addear_archivos_y_comittear(vec!["test_file.txt".to_string()], logger.clone());

        let hash_arbol = conseguir_arbol_commit("master").unwrap();
        let contenido_arbol = descomprimir_objeto_gir(&hash_arbol).unwrap();
        let hash_correcto =
            HashObject::from(&mut vec!["test_file.txt".to_string()], logger.clone())
                .unwrap()
                .ejecutar()
                .unwrap();

        io::escribir_bytes("test_file.txt", "test file modified").unwrap();
        assert_eq!(
            contenido_arbol,
            format!("tree 41\0100644 test_file.txt\0{}", hash_correcto)
        );
    }

    #[test]
    #[serial]
    fn test05_al_hacer_commit_de_un_directorio_y_luego_hacer_otro_commit_de_ese_directorio_modificado_el_hash_tree_es_correcto(
    ) {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test05")).unwrap());
        limpiar_archivo_gir(logger.clone());
        addear_archivos_y_comittear(vec!["test_dir/muchos_objetos".to_string()], logger.clone());

        io::escribir_bytes("test_dir/muchos_objetos/archivo.txt", "hola").unwrap();
        addear_archivos_y_comittear(vec!["test_dir/muchos_objetos".to_string()], logger.clone());

        let hash_arbol = conseguir_arbol_commit("master").unwrap();
        let hash_arbol_git = "c847ae43830604fea16a9830f90e60f0a5f0d993";

        io::escribir_bytes("test_dir/muchos_objetos/archivo.txt", "mas contenido").unwrap();
        assert_eq!(hash_arbol_git, hash_arbol);
    }
}
