use std::{path::PathBuf, sync::Arc};

use crate::{
    tipos_de_dato::{comando::Ejecutar, config::Config, info_ramas::RamasInfo, logger::Logger},
    utils,
};

pub struct SetUpstream {
    remoto: String,
    rama_remota: String,
    rama_local: String,
    logger: Arc<Logger>,
}

impl SetUpstream {
    pub fn new(
        remoto: String,
        rama_remota: String,
        rama_local: String,
        logger: Arc<Logger>,
    ) -> Result<SetUpstream, String> {
        logger.log(&format!(
            "Se crea set-upstream - remoto: {}, rama remota: {},rama local: {}",
            remoto, rama_remota, rama_remota
        ));

        Ok(SetUpstream {
            remoto,
            rama_remota,
            rama_local,
            logger,
        })
    }

    ///Setea la rama asosiadandola al remoto y seteando el campo de merge. Para ello escribie
    /// en el archivo config.
    /// En caso de que ya esta seteada, lo actualiza
    fn set_upstream(&self) -> Result<(), String> {
        let mut config = Config::leer_config()?;
        let merge = PathBuf::from(format!("refs/heads/{}", self.rama_remota));

        let nueva_config_rama = RamasInfo {
            nombre: self.rama_local.clone(),
            remote: self.remoto.clone(),
            merge,
        };

        let indice_resultado = config
            .ramas
            .iter()
            .position(|r| r.nombre == self.rama_local);

        match indice_resultado {
            Some(indice) => config.ramas[indice] = nueva_config_rama,
            None => config.ramas.push(nueva_config_rama),
        }

        config.guardar_config()
    }

    fn verificar_remoto(&self) -> Result<(), String> {
        if let false = Config::leer_config()?.existe_remote(&self.remoto) {
            return Err(format!(
                "Remoto desconocido: {}\n No se puede usar set-upstream\n",
                self.remoto
            ));
        };

        Ok(())
    }

    fn verificar_rama_local(&self) -> Result<(), String> {
        if !utils::ramas::existe_la_rama(&self.rama_local) {
            return Err(format!(
                "Rama desconocida: {}\n No se puede usar set-upstream\n",
                self.rama_local
            ));
        }

        Ok(())
    }
}

impl Ejecutar for SetUpstream {
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log(&format!(
            "Se ejecuta set-upstream - remoto: {}, rama remota: {},rama local: {}",
            self.remoto, self.rama_remota, self.rama_local
        ));

        self.verificar_remoto()?;
        self.verificar_rama_local()?;

        self.set_upstream()?;

        self.logger.log(&format!(
            "Se ejecuto set-upstream con exito - remoto: {}, rama remota: {},rama local: {}",
            self.remoto, self.rama_remota, self.rama_local
        ));
        Ok("".to_string())
    }
}
#[cfg(test)]

mod tests {
    use serial_test::serial;
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        tipos_de_dato::{comando::Ejecutar, config::Config, logger::Logger},
        utils,
    };

    use super::SetUpstream;

    #[test]
    #[serial]
    fn test_01_se_agrega_correctamente_la_configuracion_a_una_rama() {
        let logger = Arc::new(Logger::new("tmp/set_up_stream_01".into()).unwrap());
        let remoto = "origin".to_string();
        let rama_remota = "trabajo".to_string();
        let rama_local = "trabajando".to_string();
        utils::testing::limpiar_archivo_gir(logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::testing::escribir_rama_local(&rama_local, logger.clone());
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

        let config = Config::leer_config().unwrap();
        assert!(config.existe_rama(&rama_local));

        let (remoto_obtenido, rama_merge_obtendia) = config
            .obtener_remoto_y_rama_merge_rama(&rama_local)
            .unwrap();
        let rama_merge_esperada = PathBuf::from(format!("refs/heads/{}", rama_remota));

        assert_eq!(remoto_obtenido, remoto);
        assert_eq!(rama_merge_esperada, rama_merge_obtendia);
    }

    #[test]
    #[serial]
    fn test_02_se_puede_modificar_el_seteado_de_la_rama() {
        let logger = Arc::new(Logger::new("tmp/set_up_stream_02".into()).unwrap());
        let remoto = "origin".to_string();
        let rama_remota = "trabajo".to_string();
        let rama_local = "trabajando".to_string();
        let rama_remota_2 = "rust".to_string();
        let remoto_2 = "tp".to_string();

        utils::testing::limpiar_archivo_gir(logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto_2, logger.clone());
        utils::testing::escribir_rama_local(&rama_local, logger.clone());
        utils::testing::escribir_rama_remota(&remoto, &rama_remota);
        utils::testing::escribir_rama_remota(&remoto_2, &rama_remota_2);

        SetUpstream::new(
            remoto.clone(),
            rama_remota.clone(),
            rama_local.clone(),
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        SetUpstream::new(
            remoto_2.clone(),
            rama_remota_2.clone(),
            rama_local.clone(),
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();

        let (remoto_obtenido, rama_merge_obtendia) = Config::leer_config()
            .unwrap()
            .obtener_remoto_y_rama_merge_rama(&rama_local)
            .unwrap();
        let rama_merge_esperada = PathBuf::from(format!("refs/heads/{}", rama_remota_2));

        assert_eq!(remoto_obtenido, remoto_2);
        assert_eq!(rama_merge_esperada, rama_merge_obtendia);
    }

    #[test]
    #[serial]
    #[should_panic]
    fn test_03_no_se_puede_setear_un_remoto_que_no_existe() {
        let logger = Arc::new(Logger::new("tmp/set_up_stream_03".into()).unwrap());
        let rama_remota = "trabajo".to_string();
        let rama_local = "trabajando".to_string();
        let remoto = "origin".to_string();

        utils::testing::limpiar_archivo_gir(logger.clone());
        utils::testing::escribir_rama_local(&rama_local, logger.clone());
        utils::testing::escribir_rama_remota(&remoto, &rama_remota);

        SetUpstream::new(
            remoto.clone(),
            rama_remota.clone(),
            rama_local.clone(),
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();
    }

    #[test]
    #[serial]
    #[should_panic]
    fn test_04_no_se_puede_setear_una_rama_local_que_no_exite() {
        let logger = Arc::new(Logger::new("tmp/set_up_stream_04".into()).unwrap());
        let rama_remota = "trabajo".to_string();
        let rama_local = "trabajando".to_string();
        let remoto = "origin".to_string();

        utils::testing::limpiar_archivo_gir(logger.clone());
        utils::testing::anadir_remoto_default_config(&remoto, logger.clone());
        utils::testing::escribir_rama_remota(&remoto, &rama_remota);

        SetUpstream::new(
            remoto.clone(),
            rama_remota.clone(),
            rama_local.clone(),
            logger.clone(),
        )
        .unwrap()
        .ejecutar()
        .unwrap();
    }
}
