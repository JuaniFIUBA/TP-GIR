use crate::tipos_de_dato::comando::Ejecutar;
use crate::tipos_de_dato::comunicacion::Comunicacion;
use crate::tipos_de_dato::config::Config;
use crate::tipos_de_dato::logger::Logger;
use crate::tipos_de_dato::packfile::Packfile;
use crate::tipos_de_dato::referencia_commit::ReferenciaCommit;
use crate::utils::{self, io, objects};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

const SE_ENVIO_ALGUN_PEDIDO: bool = true;
const NO_SE_ENVIO_NINGUN_PEDIDO: bool = false;
const GIR_FETCH: &str = "gir fetch <remoto>";

pub struct Fetch {
    remoto: String,
    capacidades_local: Vec<String>,
    logger: Arc<Logger>,
}

impl Fetch {
    pub fn new(args: Vec<String>, logger: Arc<Logger>) -> Result<Fetch, String> {
        Self::verificar_argumentos(&args)?;

        let remoto = Self::obtener_remoto(args)?;

        let capacidades_local = vec!["ofs-delta".to_string()];
        //esto lo deberia tener la comunicacion creo yo

        Ok(Fetch {
            remoto,
            capacidades_local,
            logger,
        })
    }

    fn verificar_argumentos(args: &Vec<String>) -> Result<(), String> {
        if args.len() > 1 {
            return Err(format!(
                "Parametros desconocidos {}\n {}",
                args.join(" "),
                GIR_FETCH
            ));
        };
        Ok(())
    }

    ///Le pide al config el url asosiado a la rama
    fn obtener_url(&self, remoto: &str) -> Result<String, String> {
        Config::leer_config()?.obtenet_url_asosiado_remoto(remoto)
    }

    ///obtiene el remoto para el comando, si argumentos lo contiene y es valido lo saca de argumentos. Si no hay argumetos lo saca
    /// del remoto asosiado a la rama actual. Si no esta configura la rama actual para ningun remoto devuleve error.
    fn obtener_remoto(args: Vec<String>) -> Result<String, String> {
        let remoto = if args.len() == 1 {
            Self::verificar_remoto(&args[0])?
        } else {
            Self::obtener_remoto_rama_actual()?
        };
        Ok(remoto)
    }

    ///verifica si el remoto envio por el usario existe
    fn verificar_remoto(remoto: &str) -> Result<String, String> {
        if let false = Config::leer_config()?.existe_remote(remoto) {
            return  Err(format!("Remoto desconocido{}\nSi quiere añadir un nuevo remoto:\n\ngir remote add [<nombre-remote>] [<url-remote>]\n\n", remoto));
        };

        Ok(remoto.to_string())
    }

    ///obtiene el remo asosiado a la rama remota actual. Falla si no existe
    fn obtener_remoto_rama_actual() -> Result<String, String> {
        Config::leer_config()?
            .obtener_remoto_rama_actual()
            .ok_or(format!(
                "La rama actual no se encuentra asosiado a ningun remoto\nUtilice:\n\ngir remote add [<nombre-remote>] [<url-remote>]\n\nDespues:\n\n{}\n\n", GIR_FETCH
            ))
    }

    fn guardar_los_tags(
        &self,
        commits_y_tags_asosiados: &Vec<(String, PathBuf)>,
    ) -> Result<(), String> {
        for (commit, ref_tag) in commits_y_tags_asosiados {
            let dir_tag = PathBuf::from("./.gir/").join(ref_tag);
            utils::io::escribir_bytes(dir_tag, commit)?
        }

        self.logger.log("Escritura de tags en fetch exitosa");
        Ok(())
    }

    fn fase_de_negociacion(
        &self,
        capacidades_servidor: Vec<String>,
        commits_cabezas_y_dir_rama_asosiado: &Vec<(String, PathBuf)>,
        commit_y_tags_asosiado: &Vec<(String, PathBuf)>,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<bool, String> {
        // no hay pedidos :D
        if !self.enviar_pedidos(
            &capacidades_servidor,
            commits_cabezas_y_dir_rama_asosiado,
            commit_y_tags_asosiado,
            comunicacion,
        )? {
            return Ok(NO_SE_ENVIO_NINGUN_PEDIDO);
        }

        self.enviar_lo_que_tengo(comunicacion)?;

        self.logger
            .log("Se completo correctamente la fase de negociacion en Fetch");
        Ok(SE_ENVIO_ALGUN_PEDIDO)
    }

    fn recibir_packfile_y_guardar_objetos(
        &self,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<(), String> {
        self.logger.log("Obteniendo paquete..");

        let packfile = comunicacion.obtener_packfile()?;
        let primeros_bytes = &packfile[..4];
        if primeros_bytes != "PACK".as_bytes() {
            self.logger.log(&format!(
                "Se recibio: {}",
                String::from_utf8_lossy(packfile.as_slice())
            ));

            return Err(format!(
                "Error al recibir el packfile, se recibio: {}",
                String::from_utf8_lossy(packfile.as_slice())
            ));
        }
        self.logger.log("Recepcion del pack file en fetch exitoso");
        Packfile::leer_packfile_y_escribir(&packfile, "./.gir/objects/".to_string()).unwrap();
        Ok(())
    }

    ///Envia un mensaje al servidor para avisarle que ya se termino de de mandarle lineas.
    /// Para seguir el protocolo el mensaje que se envia es done
    fn finalizar_pedido(&self, comunicacion: &mut Comunicacion<TcpStream>) -> Result<(), String> {
        comunicacion.enviar(&utils::strings::obtener_linea_con_largo_hex("done\n"))
    }

    ///Actuliza el archivo head correspondiente al remoto que se hizo fetch o si no existe lo crea.
    /// Si se hizo fetch del remoto 'san_lorenzo' -> se actuliza o crea el archivo `SAN_LORENZO_HEAD`
    /// con el commit hash cabeza recibido del servidor    
    fn acutualizar_archivo_head_remoto(
        &self,
        commit_head_remoto: &Option<String>,
    ) -> Result<(), String> {
        if let Some(hash) = commit_head_remoto {
            let ubicacion_archivo_head_remoto =
                format!("./.gir/{}_HEAD", self.remoto.to_uppercase());

            println!(
                "ubicacion_archivo_head_remoto: {}",
                ubicacion_archivo_head_remoto
            );
            io::escribir_bytes(ubicacion_archivo_head_remoto, hash)?;
        }

        Ok(())
    }

    ///Envia todo los objetos (sus hash) que ya se tienen y por lo tanto no es necesario que el servidor manda
    fn enviar_lo_que_tengo(
        &self,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<(), String> {
        let objetos = objects::obtener_objetos_del_dir(&PathBuf::from("./.gir/objects"))?;

        if !objetos.is_empty() {
            comunicacion.enviar_lo_que_tengo_al_servidor_pkt(&objetos)?;
            self.recivir_nack(comunicacion)?;
            self.finalizar_pedido(comunicacion)?
        } else {
            self.finalizar_pedido(comunicacion)?;
            self.recivir_nack(comunicacion)?;
        }
        self.logger.log("Se envio con exito lo que tengo en Fetch");
        Ok(())
    }

    ///Recibe el la repusta Nack del servidor del envio de HAVE
    fn recivir_nack(&self, comunicacion: &mut Comunicacion<TcpStream>) -> Result<(), String> {
        let _acks_nak = comunicacion.obtener_lineas()?;
        Ok(())
    }

    ///Envia al servidor todos los commits cabeza de rama que se quieren actulizar junto con las capacidades del
    /// servidor.
    /// La operacion devulve un booleando que dice si se mando o no algun pedido. En caso de enviar algun pedido
    /// se devuelve true, en caso de no enviar ninigun pedido(es decir no se quiere nada del server) se devuelve
    /// false
    fn enviar_pedidos(
        &self,
        capacidades_servidor: &[String],
        commits_cabezas_y_dir_rama_asosiado: &Vec<(String, PathBuf)>,
        commit_y_tags_asosiado: &Vec<(String, PathBuf)>,
        comunicacion: &mut Comunicacion<TcpStream>,
    ) -> Result<bool, String> {
        let capacidades_a_usar_en_la_comunicacion =
            self.obtener_capacidades_en_comun_con_el_servidor(capacidades_servidor);

        let commits_de_cabeza_de_rama_faltantes =
            self.obtener_commits_cabeza_de_rama_faltantes(commits_cabezas_y_dir_rama_asosiado)?;
        let tags_faltantes = self.obtener_tags_faltantes(commit_y_tags_asosiado)?;

        let pedidos = [
            &commits_de_cabeza_de_rama_faltantes[..],
            &tags_faltantes[..],
        ]
        .concat();

        if pedidos.is_empty() {
            comunicacion.enviar_flush_pkt()?;
            self.logger.log(
                "Se completo correctamente el envio de pedidos en Fetch pero no se envio nada",
            );
            return Ok(NO_SE_ENVIO_NINGUN_PEDIDO);
        }

        comunicacion
            .enviar_pedidos_al_servidor_pkt(pedidos, capacidades_a_usar_en_la_comunicacion)?;

        self.logger
            .log("Se completo correctamente el envio de pedidos en Fetch");
        Ok(SE_ENVIO_ALGUN_PEDIDO)
    }

    ///Obtiene los commits que son necesarios a actulizar y por lo tanto hay que pedirle al servidor esas ramas.
    /// Obtiene aquellos commits que pertenecesen a ramas cuyas cabezas en el servidor apuntan commits distintos
    /// que sus equivalencias en el repositorio local, implicando que la rama local esta desacululizada.
    ///
    /// # Resultado
    ///
    /// - Devuleve un vector con los commits cabezas de las ramas que son necearias actualizar con
    ///     respecto a las del servidor
    fn obtener_commits_cabeza_de_rama_faltantes(
        &self,
        commits_cabezas_y_dir_rama_asosiado: &Vec<(String, PathBuf)>,
    ) -> Result<Vec<String>, String> {
        let mut commits_de_cabeza_de_rama_faltantes: Vec<String> = Vec::new();

        for (commit_cabeza_remoto, dir_rama_asosiada) in commits_cabezas_y_dir_rama_asosiado {
            let dir_rama_asosiada_local =
                utils::ramas::convertir_de_dir_rama_remota_a_dir_rama_local(
                    &self.remoto,
                    dir_rama_asosiada,
                )?;

            if !dir_rama_asosiada_local.exists() {
                commits_de_cabeza_de_rama_faltantes.push(commit_cabeza_remoto.to_string());
                continue;
            }
            let commit_cabeza_local = io::leer_a_string(dir_rama_asosiada_local)?;

            if commit_cabeza_local != *commit_cabeza_remoto {
                commits_de_cabeza_de_rama_faltantes.push(commit_cabeza_remoto.to_string());
            }
        }

        self.logger.log(&format!(
            "Commits ramas faltantes {:?}",
            commits_de_cabeza_de_rama_faltantes
        ));

        Ok(commits_de_cabeza_de_rama_faltantes)
    }

    ///Obtiene los commits que son necesarios a actulizar y por lo tanto hay que pedirle al servidor esas ramas.
    /// Obtiene aquellos commits que pertenecesen a ramas cuyas cabezas en el servidor apuntan commits distintos
    /// que sus equivalencias en el repositorio local, implicando que la rama local esta desacululizada.
    ///
    /// # Resultado
    ///
    /// - Devuleve un vector con los commits cabezas de las ramas que son necearias actualizar con
    ///     respecto a las del servidor
    fn obtener_tags_faltantes(
        &self,
        commit_y_tags_asosiado: &Vec<(String, PathBuf)>,
    ) -> Result<Vec<String>, String> {
        let mut commits_de_tags_faltantes: Vec<String> = Vec::new();

        for (commit_cabeza_remoto, tag_asosiado) in commit_y_tags_asosiado {
            let dir_tag = PathBuf::from("./.gir").join(tag_asosiado);

            if !dir_tag.exists() {
                commits_de_tags_faltantes.push(commit_cabeza_remoto.to_string());
                continue;
            }
            let commit_cabeza_local = io::leer_a_string(dir_tag)?;

            if commit_cabeza_local != *commit_cabeza_remoto {
                commits_de_tags_faltantes.push(commit_cabeza_remoto.to_string());
            }
        }

        self.logger.log(&format!(
            "Commits tags faltantes {:?}",
            commits_de_tags_faltantes
        ));

        Ok(commits_de_tags_faltantes)
    }
    ///compara las capacidades del servidor con las locales y devulve un string con las capacidades en comun
    /// para usar en la comunicacion
    fn obtener_capacidades_en_comun_con_el_servidor(
        &self,
        capacidades_servidor: &[String],
    ) -> String {
        let mut capacidades_a_usar_en_la_comunicacion: Vec<&str> = Vec::new();

        capacidades_servidor.iter().for_each(|capacidad| {
            if self.capacidades_local.contains(&capacidad.to_string()) {
                capacidades_a_usar_en_la_comunicacion.push(capacidad);
            }
        });

        capacidades_a_usar_en_la_comunicacion.join(" ")
    }
    ///Se encarga de la fase de descubrimiento con el servidor, en la cual se recibe del servidor
    /// una lista de referencias.
    /// La primera linea contiene la version del server
    /// La segunda linea recibida tiene el siguiente : 'hash_del_commit_head HEAD'\0'lista de capacida'
    /// Las siguients lineas: 'hash_del_commit_cabeza_de_rama_en_el_servidor'
    ///                        'direccion de la carpeta de la rama en el servidor'
    ///
    /// # Resultado
    /// - vector con las capacidades del servidor
    /// - hash del commit cabeza de rama
    /// - vector de tuplas con los hash del commit cabeza de rama y la direccion de la
    ///     carpeta de la rama en el servidor(ojo!! la direccion para el servidor no para el local)
    /// - vector de tuplas con el hash del commit y el tag asosiado
    fn fase_de_descubrimiento<T: Write + Read>(
        &self,
        comunicacion: &mut Comunicacion<T>,
    ) -> Result<
        (
            Vec<String>,
            Option<String>,
            ReferenciaCommit,
            ReferenciaCommit,
        ),
        String,
    > {
        let resultado = utils::fase_descubrimiento::fase_de_descubrimiento(comunicacion)?;

        self.logger.log(&format!(
            "Se ejecuto correctamte la fase de decubrimiento en Fetch: {:?}",
            resultado
        ));

        Ok(resultado)
    }

    ///actuliza a donde apuntan las cabeza del rama de las ramas locales pertenecientes al remoto
    fn actualizar_ramas_locales_del_remoto(
        &self,
        commits_cabezas_y_dir_rama_asosiado: &Vec<(String, PathBuf)>,
    ) -> Result<(), String> {
        for (commit_cabeza_de_rama, dir_rama_remota) in commits_cabezas_y_dir_rama_asosiado {
            let dir_rama_local_del_remoto =
                utils::ramas::convertir_de_dir_rama_remota_a_dir_rama_local(
                    &self.remoto,
                    dir_rama_remota,
                )?;

            io::escribir_bytes(dir_rama_local_del_remoto, commit_cabeza_de_rama)?;
        }

        self.logger
            .log("Actualizacion de ramas remotas en fetch exitosa");
        Ok(())
    }

    fn iniciar_git_upload_pack_con_servidor(&self) -> Result<Comunicacion<TcpStream>, String> {
        let url = self.obtener_url(&self.remoto)?;
        let mut comunicacion = Comunicacion::<TcpStream>::new_desde_url(&url, self.logger.clone())?;
        comunicacion.iniciar_git_upload_pack_con_servidor()?;
        Ok(comunicacion)
    }
}

impl Ejecutar for Fetch {
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Se ejecuto el comando fetch");
        let mut comunicacion = self.iniciar_git_upload_pack_con_servidor()?;

        let (
            capacidades_servidor,
            commit_head_remoto,
            commits_cabezas_y_dir_rama_asosiado,
            commits_y_tags_asosiados,
        ) = self.fase_de_descubrimiento(&mut comunicacion)?;

        if !self.fase_de_negociacion(
            capacidades_servidor,
            &commits_cabezas_y_dir_rama_asosiado,
            &commits_y_tags_asosiados,
            &mut comunicacion,
        )? {
            return Ok(String::from("El cliente esta actualizado"));
        }

        self.recibir_packfile_y_guardar_objetos(&mut comunicacion)?;

        self.actualizar_ramas_locales_del_remoto(&commits_cabezas_y_dir_rama_asosiado)?;

        self.guardar_los_tags(&commits_y_tags_asosiados)?;

        self.acutualizar_archivo_head_remoto(&commit_head_remoto)?;

        let mensaje = "Fetch ejecutado con exito".to_string();
        self.logger.log(&mensaje);
        Ok(mensaje)
    }
}

#[cfg(test)]

mod tests {
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{comunicacion::Comunicacion, logger::Logger},
        utils::{self, testing::MockTcpStream},
    };

    use super::Fetch;
    #[test]
    fn test01_la_fase_de_descubrimiento_funcion() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_01.txt")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());
        let contenido_mock = "000eversion 1\n\
        00887217a7c7e582c46cec22a130adf4b9d7d950fba0 HEAD\0multi_ack thin-pack \
        side-band side-band-64k ofs-delta shallow no-progress include-tag \
        00441d3fcd5ced445d1abc402225c0b8a1299641f497 refs/heads/integration \
        003f7217a7c7e582c46cec22a130adf4b9d7d950fba0 refs/heads/master \
        003cb88d2441cac0977faf98efc80305012112238d9d refs/tags/v0.9 \
        003c525128480b96c89e6418b1e40909bf6c5b2d580f refs/tags/v1.0 \
        003fe92df48743b7bc7d26bcaabfddde0a1e20cae47c refs/tags/v1.0^{} \
        0000";

        let mock = MockTcpStream {
            lectura_data: contenido_mock.as_bytes().to_vec(),
            escritura_data: Vec::new(),
        };

        let mut comunicacion = Comunicacion::new_para_testing(mock, logger.clone());
        let remoto = "origin".to_string();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        let (capacidades, commit_head, commits_y_ramas, commits_y_tags) =
            Fetch::new(vec![remoto], logger)
                .unwrap()
                .fase_de_descubrimiento(&mut comunicacion)
                .unwrap();

        let capacidades_esperadas =
            "multi_ack thin-pack side-band side-band-64k ofs-delta shallow no-progress include-tag";
        assert_eq!(capacidades_esperadas, capacidades.join(" "));

        let commit_head_esperado = "7217a7c7e582c46cec22a130adf4b9d7d950fba0";
        assert_eq!(commit_head_esperado, commit_head.unwrap());

        let commits_y_ramas_esperadas = vec![
            (
                "1d3fcd5ced445d1abc402225c0b8a1299641f497".to_string(),
                PathBuf::from("refs/heads/integration"),
            ),
            (
                "7217a7c7e582c46cec22a130adf4b9d7d950fba0".to_string(),
                PathBuf::from("refs/heads/master"),
            ),
        ];
        assert_eq!(commits_y_ramas_esperadas, commits_y_ramas);

        let commits_y_tags_esperados = vec![
            (
                "b88d2441cac0977faf98efc80305012112238d9d".to_string(),
                PathBuf::from("refs/tags/v0.9"),
            ),
            (
                "525128480b96c89e6418b1e40909bf6c5b2d580f".to_string(),
                PathBuf::from("refs/tags/v1.0"),
            ),
            (
                "e92df48743b7bc7d26bcaabfddde0a1e20cae47c".to_string(),
                PathBuf::from("refs/tags/v1.0^{}".to_string()),
            ),
        ];
        assert_eq!(commits_y_tags_esperados, commits_y_tags)
    }

    #[test]
    fn test02_la_fase_de_descubrimiento_funcion_aun_si_no_hay_un_head() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_02.txt")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let contenido_mock = "000eversion 1\n\
        009a1d3fcd5ced445d1abc402225c0b8a1299641f497 refs/heads/integration\0multi_ack thin-pack \
        side-band side-band-64k ofs-delta shallow no-progress include-tag \
        003f7217a7c7e582c46cec22a130adf4b9d7d950fba0 refs/heads/master \
        003cb88d2441cac0977faf98efc80305012112238d9d refs/tags/v0.9 \
        003c525128480b96c89e6418b1e40909bf6c5b2d580f refs/tags/v1.0 \
        003fe92df48743b7bc7d26bcaabfddde0a1e20cae47c refs/tags/v1.0^{} \
        0000";

        let mock = MockTcpStream {
            lectura_data: contenido_mock.as_bytes().to_vec(),
            escritura_data: Vec::new(),
        };

        let mut comunicacion = Comunicacion::new_para_testing(mock, logger.clone());

        let remoto = "origin".to_string();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());

        let (capacidades, commit_head, commits_y_ramas, commits_y_tags) =
            Fetch::new(vec![remoto], logger)
                .unwrap()
                .fase_de_descubrimiento(&mut comunicacion)
                .unwrap();

        let capacidades_esperadas =
            "multi_ack thin-pack side-band side-band-64k ofs-delta shallow no-progress include-tag";
        assert_eq!(capacidades_esperadas, capacidades.join(" "));

        assert_eq!(Option::None, commit_head);

        let commits_y_ramas_esperadas = vec![
            (
                "1d3fcd5ced445d1abc402225c0b8a1299641f497".to_string(),
                PathBuf::from("refs/heads/integration"),
            ),
            (
                "7217a7c7e582c46cec22a130adf4b9d7d950fba0".to_string(),
                PathBuf::from("refs/heads/master"),
            ),
        ];
        assert_eq!(commits_y_ramas_esperadas, commits_y_ramas);

        let commits_y_tags_esperados = vec![
            (
                "b88d2441cac0977faf98efc80305012112238d9d".to_string(),
                PathBuf::from("refs/tags/v0.9"),
            ),
            (
                "525128480b96c89e6418b1e40909bf6c5b2d580f".to_string(),
                PathBuf::from("refs/tags/v1.0"),
            ),
            (
                "e92df48743b7bc7d26bcaabfddde0a1e20cae47c".to_string(),
                PathBuf::from("refs/tags/v1.0^{}".to_string()),
            ),
        ];
        assert_eq!(commits_y_tags_esperados, commits_y_tags)
    }

    #[test]
    #[serial]
    fn test_03_los_tags_se_gurdan_correctamtene() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_03.txt")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let tag_1 = "v0.9".to_string();
        let tag_1_contenido = "b88d2441cac0977faf98efc80305012112238d9d".to_string();
        let tag_2 = "v1.0".to_string();
        let tag_2_contenido = "525128480b96c89e6418b1e40909bf6c5b2d580f".to_string();

        let commits_y_tags = vec![
            (
                tag_1_contenido.clone(),
                PathBuf::from(format!("refs/tags/{}", tag_1)),
            ),
            (
                tag_2_contenido.clone(),
                PathBuf::from(format!("refs/tags/{}", tag_2)),
            ),
        ];

        let remoto = "origin".to_string();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());

        Fetch::new(vec![remoto], logger)
            .unwrap()
            .guardar_los_tags(&commits_y_tags)
            .unwrap();

        assert!(utils::tags::existe_tag(&tag_1));
        let tag_1_contenido_obtenido =
            utils::io::leer_a_string(format!("./.gir/refs/tags/{}", tag_1)).unwrap();
        assert_eq!(tag_1_contenido_obtenido, tag_1_contenido);

        assert!(utils::tags::existe_tag(&tag_2));
        let tag_2_contenido_obtenido =
            utils::io::leer_a_string(format!("./.gir/refs/tags/{}", tag_2)).unwrap();
        assert_eq!(tag_2_contenido_obtenido, tag_2_contenido);
    }

    #[test]
    #[serial]
    fn test_04_los_ramas_remotas_se_escriben_correctamente() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_04.txt")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let remoto = "san-siro".to_string();
        let rama_remota = "tomate".to_string();
        let rama_contenido = "b88d2441cac0977faf98efc80305012112238d9d".to_string();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());

        let commits_y_ramas = vec![(
            rama_contenido.clone(),
            PathBuf::from(format!("refs/heads/{}", rama_remota)),
        )];

        Fetch::new(vec![remoto.clone()], logger)
            .unwrap()
            .actualizar_ramas_locales_del_remoto(&commits_y_ramas)
            .unwrap();

        let rama_contendio_obtenido =
            utils::io::leer_a_string(format!("./.gir/refs/remotes/{}/{}", remoto, rama_remota))
                .unwrap();
        assert_eq!(rama_contendio_obtenido, rama_contenido);
    }

    #[test]
    #[serial]
    fn test_05_los_ramas_remotas_se_actualizan_correctamente() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_05.txt")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let remoto = "san-siro".to_string();
        let rama_remota = "tomate".to_string();
        let rama_contenido_actualizar = "b88d2441cac0977faf98efc80305012112238d9d".to_string();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());

        let commits_y_ramas = vec![(
            rama_contenido_actualizar.clone(),
            PathBuf::from(format!("refs/heads/{}", rama_remota)),
        )];

        Fetch::new(vec![remoto.clone()], logger)
            .unwrap()
            .actualizar_ramas_locales_del_remoto(&commits_y_ramas)
            .unwrap();

        let rama_contendio_obtenido =
            utils::io::leer_a_string(format!("./.gir/refs/remotes/{}/{}", remoto, rama_remota))
                .unwrap();
        assert_eq!(rama_contendio_obtenido, rama_contenido_actualizar);
    }

    #[test]
    #[serial]
    fn test_05_los_ramas_remotas_se_escriben_correctamente() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_05.txt")).unwrap());
        utils::testing::limpiar_archivo_gir(logger.clone());

        let remoto = "san-siro".to_string();
        let rama_remota = "tomate".to_string();
        let rama_contenido = "b88d2441cac0977faf98efc80305012112238d9d".to_string();
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());

        let commits_y_ramas = vec![(
            rama_contenido.clone(),
            PathBuf::from(format!("refs/heads/{}", rama_remota)),
        )];

        Fetch::new(vec![remoto.clone()], logger)
            .unwrap()
            .actualizar_ramas_locales_del_remoto(&commits_y_ramas)
            .unwrap();

        let rama_contendio_obtenido =
            utils::io::leer_a_string(format!("./.gir/refs/remotes/{}/{}", remoto, rama_remota))
                .unwrap();
        assert_eq!(rama_contendio_obtenido, rama_contenido);
    }
}
