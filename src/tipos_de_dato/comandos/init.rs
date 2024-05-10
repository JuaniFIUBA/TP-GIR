use std::{fs, path::Path, sync::Arc};

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger},
    utils::io,
};

pub struct Init {
    /// Path donde se creara el directorio .gir
    pub path: String,
    /// Logger para registrar los eventos ocurridos durante la ejecucion del comando.
    pub logger: Arc<Logger>,
}

impl Init {
    /// Valida que los argumentos sean correctos, no puede haber mas de un argumento.
    /// En caso de ser invalidos devuelve un error.
    pub fn validar_argumentos(args: Vec<String>) -> Result<(), String> {
        if args.len() > 1 {
            return Err("Argumentos desconocidos\n gir init [<directory>]".to_string());
        }

        Ok(())
    }

    /// Crea un comando init a partir de los argumentos pasados por linea de comandos.
    /// En caso de tener argumentos invalidos devuelve error.
    /// Si no se especifica un directorio, se crea el directorio .gir en el directorio actual.
    pub fn from(args: Vec<String>, logger: Arc<Logger>) -> Result<Init, String> {
        logger.log(&format!(
            "Se intenta crear comando init con args:{:?}",
            args
        ));

        Self::validar_argumentos(args.clone())?;

        logger.log(&format!("Se creo correctamente el comando init:{:?}", args));

        Ok(Init {
            path: Self::obtener_path(args),
            logger,
        })
    }

    /// Obtiene desde los argumentos el path donde se creara el directorio .gir.
    /// Si no se especifica un directorio, devuelve el directorio actual.
    fn obtener_path(args: Vec<String>) -> String {
        if args.is_empty() {
            "./.gir".to_string()
        } else {
            format!("{}{}", args[0], "/.gir")
        }
    }

    /// Crea el directorio .gir en el lugar indicado por el path, si es que no esta creado.
    /// En caso de ocurrir un error al crear el directorio completo, se borra el directorio .gir creado.
    fn crear_directorio_gir(&self) -> Result<(), String> {
        if self.verificar_si_ya_esta_creado_directorio_gir() {
            return Err("Ya existe un repositorio en este directorio".to_string());
        };

        if let Err(msj_err) = self.crear_directorios_y_archivos_gir() {
            self.borrar_directorios_y_archivos_git();
            return Err(msj_err);
        }

        Ok(())
    }

    /// Borra el directorio .gir creado.
    fn borrar_directorios_y_archivos_git(&self) {
        let _ = fs::remove_dir_all(self.path.clone());
    }

    /// Crea los directorios y archivos necesarios para el funcionamiento de gir.
    /// Crea los directorios: .gir, .gir/objects, .gir/refs/heads, .gir/refs/tags, .gir/refs/remotes.
    /// Ademas crea el archivo .gir/CONFIG e inicializa la rama default llamada master.
    /// En caso de ocurrir un error al crear alguno de los directorios o archivos, devuelve un error.
    fn crear_directorios_y_archivos_gir(&self) -> Result<(), String> {
        io::crear_directorio(self.path.clone())?;
        io::crear_directorio(self.path.clone() + "/objects")?;
        io::crear_directorio(self.path.clone() + "/refs/heads")?;
        io::crear_directorio(self.path.clone() + "/refs/tags")?;
        io::crear_directorio(self.path.clone() + "/refs/remotes")?;
        io::crear_archivo(self.path.clone() + "/config")?;
        io::crear_archivo(self.path.clone() + "/refs/heads/master")?;
        io::crear_archivo(self.path.clone() + "/index")?;
        self.crear_archivo_head()
    }

    /// Crea el archivo .gir/HEAD apuntando a la rama actual, la cual es la default master.
    /// En caso de ocurrir un error al crear el archivo, devuelve un error.
    fn crear_archivo_head(&self) -> Result<(), String> {
        let dir_archivo_head = self.path.clone() + "/HEAD";
        let contenido_inicial_head = "ref: refs/heads/master";

        io::crear_archivo(dir_archivo_head.clone())?;
        io::escribir_bytes(dir_archivo_head, contenido_inicial_head)
    }

    /// Verifica si ya existe un directorio .gir en el lugar indicado por el path.
    fn verificar_si_ya_esta_creado_directorio_gir(&self) -> bool {
        Path::new(&self.path).exists()
    }
}

impl Ejecutar for Init {
    /// Ejecuta el comando init.
    /// Crea el directorio en el lugar indicado por el path, caso contrario en el directorio actual.
    /// Si el directorio ya existe devuelve un mensaje y cancela la ejecucion.
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Se ejecuta init");

        self.crear_directorio_gir()?;

        let mensaje = format!("Directorio gir creado en {}", self.path);

        self.logger.log(&mensaje);

        Ok(mensaje)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{path::Path, sync::Arc};

    use crate::tipos_de_dato::{comandos::init::Init, logger::Logger};
    use std::path::PathBuf;

    #[test]
    #[serial]
    fn test01_obtener_path() {
        let args = vec!["otro".to_string()];
        assert_eq!(Init::obtener_path(args), "otro/.gir");
    }

    #[test]
    #[serial]
    fn test02_crear_directorio_gir() {
        let _ = std::fs::remove_dir_all("./.gir");
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_test01")).unwrap());
        let init = Init {
            path: "./.gir".to_string(),
            logger,
        };

        assert!(init.crear_directorio_gir().is_ok());
        assert!(Path::new("./.gir").exists());
        assert!(Path::new("./.gir/objects").exists());
        assert!(Path::new("./.gir/refs/heads").exists());
        assert!(Path::new("./.gir/refs/tags").exists());
        assert!(Path::new("./.gir/refs/remotes").exists());
        assert!(Path::new("./.gir/config").exists());
        assert!(Path::new("./.gir/refs/heads/master").exists());
        assert!(Path::new("./.gir/HEAD").exists());
        assert!(Path::new("./.gir/index").exists());
    }
}
