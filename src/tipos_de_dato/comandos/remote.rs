use std::sync::Arc;

use crate::{
    tipos_de_dato::{
        comando::Ejecutar,
        config::{Config, RemoteInfo},
        logger::Logger,
        variante_comando_remote::ComandoRemote,
    },
    utils::ramas,
};

pub struct Remote {
    /// Comando a ejecutar.
    comando: ComandoRemote,
    /// Nombre del remote.
    nombre: Option<String>,
    /// Url del remote.
    url: Option<String>,
    /// Logger para imprimir mensajes en el archivo log.
    logger: Arc<Logger>,
}

const INPUT_ERROR: &str = "gir remote add [<nombre-remote>] [<url-remote>]\ngir remote delete [<nombre-remote>] [<url-remote>]\ngir remote set-url [<nombre-remote>] [<url-remote>]\ngir remote show-url [<nombre-remote>]";

impl Remote {
    /// Crea una instancia de Remote.
    /// Si la cantidad de argumentos es mayor a 3 devuelve error.
    /// Si la cantidad de argumentos es 0 devuelve una instancia de Remote con el comando Mostrar.
    /// Si la cantidad de argumentos es 2 devuelve una instancia de Remote con el comando Eliminar o MostrarUrl.
    /// Si la cantidad de argumentos es 3 devuelve una instancia de Remote con el comando Agregar o CambiarUrl.
    /// Quedo muy largo debido a que son muchas variantes del mismo comando y cargo fmt agrega muchas lineas innecesarias
    pub fn from(args: &mut Vec<String>, logger: Arc<Logger>) -> Result<Remote, String> {
        if args.len() > 3 {
            return Err(format!("Demasiados argumentos\n{}", INPUT_ERROR));
        }

        if args.is_empty() {
            return Ok(Remote {
                comando: ComandoRemote::Mostrar,
                logger,
                nombre: None,
                url: None,
            });
        }

        if args.len() == 2 {
            match args[0].as_str() {
                "show-url" => {
                    return Ok(Remote {
                        comando: ComandoRemote::MostrarUrl,
                        logger,
                        nombre: Some(args[1].clone()),
                        url: None,
                    })
                }
                "delete" => {
                    return Ok(Remote {
                        comando: ComandoRemote::Eliminar,
                        logger,
                        nombre: Some(args[1].clone()),
                        url: None,
                    })
                }
                _ => return Err(INPUT_ERROR.to_string()),
            }
        }

        if args.len() == 3 {
            match args[0].as_str() {
                "add" => {
                    return Ok(Remote {
                        comando: ComandoRemote::Agregar,
                        logger,
                        nombre: Some(args[1].clone()),
                        url: Some(args[2].clone()),
                    })
                }
                "set-url" => {
                    return Ok(Remote {
                        comando: ComandoRemote::CambiarUrl,
                        logger,
                        nombre: Some(args[1].clone()),
                        url: Some(args[2].clone()),
                    })
                }
                _ => return Err(INPUT_ERROR.to_string()),
            }
        };

        Err(INPUT_ERROR.to_string())
    }

    /// Agrega un remote a la configuración.
    fn agregar(&self) -> Result<String, String> {
        let mut config = Config::leer_config()?;

        let remote = RemoteInfo {
            nombre: self.nombre.clone().unwrap(),
            url: self.url.clone().unwrap(),
        };

        let remote_encontrada = config.remotos.iter().find(|r| r.nombre == remote.nombre);

        if remote_encontrada.is_some() {
            return Err("Ya existe un remote con ese nombre".to_string());
        }

        config.remotos.push(remote);
        config.guardar_config()?;
        let msg = format!("Se agrego el remote {}", self.nombre.clone().unwrap());

        self.logger.log(&msg);
        Ok(msg)
    }

    /// Elimina un remote de la configuración.
    fn eliminar(&self) -> Result<String, String> {
        let mut config = Config::leer_config()?;

        let nombre = self
            .nombre
            .clone()
            .ok_or("No se especifico el nombre del remote")?;

        let indice = config
            .remotos
            .iter()
            .position(|r| r.nombre == nombre.clone());

        if indice.is_none() {
            return Err("No existe un remote con ese nombre".to_string());
        }

        config.remotos.remove(indice.unwrap());
        config.guardar_config()?;

        Ok(format!("Se elimino el remote {}", nombre))
    }

    /// Cambia la url de un remote de la configuración.
    fn cambiar_url(&self) -> Result<String, String> {
        let mut config = Config::leer_config()?;

        let nombre = self
            .nombre
            .clone()
            .ok_or("No se especifico el nombre del remote")?;

        let url = self
            .url
            .clone()
            .ok_or("No se especifico la url del remote")?;

        let indice_result = config.remotos.iter().position(|r| r.nombre == nombre);

        let indice = match indice_result {
            Some(indice) => indice,
            None => return Err("No existe un remote con ese nombre".to_string()),
        };

        config.remotos[indice] = RemoteInfo {
            nombre: nombre.clone(),
            url: url.clone(),
        };
        config.guardar_config()?;

        Ok(format!("Se cambio la url del remote {} a {}", nombre, url))
    }

    /// Muestra la url asociada a un remote de la configuración.
    fn mostrar_url(&self) -> Result<String, String> {
        let config = Config::leer_config()?;

        let nombre = self
            .nombre
            .clone()
            .ok_or("No se especifico el nombre del remote")?;

        let remote = config.remotos.iter().find(|r| r.nombre == nombre);

        if remote.is_none() {
            return Err("No existe un remote con ese nombre".to_string());
        }

        config.guardar_config()?;

        Ok(format!(
            "La url del remote {} es {}",
            nombre,
            remote.unwrap().url
        ))
    }

    /// Muestra el remote asociado a la rama actual.
    fn mostrar(&self) -> Result<String, String> {
        let config = Config::leer_config()?;

        let branch_actual = ramas::obtener_rama_actual()?;
        let remote_actual = config
            .ramas
            .iter()
            .find(|branch| branch.nombre == branch_actual);

        match remote_actual {
            Some(remote) => Ok(remote.remote.clone()),
            None => Err("No hay un remote asociado a la branch actual\n".to_string()),
        }
    }
}

impl Ejecutar for Remote {
    /// Ejecuta el comando.
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Ejecutando comando remote");
        match &self.comando {
            ComandoRemote::Mostrar => self.mostrar(),
            ComandoRemote::Agregar => self.agregar(),
            ComandoRemote::Eliminar => self.eliminar(),
            ComandoRemote::CambiarUrl => self.cambiar_url(),
            ComandoRemote::MostrarUrl => self.mostrar_url(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serial_test::serial;

    use super::*;
    use crate::{
        tipos_de_dato::comandos::set_upstream::SetUpstream,
        utils::{self, testing::limpiar_archivo_gir},
    };

    #[test]
    #[serial]
    fn test01_agregar_remote() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/remote_test01")).unwrap());
        limpiar_archivo_gir(logger.clone());

        let mut args = vec![
            "add".to_string(),
            "origin".to_string(),
            "ip:puerto/remoto/".to_string(),
        ];
        let mut remote = Remote::from(&mut args, logger.clone()).unwrap();

        let _ = remote.ejecutar().unwrap();
        let config = Config::leer_config().unwrap();
        assert!(config.existe_remote("origin"));
    }

    #[test]
    #[serial]
    fn test02_mostrar_remote() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/remote_test02")).unwrap());

        let mut args = vec![];
        let mut remote = Remote::from(&mut args, logger.clone()).unwrap();
        let remoto = "origin".to_string();
        let rama_remota = "trabajo".to_string();
        let rama_local = "master".to_string();

        utils::testing::escribir_rama_remota(&remoto, &rama_remota);

        SetUpstream::new(
            remoto.clone(),
            rama_remota.clone(),
            rama_local.clone(),
            logger,
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let remoto = remote.ejecutar().unwrap();
        assert_eq!(remoto, "origin");
    }

    #[test]
    #[serial]
    fn test03_mostrar_url_remote() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/remote_test03")).unwrap());

        let mut args = vec!["show-url".to_string(), "origin".to_string()];
        let mut remote = Remote::from(&mut args, logger.clone()).unwrap();

        let url = remote.ejecutar().unwrap();
        assert_eq!(url, "La url del remote origin es ip:puerto/remoto/");
    }

    #[test]
    #[serial]
    fn test04_cambiar_url_remote() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/remote_test04")).unwrap());

        let mut args = vec![
            "set-url".to_string(),
            "origin".to_string(),
            "ip:puerto/remoto/nueva/".to_string(),
        ];
        let mut remote = Remote::from(&mut args, logger.clone()).unwrap();

        let _ = remote.ejecutar().unwrap();
        let config = Config::leer_config().unwrap();
        assert_eq!(config.remotos[0].url, "ip:puerto/remoto/nueva/".to_string());
    }

    #[test]
    #[serial]
    fn test05_mostrar_url_remote() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/remote_test05")).unwrap());

        let mut args = vec!["show-url".to_string(), "origin".to_string()];
        let mut remote = Remote::from(&mut args, logger.clone()).unwrap();

        let url = remote.ejecutar().unwrap();
        assert_eq!(url, "La url del remote origin es ip:puerto/remoto/nueva/");
    }

    #[test]
    #[serial]
    fn test06_eliminar_remote() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/remote_test06")).unwrap());

        let mut args = vec!["delete".to_string(), "origin".to_string()];
        let mut remote = Remote::from(&mut args, logger.clone()).unwrap();

        let _ = remote.ejecutar().unwrap();
        let config = Config::leer_config().unwrap();
        assert!(!config.existe_remote("origin"));
    }
}
