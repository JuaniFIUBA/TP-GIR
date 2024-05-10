use crate::tipos_de_dato::comunicacion::Comunicacion;
use crate::tipos_de_dato::logger::Logger;
use crate::tipos_de_dato::packfile::Packfile;
use crate::utils::io;
use std::io::{Read, Write};
use std::sync::Arc;

/// Funcion que se encarga de recibir un packfile y actualizar las referencias siguiendo el git transfer protocol
/// # Argumentos
/// * `dir` - Direccion del repositorio
/// * `comunicacion` - Comunicacion con el cliente
/// # Errores
/// Devuelve un error si no se puede leer el packfile o si no se puede escribir en el repositorio
pub fn receive_pack<T>(
    dir: String,
    comunicacion: &mut Comunicacion<T>,
    logger: Arc<Logger>,
) -> Result<(), String>
where
    T: Read + Write,
{
    logger.log("Iniciando receive pack");
    let actualizaciones = comunicacion.obtener_lineas()?;
    let packfile = comunicacion.obtener_packfile()?;

    Packfile::leer_packfile_y_escribir(&packfile, dir.clone() + "objects/")?;

    for actualizacion in &actualizaciones {
        let mut partes = actualizacion.split(' ');
        let viejo_hash_ref = partes.next().unwrap_or("");
        let nuevo_hash_ref = partes.next().unwrap_or("");
        let referencia = partes.next().unwrap_or("").trim_end_matches('\n');
        if nuevo_hash_ref != viejo_hash_ref {
            io::escribir_bytes(dir.clone() + referencia, nuevo_hash_ref)?;
        }
    }
    logger.log("Receive pack ejecutado con exito");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tipos_de_dato::{comunicacion::Comunicacion, logger::Logger, packfile};
    use crate::utils;
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
    fn test01_refs_se_actualizan_correctamente() {
        let test_dir = env!("CARGO_MANIFEST_DIR").to_string() + "/server_test_dir/test03/.gir/";
        let mock: MockTcpStream = MockTcpStream {
            lectura_data: Vec::new(),
        };
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/fetch_02.txt")).unwrap());
        let mut comunicacion = Comunicacion::new_para_testing(mock, logger.clone());

        let actualizaciones = utils::strings::obtener_linea_con_largo_hex(
            &("0".repeat(40) + " " + &"1".repeat(40) + " refs/heads/master\n"),
        );
        comunicacion.enviar(&actualizaciones).unwrap();
        comunicacion.enviar("0000").unwrap();
        let packfile =
            packfile::Packfile::obtener_pack_con_archivos(vec![], &(test_dir.clone() + "objects/"))
                .unwrap();
        comunicacion.enviar_pack_file(packfile).unwrap();
        let nuevo_repo = env!("CARGO_MANIFEST_DIR").to_string() + "/server_test_dir/test04/";
        receive_pack(nuevo_repo.clone(), &mut comunicacion, logger.clone()).unwrap();
        // let ref_nuevo_repo = io::leer_bytes(nuevo_repo + "refs/heads/master").unwrap();
        let nueva_ref = io::leer_a_string(nuevo_repo + "refs/heads/master").unwrap();
        assert_eq!(nueva_ref, "1".repeat(40));
    }
}
