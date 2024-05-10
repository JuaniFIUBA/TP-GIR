use chrono::Local;
use std::default::Default;
use std::io::prelude::*;
use std::path::PathBuf;
use std::{
    env,
    fs::{File, OpenOptions},
    sync::{mpsc, mpsc::Sender},
    thread::{self, JoinHandle},
};

use super::mensajes_log::Log;
use crate::utils::io;

/// Un logger que escribe mensajes en un archivo.
#[derive(Debug)]
pub struct Logger {
    tx: Sender<Log>,
    handle: Option<JoinHandle<()>>,
}

impl Logger {
    /// Crea un nuevo logger que escribe los mensajes en el archivo de log ubicado en la ruta especificada.
    /// Si no se puede obtener el directorio actual o no se puede abrir el archivo de log,
    /// el logger escribira los mensajes en un archivo llamado "log.txt" en el directorio actual.
    pub fn new(ubicacion_archivo: PathBuf) -> Result<Logger, String> {
        let (tx, rx) = mpsc::channel();

        let ubicacion_archivo_completa = Self::obtener_archivo_log(ubicacion_archivo)?;

        let handle = Self::crear_logger_thread(rx, ubicacion_archivo_completa)?;

        Ok(Logger {
            tx,
            handle: Some(handle),
        })
    }

    /// Escribir un mensaje en el archivo de log.
    pub fn log(&self, msg: &str) {
        let log = Log::Message(msg.to_string());
        if self.tx.send(log).is_err() {
            println!("No se pudo escribir {}", msg);
        };
    }

    /// Crea un nuevo logger que escribe los mensajes en un archivo pasado por parametro.
    fn crear_logger_thread(
        rx: mpsc::Receiver<Log>,
        mut archivo_log: File,
    ) -> Result<JoinHandle<()>, String> {
        let logger_thread = thread::Builder::new().name("Logger".to_string());

        logger_thread
            .spawn(move || loop {
                match rx.recv() {
                    Ok(Log::Message(msg)) => {
                        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

                        let _ = escribir_mensaje_en_archivo_log(&mut archivo_log, timestamp, &msg);
                    }
                    Ok(Log::End) => break,
                    Err(_) => break,
                }
            })
            .map_err(|err| format!("ERROR: No se pudo crear el logger.\n{}", err))
    }

    /// Obtiene el archivo de log a partir de la ubicacion pasada por parametro.
    fn obtener_archivo_log(ubicacion_archivo: PathBuf) -> Result<File, String> {
        let dir_archivo_log = Self::obtener_dir_archivo_log(ubicacion_archivo)?;
        OpenOptions::new()
            .append(true)
            .open(dir_archivo_log)
            .map_err(|err| format!("{}", err))
    }

    /// Obtiene el directorio del archivo de log a partir de la ubicacion pasada por parametro.
    fn obtener_dir_archivo_log(ubicacion_archivo: PathBuf) -> Result<PathBuf, String> {
        if ubicacion_archivo.is_absolute() {
            io::crear_archivo(&ubicacion_archivo)?;
            return Ok(ubicacion_archivo);
        }

        let dir_actual = Self::obtener_directorio_actual()?;

        let dir_archivo_log = dir_actual.as_path().join(ubicacion_archivo);

        io::crear_archivo(&dir_archivo_log)?;

        Ok(dir_archivo_log)
    }

    /// Obtiene el directorio actual.
    fn obtener_directorio_actual() -> Result<PathBuf, String> {
        let dir_actual = env::current_dir().map_err(|err| format!("{}", err))?;
        Ok(dir_actual)
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        if self.tx.send(Log::End).is_err() {
            return;
        };

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        let (tx, _) = mpsc::channel();
        Self { tx, handle: None }
    }
}

/// Escribe el mensaje pasado por parametro en el archivo de log junto el timestamp.
fn escribir_mensaje_en_archivo_log(
    data_archivo: &mut File,
    timestamp: chrono::format::DelayedFormat<chrono::format::StrftimeItems<'_>>,
    msg: &str,
) -> Result<(), String> {
    data_archivo
        .write_all(format!("{} | {}\n", timestamp, msg).as_bytes())
        .map_err(|err| format!("{}", err))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Logger;
    use serial_test::serial;
    use std::{env, fs, path::PathBuf, sync::Arc, thread};
    extern crate serial_test;

    #[test]
    #[serial]
    fn test01_al_iniciar_si_archivo_log_no_esta_creado_se_crea() {
        let ubicacion_archivo = PathBuf::from("test_dir/test01.txt");
        Logger::new(ubicacion_archivo.clone()).unwrap();

        assert!(obtener_dir_archivo_log(ubicacion_archivo.clone()).exists());
        eliminar_archivo_log(ubicacion_archivo);
    }

    #[test]
    #[serial]
    fn test02_se_escribe_correctamente_los_mensajes_archivo_log() {
        let ubicacion_archivo = PathBuf::from("test_dir/test02.txt");
        let logger = Logger::new(ubicacion_archivo.clone()).unwrap();

        let msg_test_01 = "sipiropo fapatapalapa";
        let msg_test_02 = "juapuanipi peperezpez";

        logger.log(msg_test_01);
        logger.log(msg_test_02);
        drop(logger);

        assert_el_archivo_log_contiene(ubicacion_archivo.clone(), vec![msg_test_01, msg_test_02]);
        eliminar_archivo_log(ubicacion_archivo);
    }

    #[test]
    #[serial]
    fn test03_si_se_crea_un_logger_no_se_pierden_los_mensajes_anterior() {
        let msg_test_01 = "sipiropo fapatapalapa";
        let ubicacion_archivo = PathBuf::from("test_dir/test03.txt");
        Logger::new(ubicacion_archivo.clone())
            .unwrap()
            .log(msg_test_01);

        let msg_test_02 = "juapuanipi peperezpez";
        Logger::new(ubicacion_archivo.clone())
            .unwrap()
            .log(msg_test_02);

        assert_el_archivo_log_contiene(ubicacion_archivo.clone(), vec![msg_test_01, msg_test_02]);
        eliminar_archivo_log(ubicacion_archivo);
    }

    #[test]
    #[serial]
    fn test04_el_logger_puede_escribir_mensajes_de_varios_threads() {
        let ubicacion_archivo = PathBuf::from("test_dir/test04.txt");
        let logger = Arc::new(Logger::new(ubicacion_archivo.clone()).unwrap());
        let msg_test_01 = "Thread 1 saluda";
        let msg_test_02 = "Thread 2 saluda";
        let msg_test_03 = "Thread 3 saluda";

        let handle_1 = crear_thread_que_mande_mensaje_al_loger(&logger, msg_test_01);
        let handle_2 = crear_thread_que_mande_mensaje_al_loger(&logger, msg_test_02);
        let handle_3 = crear_thread_que_mande_mensaje_al_loger(&logger, msg_test_03);

        handle_1.join().unwrap();
        handle_2.join().unwrap();
        handle_3.join().unwrap();
        drop(logger);

        assert_el_archivo_log_contiene(
            ubicacion_archivo.clone(),
            vec![msg_test_01, msg_test_02, msg_test_03],
        );
        eliminar_archivo_log(ubicacion_archivo);
    }

    fn crear_thread_que_mande_mensaje_al_loger(
        logger: &Arc<Logger>,
        msg: &str,
    ) -> thread::JoinHandle<()> {
        let logger1 = logger.clone();
        let msg_clone = msg.to_string();
        thread::spawn(move || {
            logger1.log(&msg_clone);
        })
    }

    fn assert_el_archivo_log_contiene(ubicacion_archivo: PathBuf, contenidos: Vec<&str>) {
        let contenido_archvo_log =
            fs::read_to_string(obtener_dir_archivo_log(ubicacion_archivo)).unwrap();

        for contenido in contenidos {
            assert!(contenido_archvo_log.contains(contenido));
        }
    }

    fn eliminar_archivo_log(ubicacion_archivo: PathBuf) {
        let dir_archivo_log = obtener_dir_archivo_log(ubicacion_archivo);
        if dir_archivo_log.exists() {
            fs::remove_file(dir_archivo_log.clone()).unwrap();
        }
    }

    fn obtener_dir_archivo_log(ubicacion_archivo: PathBuf) -> std::path::PathBuf {
        env::current_dir()
            .unwrap()
            .as_path()
            .join(ubicacion_archivo)
    }
}
