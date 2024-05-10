use std::path::PathBuf;

use crate::utils::{self, io};

use super::info_ramas::RamasInfo;

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub nombre: String,
    pub url: String,
}

pub struct Config {
    pub remotos: Vec<RemoteInfo>,
    pub ramas: Vec<RamasInfo>,
}

impl Config {
    /// Lee el archivo gir/config y fetchea toda la informacion de remotes y branches.
    /// Por cada remote que lee crea su respectivo RemoteInfo.
    /// Por cada branch que lee crea su respectivo BranchInfo.
    /// Si el archivo no existe, devuelve un Config vacio.
    pub fn leer_config() -> Result<Config, String> {
        let contenido_config = io::leer_a_string(".gir/config")?;
        let contenido_spliteado = contenido_config.split('[').collect::<Vec<&str>>();
        let mut remotos: Vec<RemoteInfo> = Vec::new();
        let mut ramas: Vec<RamasInfo> = Vec::new();

        if contenido_config.is_empty() {
            return Ok(Config { remotos, ramas });
        }

        for contenido_raw in contenido_spliteado {
            if contenido_raw.is_empty() {
                continue;
            }

            let contenido = contenido_raw.split(']').collect::<Vec<&str>>();
            let header = contenido[0].split_whitespace().collect::<Vec<&str>>();
            match header[0] {
                "remote" => {
                    let informacion_remote = contenido[1].split(" = ").collect::<Vec<&str>>();

                    if informacion_remote[0].trim() != "url" {
                        return Err("Error en el archivo de configuracion".to_string());
                    }

                    let remote = RemoteInfo {
                        nombre: header[1].replace('\"', "").to_string(),
                        url: informacion_remote[1].trim().to_string(),
                    };
                    remotos.push(remote);
                }
                "branch" => {
                    let informacion_branch = contenido[1].trim().split('\n').collect::<Vec<&str>>();
                    let mut remote = String::new();
                    let mut merge = PathBuf::new();

                    for linea in informacion_branch {
                        let linea = linea.split(" = ").collect::<Vec<&str>>();
                        match linea[0].trim() {
                            "remote" => remote = linea[1].to_string(),
                            "merge" => merge.push(linea[1]),
                            _ => return Err("Error en el archivo de configuracion".to_string()),
                        }
                    }

                    if remote.is_empty() || merge.to_string_lossy().is_empty() {
                        return Err("Error en el archivo de configuracion".to_string());
                    }

                    let branch = RamasInfo {
                        nombre: header[1].replace('\"', "").to_string(),
                        remote,
                        merge,
                    };

                    ramas.push(branch);
                }
                _ => return Err("Error en el archivo de configuracion".to_string()),
            }
        }
        Ok(Config { remotos, ramas })
    }

    ///busca dentro de los remote del config, si remote efectivente existe.
    /// Si existe devuelve true, caso contrario false
    pub fn existe_remote(&self, remote: &str) -> bool {
        self.remotos.iter().any(|x| x.nombre == *remote)
    }

    ///Se fija si esta setea la rama en el config, si es asi devuelve true.
    /// Caso contrario devuleve false.
    pub fn existe_rama(&self, rama: &str) -> bool {
        self.ramas.iter().any(|x| x.nombre == *rama)
    }

    /// Devuelve true si la rama actual tiene un upstream asociado.
    /// Osea si tiene un remote asociado.
    pub fn hay_upstream(&self, rama: &str) -> bool {
        self.ramas
            .iter()
            .any(|x| x.nombre == *rama && !x.remote.is_empty())
    }

    ///en caso de existir un remoto asosiado a la rama actual, lo devuelve
    pub fn obtener_remoto_rama_actual(&self) -> Option<String> {
        let rama_actual = utils::ramas::obtener_rama_actual().ok()?;

        self.ramas
            .iter()
            .find(|&rama| rama.nombre == rama_actual)
            .map(|rama| rama.remote.to_owned())
    }

    ///En caso de existir un remoto y un rama_merge (osea si la rama actual esta configurada)asosiado a la rama actual, lo devuelve
    /// Ojo!! rama merge en formato dir como lo ve el server(Ej: refs/heads/master)
    pub fn obtener_remoto_y_rama_merge_rama_actual(&self) -> Option<(String, PathBuf)> {
        let rama_actual = utils::ramas::obtener_rama_actual().ok()?;
        self.obtener_remoto_y_rama_merge_rama(&rama_actual)
    }

    ///En caso de existir un remoto y un rama_merge (osea si la rama actual esta configurada)asosiado a la rama actual, lo devuelve
    /// Ojo!! rama merge en formato dir como lo ve el server(Ej: refs/heads/master)
    pub fn obtener_remoto_y_rama_merge_rama(
        &self,
        rama_actual: &String,
    ) -> Option<(String, PathBuf)> {
        self.ramas
            .iter()
            .find(|&rama| rama.nombre == *rama_actual)
            .map(|rama| (rama.remote.to_owned(), (*rama.merge).to_path_buf()))
    }

    ///Da el url asosiado al remoto
    pub fn obtenet_url_asosiado_remoto(&self, remoto: &str) -> Result<String, String> {
        match self
            .remotos
            .iter()
            .find(|remoto_i| remoto_i.nombre == *remoto)
        {
            Some(remoto) => Ok(remoto.url.clone()),
            None => Err(format!("Fallo en la busqueda de {}", remoto)),
        }
    }

    /// Por cada entry de informacion que tiene el Config, lo escribe en el archivo CONFIG.
    /// Si el archivo existe, lo sobreescribe.
    /// Si el archivo no existe, lo crea.
    pub fn guardar_config(&self) -> Result<(), String> {
        let mut contenido = String::new();

        for remote in &self.remotos {
            contenido.push_str(&format!("[remote \"{}\"]\n", remote.nombre));
            contenido.push_str(&format!("   url = {}\n", remote.url));
        }

        for branch in &self.ramas {
            contenido.push_str(&format!("[branch \"{}\"]\n", branch.nombre));
            contenido.push_str(&format!("   remote = {}\n", branch.remote));
            contenido.push_str(&format!("   merge = {}\n", branch.merge.to_string_lossy()));
        }

        io::escribir_bytes(PathBuf::from(".gir/config"), contenido)?;

        Ok(())
    }
}

#[cfg(test)]

mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test01_guardar_config() {
        let remote = RemoteInfo {
            nombre: "origin".to_string(),
            url: "localhost:3000/test_repo/".to_string(),
        };

        let rama = RamasInfo {
            nombre: "aaa".to_string(),
            remote: "origin".to_string(),
            merge: PathBuf::from("refs/heads/aaa"),
        };

        let config = Config {
            remotos: vec![remote],
            ramas: vec![rama],
        };

        config.guardar_config().unwrap();

        let file = io::leer_a_string(".gir/config").unwrap();

        assert_eq!(
            file,
            "[remote \"origin\"]\n   url = localhost:3000/test_repo/\n\
            [branch \"aaa\"]\n   remote = origin\n   merge = refs/heads/aaa\n"
        );
    }

    #[test]
    #[serial]

    fn test02_leer_config() {
        let remote = RemoteInfo {
            nombre: "origin".to_string(),
            url: "localhost:3000/test_repo/".to_string(),
        };

        let rama = RamasInfo {
            nombre: "aaa".to_string(),
            remote: "origin".to_string(),
            merge: PathBuf::from("refs/heads/aaa"),
        };

        let config = Config {
            remotos: vec![remote],
            ramas: vec![rama],
        };

        config.guardar_config().unwrap();

        let config = Config::leer_config().unwrap();

        assert_eq!(config.remotos[0].nombre, "origin");
        assert_eq!(config.remotos[0].url, "localhost:3000/test_repo/");
        assert_eq!(config.ramas[0].nombre, "aaa");
        assert_eq!(config.ramas[0].remote, "origin");
        assert_eq!(config.ramas[0].merge, PathBuf::from("refs/heads/aaa"));
    }

    #[test]
    #[serial]
    fn test03_existe_remoto() {
        let mut config = Config {
            remotos: vec![],
            ramas: vec![],
        };

        //caso en el que config vacio, devulve false
        assert!(!config.existe_remote("origin"));

        let remote = RemoteInfo {
            nombre: "config".to_string(),
            url: "localhost:3000".to_string(),
        };

        config.remotos.push(remote);

        //coso tiene algo pero no lo que se busca
        assert!(!config.existe_remote("origin"));

        let remote = RemoteInfo {
            nombre: "origin".to_string(),
            url: "localhost:3000".to_string(),
        };

        config.remotos.push(remote);
        assert!(config.existe_remote("origin"));
    }
}
