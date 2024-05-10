use crate::tipos_de_dato::comando::Ejecutar;
use crate::tipos_de_dato::logger::Logger;
use crate::tipos_de_dato::objetos::tree::Tree;
use crate::utils::{self, io};

use std::path::PathBuf;
use std::sync::Arc;

use super::fetch::Fetch;
use super::init::Init;

use super::remote::Remote;
use super::set_upstream::SetUpstream;
use super::write_tree;

const UBICACION_RAMA_MASTER: &str = "./.gir/refs/heads/master";
const GIR_CLONE: &str = "gir clone <ip:puerto/repositorio/>";
pub struct Clone {
    logger: Arc<Logger>,
    url: String,
    clonar_en_dir_actual: bool,
}

impl Clone {
    /// Crea un nueva instancia de clone
    pub fn from(
        args: &mut Vec<String>,
        logger: Arc<Logger>,
        clonar_en_dir_actual: bool,
    ) -> Result<Clone, String> {
        Self::verificar_argumentos(args)?;

        let url = args.remove(0);

        logger.log(&format!("Se creo clone con exito - url: {}", url));

        Ok(Clone {
            logger,
            url,
            clonar_en_dir_actual,
        })
    }

    fn verificar_argumentos(args: &Vec<String>) -> Result<(), String> {
        if args.len() != 1 {
            return Err(format!(
                "Parametros desconocidos {}\n {}",
                args.join(" "),
                GIR_CLONE
            ));
        };
        Ok(())
    }

    /// Verifica si el repositorio ya existe en el sistema
    /// Si existe verifica que este vacio
    fn verificar_si_ya_existe_repositorio(&self, repositorio: &str) -> Result<(), String> {
        if PathBuf::from(repositorio).exists() {
            //me fijo si tiene contenido
            if utils::io::leer_directorio(&repositorio)?.count() > 0 {
                return Err(format!("Error el directorio {} no esta vacio", repositorio));
            }
        }

        Ok(())
    }

    /// Obtiene la rama predeterminada del repositorio
    /// Si no existe la rama master, devuelve la ultima rama que se creo
    pub fn obtener_rama_predeterminada() -> Result<String, String> {
        let ramas = utils::io::leer_directorio(".gir/refs/remotes/origin")?;
        let mut rama_predeterminada = String::new();

        for rama in ramas {
            let rama = rama.map_err(|e| e.to_string())?;
            let nombre_rama = utils::path_buf::obtener_nombre(&rama.path())?;
            if nombre_rama == "master" {
                return Ok(nombre_rama);
            }
            rama_predeterminada = nombre_rama;
        }

        Ok(rama_predeterminada)
    }

    /// Realiza un fast forward de la rama master local a la rama master remota. Para ello
    /// se obtiene el arbol del commit de la rama master remota y se lo escribe en el directorio
    /// de trabajo.
    fn fast_forward_de_cero(&self, commit_head_remoto: &str) -> Result<bool, String> {
        io::escribir_bytes(UBICACION_RAMA_MASTER, commit_head_remoto)?;
        let hash_tree_padre = write_tree::conseguir_arbol(commit_head_remoto)?;
        let tree_branch_a_mergear =
            Tree::from_hash(&hash_tree_padre, PathBuf::from("."), self.logger.clone())?;

        tree_branch_a_mergear.escribir_en_directorio()?;

        self.logger
            .log("Fast forward ejucutado con exito en clone de la rama remota");

        Ok(true)
    }

    ///Busca el archivo correspondiente que contien el HEAD del remoto (el NOMBREREMOTO_HEAD)y lo obtiene. En caso de no
    /// existir dicho archivo toma por defecto devulevor el commit de master del remoto.   
    fn obtener_head_remoto(&self, remoto: &str, rama_remota: &str) -> Result<String, String> {
        let path_remoto = PathBuf::from(format!("./.gir/{}_HEAD", remoto.to_uppercase()));

        if path_remoto.exists() {
            utils::io::leer_a_string(path_remoto)
        } else {
            let path_master_remoto =
                PathBuf::from(format!("./.gir/refs/remotes/{}/{}", remoto, rama_remota));

            utils::io::leer_a_string(path_master_remoto)
        }
    }
    /// Crea el repositorio en el sistema
    fn crear_repositorio(&mut self) -> Result<(), String> {
        Init::from(Vec::new(), self.logger.clone())?.ejecutar()?;

        let remote_args = &mut vec!["add".to_string(), "origin".to_string(), self.url.clone()];
        Remote::from(remote_args, self.logger.clone())?.ejecutar()?;

        Fetch::new(vec!["origin".to_string()], self.logger.clone())?.ejecutar()?;
        let rama_predeterminada = Self::obtener_rama_predeterminada()?;

        let commit_head_remoto = self.obtener_head_remoto("origin", &rama_predeterminada)?;
        self.fast_forward_de_cero(&commit_head_remoto)?;

        SetUpstream::new(
            "origin".to_string(),
            rama_predeterminada,
            utils::ramas::obtener_rama_actual()?,
            self.logger.clone(),
        )?
        .ejecutar()?;

        Ok(())
    }
}

impl Ejecutar for Clone {
    /// Ejecuta el comando clone.
    fn ejecutar(&mut self) -> Result<String, String> {
        let (_, mut repositorio) = utils::strings::obtener_ip_puerto_y_repositorio(&self.url)?;
        repositorio = repositorio.replace('/', "");

        self.verificar_si_ya_existe_repositorio(&repositorio)?;

        if !self.clonar_en_dir_actual {
            utils::io::crear_carpeta(&repositorio)?;
            utils::io::cambiar_directorio(&repositorio)?;
        }

        let resutado = self.crear_repositorio();

        if !self.clonar_en_dir_actual {
            utils::io::cambiar_directorio("..")?;
        }

        resutado?;

        let mensaje = "Clone ejecutado con exito".to_string();
        self.logger.log(&mensaje);
        Ok(mensaje)
    }
}
