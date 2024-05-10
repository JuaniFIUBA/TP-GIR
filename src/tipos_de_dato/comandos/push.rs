use super::set_upstream::SetUpstream;
use crate::tipos_de_dato::comando::Ejecutar;
use crate::tipos_de_dato::comandos::write_tree;
use crate::tipos_de_dato::comunicacion::Comunicacion;
use crate::tipos_de_dato::config::Config;
use crate::tipos_de_dato::logger::Logger;
use crate::tipos_de_dato::objetos::commit::CommitObj;
use crate::tipos_de_dato::objetos::tree::Tree;
use crate::tipos_de_dato::packfile::Packfile;

use crate::tipos_de_dato::referencia::Referencia;
use crate::utils;
use crate::utils::io;
use crate::utils::path_buf::obtener_nombre;

use std::collections::HashSet;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

const FLAG_SET_UPSTREAM: &str = "--set-upstream";
const FLAG_U: &str = "-u";
const GIR_PUSH: &str = "\tgir push <remoto> <rama-local>:<rama-remota>\n";
const GIR_PUSH_U: &str = "gir push --set-upstream/-u <nombre-remoto> <nombre-rama-local>";
pub struct Push {
    referencia: Referencia,
    remoto: String,
    set_upstream: bool,
    logger: Arc<Logger>,
}

impl Push {
    pub fn new(args: &mut Vec<String>, logger: Arc<Logger>) -> Result<Self, String> {
        Self::verificar_argumentos(args)?;

        let mut set_upstream = false;

        if Self::hay_flags(args) {
            Self::parsear_flags(args, &mut set_upstream)?;
        }

        let (remoto, referencia) = Self::parsear_argumentos(args, set_upstream)?;

        Ok(Push {
            referencia,
            remoto,
            set_upstream,
            logger,
        })
    }

    fn verificar_argumentos(args: &Vec<String>) -> Result<(), String> {
        if args.len() > 3 {
            return Err(format!(
                "Parametros desconocidos {}\n {}",
                args.join(" "),
                GIR_PUSH
            ));
        };
        Ok(())
    }

    fn hay_flags(args: &Vec<String>) -> bool {
        args.len() == 3
    }

    ///obtiene el remoto  y la referencia asosiado a la rama remota actual. Falla si no existe
    fn obtener_remoto_y_rama_merge_de_rama_actual() -> Result<(String, Referencia), String> {
        let (remoto, rama_merge) = Config::leer_config()?
            .obtener_remoto_y_rama_merge_rama_actual()
            .ok_or(format!(
                "La rama actual no se encuentra asosiado a ningun remoto\nUtilice: {}",
                GIR_PUSH_U
            ))?;

        let rama_local_y_rama_remota = format!(
            "{}:{}",
            utils::ramas::obtener_rama_actual()?,
            obtener_nombre(&rama_merge)?
        );

        let referencia = Self::obtener_y_verificar_referencia(rama_local_y_rama_remota)?;
        Ok((remoto, referencia))
    }
    ///Obtiene acorde a los argumentos recibidos, el remoto y la rama merge. En caso de no estar,
    /// busca si esta seteada la rama actual. Si esto no es asi, hay un error
    fn parsear_argumentos(
        args: &mut Vec<String>,
        set_upstream: bool,
    ) -> Result<(String, Referencia), String> {
        let remoto;
        let referencia;

        if args.len() == 2 {
            remoto = Self::verificar_remoto(&args[0])?;
            referencia = Self::obtener_y_verificar_referencia(args.remove(1))?;
        } else if args.len() == 1 {
            //si solo esta el remoto entonces se presupone que se quiere enviar la
            //rama actual
            remoto = Self::verificar_remoto(&args[0])?;
            referencia =
                Self::obtener_y_verificar_referencia(utils::ramas::obtener_rama_actual()?)?;
        } else if args.is_empty() && !set_upstream {
            //si no hay argumentos ni flags, quiere decir que deberia
            //estar configurada la rama
            (remoto, referencia) = Self::obtener_remoto_y_rama_merge_de_rama_actual()?;
        } else {
            return Err(format!(
                "Parametros faltantes {}\n {}",
                args.join(" "),
                GIR_PUSH
            ));
        }

        Ok((remoto, referencia))
    }

    fn obtener_y_verificar_referencia(referencia: String) -> Result<Referencia, String> {
        Referencia::from(referencia.clone()).ok_or(format!(
            "Referencia desconidida: {}\n{}",
            referencia, GIR_PUSH
        ))
    }

    fn parsear_flags(args: &mut Vec<String>, set_upstream: &mut bool) -> Result<(), String> {
        //busca en los argumentos si hay flag y devuelve el indice
        if let Some(index_flag) = args.iter().position(|s| s.starts_with('-')) {
            let flag = args.remove(index_flag);

            if flag == FLAG_U || flag == FLAG_SET_UPSTREAM {
                *set_upstream = true;
                Ok(())
            } else {
                Err(format!(
                    "Parametros desconocidos {}\n {}",
                    args.join(" "),
                    GIR_PUSH
                ))
            }
        } else {
            Ok(())
        }
    }

    fn verificar_remoto(remoto: &str) -> Result<String, String> {
        if let false = Config::leer_config()?.existe_remote(remoto) {
            return  Err(format!("Remoto desconocido{}\nSi quiere añadir un nuevo remoto:\n\ngir remote add [<nombre-remote>] [<url-remote>]\n\n", remoto));
        };

        Ok(remoto.to_owned())
    }

    //Le pide al config el url asosiado a la rama
    fn obtener_url(&self, remoto: &str) -> Result<String, String> {
        Config::leer_config()?.obtenet_url_asosiado_remoto(remoto)
    }
    ///Inica la comunicacion con el servidor y el protocolo git-recive-pack
    ///
    /// # Resultado
    /// - Devuelve la Comunicacion establecida con el server
    fn iniciar_git_recive_pack_con_servidor(&self) -> Result<Comunicacion<TcpStream>, String> {
        let mut comunicacion = Comunicacion::<TcpStream>::new_desde_url(
            &self.obtener_url(&self.remoto)?,
            self.logger.clone(),
        )?;
        comunicacion.iniciar_git_recive_pack_con_servidor()?;
        Ok(comunicacion)
    }

    ///termina la comunicacion con el servidor, mandando un flush pkt y deespues un pack file vacio
    fn terminar_y_mandar_pack_file_vacio(
        &self,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<(), String> {
        comunicacion.enviar_flush_pkt()?;
        // el server pide que se le mande un packfile vacio
        comunicacion.enviar_pack_file(Packfile::obtener_pack_con_archivos(
            vec![],
            "./.gir/objects/",
        )?)
    }

    fn es_necesario_actualizar(&self, referencia_actualizar: &(String, String, PathBuf)) -> bool {
        referencia_actualizar.0 != referencia_actualizar.1
    }

    //obtiene todo los objetos de una referencia hasta el viejo commit. Si no esta el viejo commit entonces termina la comunicacion
    //y envia un pack file vacio
    fn obtener_objetos_a_enviar(
        &self,
        referencia: &Path,
        viejo_commit: &str,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<HashSet<String>, String> {
        let objetos_a_enviar =
            obtener_commits_y_objetos_asociados(referencia, viejo_commit, self.logger.clone());

        match objetos_a_enviar {
            Ok(objetos_a_enviar) => Ok(objetos_a_enviar),
            Err(msj_err) => {
                //error
                self.terminar_y_mandar_pack_file_vacio(comunicacion)?;
                Err(msj_err)
            }
        }
    }

    ///Obtiene la referecia que hay que actulizar del servidor y todos sus componentes(viejo commit, nuevo commit y ref).
    /// Para obtener el viejo commit compara el nombre de la ref con los ref ramas recibidos del servidor y lo busca.
    /// Si no existe viejo commit se completa con ceros (00..00).
    ///
    /// ## Argumentos
    /// -   commits_y_refs_asosiados: vector de tuplas de commit y su ref asosiado
    ///
    /// ## Resultado
    /// - Tupla con:
    ///     - el commit viejo
    ///     - el commit nuevo
    ///     - la ref
    fn obtener_referencia_acualizar(
        &self,
        commits_y_refs_asosiado: &Vec<(String, PathBuf)>,
    ) -> Result<(String, String, PathBuf), String> {
        let (mut commit_viejo, commit_nuevo, nombre_referencia) = self.obtener_referencia()?;

        for (commit, referencia) in commits_y_refs_asosiado {
            if *referencia == nombre_referencia {
                commit_viejo = commit.to_string();
            }
        }

        self.logger.log(&format!(
            "Referencia actualizar: {} {} {:?}",
            commit_viejo, commit_nuevo, nombre_referencia
        ));
        Ok((commit_viejo, commit_nuevo, nombre_referencia))
    }

    ///obtiene la una referencia apartir de los parametros recividos en la creacion.
    ///
    /// # Resultado
    ///
    /// - referencia = commit-viejo(siempre igual 0*40 en este caso), commit-nuevo(el commit
    ///                 de la referenca acutal), la referencia actualizar en el servidor(puede ser
    ///                 una rama (Ej: refs/heads/trabajo) o un tag (Ej: refs/tags/v1.0))
    fn obtener_referencia(&self) -> Result<(String, String, PathBuf), String> {
        let commit_viejo = "0".repeat(40);
        let nombre_referencia = self.referencia.dar_ref_remota();
        let commit_nuevo =
            io::leer_a_string(PathBuf::from("./.gir").join(self.referencia.dar_ref_local()))?;
        Ok((commit_viejo, commit_nuevo, nombre_referencia))
    }

    ///Le envia las referencia a acualizar al servidor junto con todos sus objetos asosiados
    /// dentro del pack file. Finaliza la comunicacion
    fn enviar_actualizaciones_y_objetos(
        &self,
        referencia_actualizar: (String, String, PathBuf),
        objetos_a_enviar: HashSet<String>,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<(), String> {
        self.logger.log(&format!(
            "Se envia en push la referencia: {:?}",
            referencia_actualizar
        ));

        comunicacion.enviar_referencia(referencia_actualizar)?;

        self.logger.log(&format!(
            "Se envia en push los objetos: {:?}",
            objetos_a_enviar
        ));

        comunicacion.enviar_pack_file(Packfile::obtener_pack_con_archivos(
            objetos_a_enviar.into_iter().collect(),
            "./.gir/objects/",
        )?)?;
        Ok(())
    }

    ///Se encarga de la fase de descubrimiento con el servidor, en la cual se recibe del servidor
    /// una lista de referencias.
    /// La primera linea contiene la version del server
    /// La segunda linea recibida tiene el siguiente : 'hash_del_commit_head HEAD'\0'lista de capacida'
    /// Las siguients lineas: 'hash_del_commit_cabeza_de_rama_en_el_servidor'
    ///                        'direccion de la carpeta de la rama en el servidor'
    ///
    /// # Resultado
    ///
    /// - vector de tuplas con los commit cabeza de rama y la ref de la
    ///     del tag o la rama oen el servidor(ojo!! la direccion para el servidor no para el local)
    fn fase_de_descubrimiento(
        &self,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<Vec<(String, PathBuf)>, String> {
        let (
            _capacidades_servidor,
            _commit_head_remoto,
            commits_cabezas_y_ref_rama_asosiado,
            commits_y_tags_asosiados,
        ) = utils::fase_descubrimiento::fase_de_descubrimiento(comunicacion)?;

        self.logger.log("Fase de descubrimiento ejecuta con exito");

        Ok([
            &commits_cabezas_y_ref_rama_asosiado[..],
            &commits_y_tags_asosiados[..],
        ]
        .concat())
    }
}
// funcion para obtener los commits que faltan para llegar al commit limite y los objetos asociados a cada commit
// en caso de que sea una referencia nula, se enviara todo. En caso de que el commit limite no sea una referencia nula
// y no se encuentre al final de la cadena de commits, se enviara un error, ya que el servidor tiene cambios que el cliente no tiene
fn obtener_commits_y_objetos_asociados(
    referencia: &Path,
    commit_limite: &str,
    logger: Arc<Logger>,
) -> Result<HashSet<String>, String> {
    //let ruta = format!(".gir/{}", referencia);
    let ruta = format!(".gir/{}", referencia.to_string_lossy());
    let ultimo_commit = io::leer_a_string(Path::new(&ruta))?;
    if ultimo_commit.is_empty() {
        return Ok(HashSet::new());
    }

    // let mut objetos_a_agregar: HashMap<String, CommitObj> = HashMap::new();
    let mut objetos_a_agregar: HashSet<String> = HashSet::new();
    let mut commits_a_revisar: Vec<CommitObj> = Vec::new();

    let ultimo_commit = CommitObj::from_hash(ultimo_commit, logger.clone());

    match ultimo_commit {
        Ok(ultimo_commit) => {
            commits_a_revisar.push(ultimo_commit);
        }
        Err(_) => {
            return Err(
                "El servidor tiene cambios, por favor, actualice su repositorio".to_string(),
            );
        }
    }

    while let Some(commit) = commits_a_revisar.pop() {
        if objetos_a_agregar.contains(&commit.hash) {
            continue;
        }
        if commit.hash == commit_limite {
            objetos_a_agregar.insert(commit.hash.clone());
            break;
        }
        objetos_a_agregar.insert(commit.hash.clone());
        let hash_tree = write_tree::conseguir_arbol_en_directorio(&commit.hash, "./.gir/objects/")?;
        let tree = Tree::from_hash(&hash_tree, PathBuf::from("."), logger.clone())?;
        objetos_a_agregar.insert(hash_tree.clone());
        objetos_a_agregar.extend(
            tree.obtener_objetos()
                .iter()
                .map(|objeto| objeto.obtener_hash()),
        );

        for padre in commit.padres {
            let commit_padre = CommitObj::from_hash(padre, logger.clone())?;
            commits_a_revisar.push(commit_padre);
        }
    }
    if ("0".repeat(40) != *commit_limite) && !objetos_a_agregar.contains(commit_limite) {
        return Err("El servidor tiene cambios, por favor, actualice su repositorio".to_string());
    } else if ("0".repeat(40) != *commit_limite) && objetos_a_agregar.contains(commit_limite) {
        objetos_a_agregar.remove(commit_limite);
    }
    Ok(objetos_a_agregar)
}

impl Ejecutar for Push {
    fn ejecutar(&mut self) -> Result<String, String> {
        let mut comunicacion = self.iniciar_git_recive_pack_con_servidor()?;

        let commits_y_refs_asosiado = self.fase_de_descubrimiento(&mut comunicacion)?;

        let referencia_acualizar = self.obtener_referencia_acualizar(&commits_y_refs_asosiado)?;

        let mensaje = if self.es_necesario_actualizar(&referencia_acualizar) {
            let objetos_a_enviar = self.obtener_objetos_a_enviar(
                &self.referencia.dar_ref_local(),
                &referencia_acualizar.0,
                &mut comunicacion,
            )?;

            self.enviar_actualizaciones_y_objetos(
                referencia_acualizar,
                objetos_a_enviar,
                &mut comunicacion,
            )?;
            "Push ejecutado con exito".to_string()
        } else {
            self.terminar_y_mandar_pack_file_vacio(&mut comunicacion)?;
            "Nada que actualizar".to_string()
        };

        if self.set_upstream && !self.referencia.es_tag() {
            SetUpstream::new(
                self.remoto.clone(),
                self.referencia.dar_nombre_remoto(),
                self.referencia.dar_nombre_local(),
                self.logger.clone(),
            )?
            .ejecutar()?;
        }

        self.logger.log(&mensaje);
        Ok(mensaje)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{comando::Ejecutar, comandos::set_upstream::SetUpstream, logger::Logger},
        utils,
    };

    use super::Push;

    #[test]
    #[serial]
    fn test_01_se_crea_bien_la_referencia_actualizar_al_poner_rama_local_y_remota() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_01")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());
        let rama_local = "minas".to_string();
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_head_rama".to_string();

        utils::testing::escribir_rama_local(&rama_local, logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::io::escribir_bytes(format!("./.gir/refs/heads/{}", rama_local), commit.clone())
            .unwrap();

        let referencia_esperada = (
            "0".repeat(40),
            commit,
            PathBuf::from(format!("refs/heads/{}", rama_local)),
        );
        let referencia = Push::new(&mut vec![remoto, rama_local], logger)
            .unwrap()
            .obtener_referencia()
            .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }

    #[test]
    #[serial]
    fn test_02_se_crea_bien_la_referencia_actualizar_al_poner_remoto_y_tag() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_02")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());
        let tag = "v1.0".to_string();
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_tag".to_string();

        utils::io::escribir_bytes(format!("./.gir/refs/tags/{}", tag), &commit).unwrap();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());

        let referencia_esperada = (
            "0".repeat(40),
            commit,
            PathBuf::from(format!("refs/tags/{}", tag)),
        );
        let referencia = Push::new(&mut vec![remoto, tag], logger)
            .unwrap()
            .obtener_referencia()
            .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }

    #[test]
    #[serial]
    fn test_03_se_crea_bien_la_referencia_al_poner_solo_el_remoto() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_03")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_head_rama".to_string();
        let rama_actual = utils::ramas::obtener_rama_actual().unwrap();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::io::escribir_bytes(format!("./.gir/refs/heads/{}", rama_actual), commit.clone())
            .unwrap();

        let referencia_esperada = (
            "0".repeat(40),
            commit,
            PathBuf::from(format!("refs/heads/{}", rama_actual)),
        );
        let referencia = Push::new(&mut vec![remoto], logger)
            .unwrap()
            .obtener_referencia()
            .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }

    #[test]
    #[serial]
    fn test_04_se_crea_bien_la_referencia_al_poner_el_remoto_rama_local_y_rama_remota() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_04")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let rama_local = "minas".to_string();
        let rama_remota = "27-poder-poner-minas-en-tablero".to_string();
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_head_rama".to_string();
        let rama_local_y_rama_remota = format!("{}:{}", rama_local, rama_remota);

        utils::testing::escribir_rama_local(&rama_local, logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::io::escribir_bytes(format!("./.gir/refs/heads/{}", rama_local), commit.clone())
            .unwrap();

        let referencia_esperada = (
            "0".repeat(40),
            commit,
            PathBuf::from(format!("refs/heads/{}", rama_remota)),
        );
        let referencia = Push::new(&mut vec![remoto, rama_local_y_rama_remota], logger)
            .unwrap()
            .obtener_referencia()
            .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }

    #[test]
    #[serial]
    fn test_05_se_crea_bien_la_referencia_al_poner_el_tag_local_y_tag_remoto() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_05")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let tag_local = "v1.0-trabajador".to_string();
        let tag_remoto = "v1.0-trabajo".to_string();
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_tag".to_string();
        let tag_local_y_tag_remota = format!("{}:{}", tag_local, tag_remoto);

        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::io::escribir_bytes(format!("./.gir/refs/tags/{}", tag_local), commit.clone())
            .unwrap();

        let referencia_esperada = (
            "0".repeat(40),
            commit,
            PathBuf::from(format!("refs/tags/{}", tag_remoto)),
        );
        let referencia = Push::new(&mut vec![remoto, tag_local_y_tag_remota], logger)
            .unwrap()
            .obtener_referencia()
            .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }

    #[test]
    #[serial]
    fn test_06_se_crea_bien_la_referencia_al_no_poner_nada_si_esta_configurada_la_rama() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_06")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_head_rama".to_string();
        let rama_actual = utils::ramas::obtener_rama_actual().unwrap();
        let rama_remota = "28-poner-bombas".to_string();

        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::testing::escribir_rama_remota(&remoto, &rama_remota);
        utils::io::escribir_bytes(format!("./.gir/refs/heads/{}", rama_actual), commit.clone())
            .unwrap();
        SetUpstream::new(
            remoto.clone(),
            rama_remota.clone(),
            rama_actual.clone(),
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let referencia_esperada = (
            "0".repeat(40),
            commit,
            PathBuf::from(format!("refs/heads/{}", rama_remota)),
        );
        let referencia = Push::new(&mut vec![], logger)
            .unwrap()
            .obtener_referencia()
            .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }

    #[test]
    #[serial]
    fn test_07_se_crea_bien_la_referencia_a_la_rama_aun_con_el_flag_u() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_07")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let rama_local = "minas".to_string();
        let rama_remota = "27-poder-poner-minas-en-tablero".to_string();
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_head_rama".to_string();
        let rama_local_y_rama_remota = format!("{}:{}", rama_local, rama_remota);

        utils::testing::escribir_rama_local(&rama_local, logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::io::escribir_bytes(format!("./.gir/refs/heads/{}", rama_local), commit.clone())
            .unwrap();

        let referencia_esperada = (
            "0".repeat(40),
            commit,
            PathBuf::from(format!("refs/heads/{}", rama_remota)),
        );
        let mut referencia = Push::new(
            &mut vec![
                "-u".to_string(),
                remoto.clone(),
                rama_local_y_rama_remota.clone(),
            ],
            logger.clone(),
        )
        .unwrap()
        .obtener_referencia()
        .unwrap();

        assert_eq!(referencia_esperada, referencia);

        referencia = Push::new(
            &mut vec![remoto, rama_local_y_rama_remota, "-u".to_string()],
            logger,
        )
        .unwrap()
        .obtener_referencia()
        .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }

    #[test]
    #[serial]
    fn test_08_se_crea_bien_la_referencia_actualizar() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/push_01")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());
        let rama_local = "minas".to_string();
        let remoto = "buscaminas-rustico".to_string();
        let commit = "commit_head_rama".to_string();
        let commit_viejo = "commit_viejo".to_string();

        utils::testing::escribir_rama_local(&rama_local, logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::io::escribir_bytes(format!("./.gir/refs/heads/{}", rama_local), commit.clone())
            .unwrap();

        let referencia_esperada = (
            commit_viejo.clone(),
            commit,
            PathBuf::from(format!("refs/heads/{}", rama_local)),
        );

        let commit_y_refs_asosiado = vec![
            (
                "No importa".to_string(),
                PathBuf::from("refs/heads/no-importa"),
            ),
            (
                "No importaa 2".to_string(),
                PathBuf::from(format!("refs/tags/{}", rama_local)),
            ),
            (
                commit_viejo,
                PathBuf::from(format!("refs/heads/{}", rama_local)),
            ),
        ];
        let referencia = Push::new(&mut vec![remoto, rama_local], logger)
            .unwrap()
            .obtener_referencia_acualizar(&commit_y_refs_asosiado)
            .unwrap();

        assert_eq!(referencia_esperada, referencia);
    }
}
