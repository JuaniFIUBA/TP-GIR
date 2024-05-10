use std::sync::Arc;

use crate::{
    tipos_de_dato::{comando::Ejecutar, logger::Logger},
    utils::{self, io, ramas},
};

pub struct Tag {
    logger: Arc<Logger>,
    tag_to_create: Option<String>,
}

impl Tag {
    /// Devuelve un Tag con los parametros ingresados por el usuario.
    pub fn from(args: Vec<String>, logger: Arc<Logger>) -> Result<Tag, String> {
        if args.is_empty() {
            return Ok(Tag {
                logger,
                tag_to_create: None,
            });
        }

        if args.len() != 1 {
            return Err("Cantidad de argumentos invalida".to_string());
        }

        let tag_to_create = Some(args[0].clone());

        Ok(Tag {
            logger,
            tag_to_create,
        })
    }

    /// Devuelve un vector con los nombres de los tags existentes dentro del repositorio.
    /// Si no hay tags, devuelve un vector vacio.
    fn obtener_tags(&self) -> Result<Vec<String>, String> {
        let mut tags = utils::tags::obtener_tags()?;
        tags.sort();
        println!("Tags: {:?}", tags);
        Ok(tags)
    }

    /// Crea un tag con el nombre ingresado por el usuario.
    /// Si el tag ya existe, devuelve un error.
    fn crear_tag(&self, tag: &str) -> Result<(), String> {
        if utils::tags::existe_tag(tag) {
            return Err(format!("El tag {} ya existe", tag));
        }

        let ubicacion = format!(".gir/refs/tags/{}", tag);
        let commit = ramas::obtener_hash_commit_asociado_rama_actual()?;

        io::escribir_bytes(ubicacion, commit)?;

        self.logger.log(&format!("Tag {} creado con exito", tag));

        Ok(())
    }
}

impl Ejecutar for Tag {
    /// Ejecuta el comando tag.
    fn ejecutar(&mut self) -> Result<String, String> {
        match &self.tag_to_create {
            Some(tag_name) => {
                self.crear_tag(tag_name)?;
                Ok(String::new())
            }
            None => {
                let tags = self.obtener_tags()?;
                Ok(tags.join("\n"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use serial_test::serial;

    use crate::{
        tipos_de_dato::{comando::Ejecutar, comandos::tag::Tag, logger::Logger},
        utils::{tags::existe_tag, testing::limpiar_archivo_gir},
    };

    #[test]
    #[serial]
    fn test01_crear_tag() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tag_test01")).unwrap());
        limpiar_archivo_gir(logger.clone());
        let mut tag = Tag::from(vec!["mi_tag".to_string()], logger.clone()).unwrap();

        let _ = tag.ejecutar();

        assert!(existe_tag("mi_tag"));
    }

    #[test]
    #[serial]
    #[should_panic(expected = "El tag mi_tag ya existe")]
    fn test02_crear_tag_existente() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tag_test02")).unwrap());
        let mut tag = Tag::from(vec!["mi_tag".to_string()], logger.clone()).unwrap();

        let _ = tag.ejecutar().unwrap();
    }

    #[test]
    #[serial]
    fn test03_obtener_tags() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tag_test03")).unwrap());
        let mut crear_tag = Tag::from(vec!["otro_tag".to_string()], logger.clone()).unwrap();
        let _ = crear_tag.ejecutar();

        let mut mostrar_tags = Tag::from(vec![], logger.clone()).unwrap();
        let tags = mostrar_tags.ejecutar().unwrap();

        assert_eq!(tags, "mi_tag\notro_tag");
    }
}
