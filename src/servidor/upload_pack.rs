use crate::tipos_de_dato::comunicacion::Comunicacion;
use crate::tipos_de_dato::logger::Logger;
use crate::tipos_de_dato::packfile;
use crate::utils::strings::eliminar_prefijos;
use crate::utils::{self, objects};
use std::io::{Read, Write};
use std::sync::Arc;

/// Envia packfile al cliente,
/// # Argumentos
/// * `dir` - Direccion del repositorio
/// * `comunicacion` - Comunicacion con el cliente
/// * `refs_enviadas` - Referencias enviadas al cliente previamente, se utilizan para comparar con los wants posteriormente
pub fn upload_pack<T>(
    dir: String,
    comunicacion: &mut Comunicacion<T>,
    refs_enviadas: &Vec<String>,
    logger: Arc<Logger>,
) -> Result<(), String>
where
    T: Read + Write,
{
    logger.log("Iniciando upload pack");
    let wants = comunicacion.obtener_lineas()?; // obtengo los wants del cliente
    if wants.is_empty() {
        println!("Se termino la conexion");
        return Ok(()); // el cliente esta actualizado
    }
    comprobar_wants(&wants, refs_enviadas, comunicacion)?; // compruebo que los wants existan

    // ------- CLONE --------
    let lineas_siguientes = comunicacion.obtener_lineas()?;
    if lineas_siguientes[0].clone().contains("done") {
        procesar_pedido_clone(&dir, comunicacion)?;
    } else {
        // -------- fetch ----------
        procesar_pedido_fetch(&dir, comunicacion, lineas_siguientes)?;
    }
    logger.log("Upload pack ejecutado con exito");
    Ok(())
}

// Funcion que se encarga de seguir el protocolo en caso de clone
fn procesar_pedido_clone<T: Read + Write>(
    dir: &str,
    comunicacion: &mut Comunicacion<T>,
) -> Result<(), String> {
    comunicacion.responder(&vec![utils::strings::obtener_linea_con_largo_hex("NAK\n")])?; // respondo NAK
    let packfile = packfile::Packfile::obtener_pack_entero(&(dir.to_string() + "objects/"))?; // obtengo el packfile
    comunicacion.enviar_pack_file(packfile)?;
    Ok(())
}

// Funcion que se encarga de seguir el protocolo en caso de fetch
fn procesar_pedido_fetch<T: Read + Write>(
    dir: &str,
    comunicacion: &mut Comunicacion<T>,
    lineas: Vec<String>,
) -> Result<(), String> {
    let have_objs_ids = eliminar_prefijos(&lineas);
    let respuesta_acks_nak = utils::objects::obtener_objetos_en_comun(
        have_objs_ids.clone(),
        &(dir.to_string() + "objects/"),
    );
    comunicacion.responder(&respuesta_acks_nak)?;
    let _ultimo_done = comunicacion.obtener_lineas()?;
    let faltantes = objects::obtener_archivos_faltantes(have_objs_ids, dir);
    let packfile =
        packfile::Packfile::obtener_pack_con_archivos(faltantes, &(dir.to_string() + "objects/"))?;

    comunicacion.enviar_pack_file(packfile)?;
    Ok(())
}

// Funcion para comprobar si los wants enviados por el cliente son o no validos, en caso de que no lo sean se le envia un mensaje de error al cliente
fn comprobar_wants<T: Read + Write>(
    wants: &Vec<String>,
    refs_enviadas: &Vec<String>,
    comunicacion: &mut Comunicacion<T>,
) -> Result<(), String> {
    for want in wants {
        let want_split: Vec<&str> = want.split_whitespace().collect();
        let want_hash = want_split[1].to_string();
        let mut continua = false;
        for ref_enviada in refs_enviadas {
            let ref_enviada_split: Vec<&str> = ref_enviada.split_whitespace().collect();
            let mut ref_enviada_hash = ref_enviada_split[0].to_string();
            ref_enviada_hash.drain(..4);
            if want_hash == ref_enviada_hash {
                continua = true;
            }
        }
        if !continua {
            comunicacion.responder(&vec![utils::strings::obtener_linea_con_largo_hex(
                &format!(
                    "ERR La referencia {} no coincide con ninguna referencia enviada\n",
                    want_hash
                ),
            )])?;
            Err("el want enviado no coincide con las referencias enviadas.".to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tipos_de_dato::{comunicacion::Comunicacion, logger::Logger};
    use crate::utils::{self};
    use serial_test::serial;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::sync::Arc;

    struct MockTcpStream {
        lectura_data: Vec<u8>,
    }

    impl Read for MockTcpStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let bytes_to_read = std::cmp::min(buf.len(), self.lectura_data.len());
            buf[0..bytes_to_read].copy_from_slice(&self.lectura_data[..bytes_to_read]);
            self.lectura_data.drain(..bytes_to_read);
            Ok(bytes_to_read)
        }
    }

    impl Write for MockTcpStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.lectura_data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.lectura_data.flush()
        }
    }
    #[test]
    #[serial]
    fn test01_clone() {
        let wants = "4163eb28ec61fd1d0c17cf9b77f4c17e1e338b0".to_string();
        let test_dir = env!("CARGO_MANIFEST_DIR").to_string() + "/server_test_dir/test03/.gir/";

        let mock: MockTcpStream = MockTcpStream {
            lectura_data: Vec::new(),
        };
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_02.txt")).unwrap());

        let mut comunicacion = Comunicacion::new_para_testing(mock, logger.clone());
        comunicacion
            .enviar_pedidos_al_servidor_pkt(vec![wants], "".to_string())
            .unwrap();

        comunicacion
            .enviar(&utils::strings::obtener_linea_con_largo_hex("done\n"))
            .unwrap();

        upload_pack(
            test_dir,
            &mut comunicacion,
            &vec![utils::strings::obtener_linea_con_largo_hex(
                "4163eb28ec61fd1d0c17cf9b77f4c17e1e338b0 refs/heads/master\n",
            )],
            logger.clone(),
        )
        .unwrap();
        let respuesta = comunicacion.obtener_lineas().unwrap();
        let respuesta_esperada = vec!["NAK\n".to_string()];
        assert_eq!(respuesta, respuesta_esperada);
        let packfile = comunicacion.obtener_packfile().unwrap();
        assert_eq!(&packfile[..4], "PACK".as_bytes());
    }

    #[test]
    #[serial]
    fn test02_fetch() {
        let wants = "4163eb28ec61fd1d0c17cf9b77f4c17e1e338b0".to_string();
        let test_dir = env!("CARGO_MANIFEST_DIR").to_string() + "/server_test_dir/test03/.gir/";

        let mock: MockTcpStream = MockTcpStream {
            lectura_data: Vec::new(),
        };
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_02.txt")).unwrap());
        let mut comunicacion = Comunicacion::new_para_testing(mock, logger.clone());
        comunicacion
            .enviar_pedidos_al_servidor_pkt(vec![wants], "".to_string())
            .unwrap();
        comunicacion
            .enviar_lo_que_tengo_al_servidor_pkt(&vec![
                "8f63722a025d936c53304d40ba3197ffebf194d1\n".to_string(),
            ])
            .unwrap();
        comunicacion
            .responder(&vec![utils::strings::obtener_linea_con_largo_hex("done\n")])
            .unwrap();
        upload_pack(
            test_dir,
            &mut comunicacion,
            &vec![utils::strings::obtener_linea_con_largo_hex(
                "4163eb28ec61fd1d0c17cf9b77f4c17e1e338b0 refs/heads/master\n",
            )],
            logger.clone(),
        )
        .unwrap();
        let respuesta = comunicacion.obtener_lineas().unwrap();
        let respuesta_esperada = vec!["ACK 8f63722a025d936c53304d40ba3197ffebf194d1\n".to_string()];
        assert_eq!(respuesta, respuesta_esperada);
        let packfile = comunicacion.obtener_packfile().unwrap();
        assert_eq!(&packfile[..4], "PACK".as_bytes());
    }

    #[test]
    #[serial]
    fn test03_want_con_referencia_invalida_produce_error() {
        let wants = "1".repeat(40);
        let test_dir = env!("CARGO_MANIFEST_DIR").to_string() + "/server_test_dir/test03/.gir/";

        let mock: MockTcpStream = MockTcpStream {
            lectura_data: Vec::new(),
        };
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_02.txt")).unwrap());
        let mut comunicacion = Comunicacion::new_para_testing(mock, logger.clone());
        comunicacion
            .enviar_pedidos_al_servidor_pkt(vec![wants], "".to_string())
            .unwrap();
        let resultado_upload = upload_pack(
            test_dir,
            &mut comunicacion,
            &vec![utils::strings::obtener_linea_con_largo_hex(
                &("4163eb28ec61fd1d0c17cf9b77f4c17e1e338b0".to_string() + " refs/heads/master\n"),
            )],
            logger.clone(),
        );
        assert!(resultado_upload.is_err());
        let respuesta = comunicacion.obtener_lineas().unwrap();
        let respuesta_esperada = vec![
            "ERR ".to_string()
                + &("La referencia ".to_string()
                    + &"1".repeat(40)
                    + " no coincide con ninguna referencia enviada\n"),
        ];
        assert_eq!(respuesta, respuesta_esperada);
    }
}
