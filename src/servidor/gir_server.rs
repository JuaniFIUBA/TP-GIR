use crate::err_comunicacion::ErrorDeComunicacion;
use crate::servidor::{receive_pack::receive_pack, upload_pack::upload_pack};
use crate::tipos_de_dato::respuesta_pedido::RespuestaDePedido;
use crate::tipos_de_dato::{comunicacion::Comunicacion, logger::Logger};
use crate::utils::{self, io as gir_io};
use std::env::args;
use std::sync::mpsc::Sender;

use std::{
    env,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    str,
    sync::Arc,
    thread,
};

use super::repos_almacen::ReposAlmacen;
use super::rutas::mensaje_servidor::MensajeServidor;
use super::vector_threads::VectorThreads;

const VERSION: &str = "version 1\n";
const CAPABILITIES: &str = "ofs-delta symref=HEAD:refs/heads/master agent=git/2.17.1";
const DIR: &str = "/srv"; // direccion relativa
static SERVER_ARGS: usize = 2;

///
pub struct ServidorGir {
    /// Canal para escuchar las conexiones de clientes
    pub listener: TcpListener,

    /// Threads que se spawnean para atender a los clientes
    pub threads: VectorThreads,

    /// Logger para registrar los eventos del servidor
    pub logger: Arc<Logger>,

    pub main: Option<thread::JoinHandle<()>>,

    pub tx: Sender<MensajeServidor>,

    pub repos_almacen: ReposAlmacen,
}

impl ServidorGir {
    /// # Argumentos:
    /// * `address` - Direccion en la que se va a escuchar las conexiones de los clientes
    /// * `logger` - Logger para registrar los eventos del servidor
    pub fn new(
        logger: Arc<Logger>,
        threads: VectorThreads,
        tx: Sender<MensajeServidor>,
        repos_almacen: ReposAlmacen,
    ) -> Result<ServidorGir, String> {
        let argv = args().collect::<Vec<String>>();
        if argv.len() != SERVER_ARGS {
            println!("Cantidad de argumentos inválido");
            let app_name = &argv[0];
            println!("Usage:\n{:?} <puerto>", app_name);
            return Err("Cantidad de argumentos inválido".to_string());
        }

        let address = "127.0.0.1:".to_owned() + &argv[1];

        let listener = TcpListener::bind(&address).map_err(|e| e.to_string())?;
        println!("Escuchando servidor gir en {}", address);
        logger.log("Servidor iniciado");

        Ok(ServidorGir {
            listener,
            threads,
            logger,
            main: None,
            tx,
            repos_almacen,
        })
    }

    pub fn reiniciar_servidor(&mut self) -> Result<(), String> {
        self.logger.log("Reiniciando servidor gir");
        self.main.take();
        self.iniciar_servidor()
    }

    fn aceptar_conexiones(
        listener: Arc<TcpListener>,
        threads: VectorThreads,
        logger: Arc<Logger>,
        tx: Sender<MensajeServidor>,
        repos_almacen: ReposAlmacen,
    ) {
        while let Ok((stream, socket)) = listener.accept() {
            logger.log(&format!("Se conecto un cliente a gir desde {}", socket));
            let logger_clone = logger.clone();
            let tx = tx.clone();
            let repos_almacen = repos_almacen.clone();
            let handle = thread::spawn(move || -> Result<(), String> {
                let stream_clonado = match stream.try_clone() {
                    Ok(stream) => stream,
                    Err(e) => {
                        tx.send(MensajeServidor::HttpErrorFatal)
                            .expect("Error al enviar mensaje de error fatal al servidor");
                        logger_clone.log(&format!("Error al clonar el stream en http server: {e}"));
                        return Err(format!("Error al clonar el stream en http server: {e}"));
                    }
                };
                let mut comunicacion = Comunicacion::<TcpStream>::new_para_server(
                    stream_clonado,
                    logger_clone.clone(),
                );
                Self::manejar_cliente(
                    &mut comunicacion,
                    &(env!("CARGO_MANIFEST_DIR").to_string() + DIR),
                    logger_clone.clone(),
                    repos_almacen,
                )?;
                Ok(())
            });

            if let Ok(mut threads) = threads.lock() {
                threads.push(handle);
            } else {
                logger.log("Error al obtener el lock de threads");
            }
        }

        tx.send(MensajeServidor::GirErrorFatal)
            .expect("Error al enviar mensaje de error fatal al servidor");
        logger.log("Se cerro el servidor");
    }

    /// Pone en funcionamiento el servidor, spawneando un thread por cada cliente que se conecte al mismo.
    /// Procesa el pedido del cliente y responde en consecuencia.
    pub fn iniciar_servidor(&mut self) -> Result<(), String> {
        let listener = Arc::new(self.listener.try_clone().map_err(|e| e.to_string())?);
        let threads = self.threads.clone();
        let logger = self.logger.clone();
        let tx = self.tx.clone();
        let repos_almacen = self.repos_almacen.clone();
        let handle = thread::spawn(move || {
            Self::aceptar_conexiones(listener, threads, logger, tx, repos_almacen);
        });
        self.main = Some(handle);
        Ok(())
    }

    // Funcion para parsear el pedido del cliente y actuar segun corresponda, retorna en caso de que se haya enviado un
    fn manejar_cliente(
        comunicacion: &mut Comunicacion<TcpStream>,
        dir: &str,
        logger: Arc<Logger>,
        repos_almacen: ReposAlmacen,
    ) -> Result<(), String> {
        let pedido = match comunicacion.aceptar_pedido()? {
            RespuestaDePedido::Mensaje(mensaje) => mensaje,
            RespuestaDePedido::Terminate => return Ok(()),
        }; // acepto la primera linea
        Self::procesar_pedido(&pedido, comunicacion, dir, logger, repos_almacen)?; // parse de la liena para ver que se pide
        Ok(())
    }

    // Facilita la primera parte de la funcion anterior
    fn parsear_linea_pedido_y_responder_con_version(
        linea_pedido: &str,
        dir: &str,
    ) -> Result<(String, String, String), String> {
        let pedido: Vec<String> = linea_pedido
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let args: Vec<String> = pedido[1].split('\0').map(|s| s.to_string()).collect();
        let repositorio = args[0].clone();
        let dir_repositorio = dir.to_string() + &args[0] + "/.gir/";
        let pedido = &pedido[0];
        Ok((pedido.to_owned(), repositorio, dir_repositorio))
    }

    // Funcion para actuar segun si se recibe un upload-pack o un receive-pack, en caso de que sea un receive-pack y el repositorio no exista, se crea el mismo
    fn procesar_pedido(
        linea: &str,
        comunicacion: &mut Comunicacion<TcpStream>,
        dir: &str,
        logger: Arc<Logger>,
        repos_almacen: ReposAlmacen,
    ) -> Result<(), String> {
        let (pedido, repo, dir_repo) =
            Self::parsear_linea_pedido_y_responder_con_version(linea, dir)?;

        let mutex = repos_almacen.obtener_mutex_del_repo(&repo)?;

        let _lock = mutex.lock().map_err(|e| e.to_string())?;

        let refs: Vec<String>;
        let resultado_ejecucion = match pedido.as_str() {
            "git-upload-pack" => {
                if !PathBuf::from(&dir_repo).exists() {
                    let error = ErrorDeComunicacion::ErrorRepositorioNoExiste(repo).to_string();
                    comunicacion.enviar(&utils::strings::obtener_linea_con_largo_hex(&error))?;
                    logger.log(&error);
                    return Err("No existe el repositorio".to_string());
                }
                comunicacion.enviar(&utils::strings::obtener_linea_con_largo_hex(VERSION))?;
                println!("upload-pack recibido, ejecutando");
                refs = server_utils::obtener_refs_de(PathBuf::from(&dir_repo))?;
                comunicacion.responder(&refs)?;
                upload_pack(dir_repo, comunicacion, &refs, logger.clone())
            }
            "git-receive-pack" => {
                println!("receive-pack recibido, ejecutando");

                let path = PathBuf::from(&dir_repo);

                if !path.exists() {
                    gir_io::crear_directorio(path.join("refs/"))?;
                    gir_io::crear_directorio(path.join("refs/heads/"))?;
                    gir_io::crear_directorio(path.join("refs/tags/"))?;
                    gir_io::crear_directorio(path.join("pulls"))?;
                }

                comunicacion.enviar(&utils::strings::obtener_linea_con_largo_hex(VERSION))?;
                refs = server_utils::obtener_refs_de(path)?;
                comunicacion.responder(&refs)?;
                receive_pack(dir_repo.to_string(), comunicacion, logger.clone())
            }
            _ => {
                comunicacion.enviar(&utils::strings::obtener_linea_con_largo_hex(
                    "ERR No existe el comando\n",
                ))?;
                logger.log(&format!("No existe el comando {}", pedido));
                Err("No existe el comando".to_string())
            }
        };

        if let Err(e) = &resultado_ejecucion {
            logger.log(e);
        }

        resultado_ejecucion
    }
}

// -------------- utils del server --------------
mod server_utils {
    use super::*;

    /// Funcion que busca y devuelve las referencias de una direccion dada en formato pkt de un directorio con el formato de git
    pub fn obtener_refs_de(dir: PathBuf) -> Result<Vec<String>, String> {
        let mut refs: Vec<String> = Vec::new();
        let head_ref = utils::referencia::obtener_ref_head(dir.join("HEAD"));
        if let Ok(head) = head_ref {
            refs.push(head)
        }
        let dir_str = match dir.to_str() {
            Some(s) => s,
            None => return Err("No se pudo convertir el path {dir} a str".to_string()),
        };
        utils::referencia::obtener_refs_con_largo_hex(&mut refs, dir.join("refs/heads/"), dir_str)?;
        utils::referencia::obtener_refs_con_largo_hex(&mut refs, dir.join("refs/tags/"), dir_str)?;
        if !refs.is_empty() {
            let ref_con_cap = agregar_capacidades(refs[0].clone());
            refs.remove(0);
            refs.insert(0, ref_con_cap);
        } else {
            refs.push(agregar_capacidades("0".repeat(40)));
        }
        Ok(refs)
    }

    /// Funcion que agrega las capacidades del servidor a una referencia dada en formato pkt
    pub fn agregar_capacidades(referencia: String) -> String {
        let mut referencia_con_capacidades: String;
        if referencia.len() > 40 {
            referencia_con_capacidades = referencia.split_at(4).1.to_string() + "\0";
        } else {
            referencia_con_capacidades = referencia + "\0";
        }
        let capacidades: Vec<&str> = CAPABILITIES.split_whitespace().collect();
        for cap in capacidades.iter() {
            referencia_con_capacidades.push_str(&format!("{} ", cap));
        }
        let mut referencia_con_capacidades = referencia_con_capacidades.trim_end().to_string();
        referencia_con_capacidades.push('\n');
        utils::strings::obtener_linea_con_largo_hex(&referencia_con_capacidades)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test01_agregar_capacidades() {
        let referencia = "0".repeat(40);
        let referencia_con_capacidades = server_utils::agregar_capacidades(referencia);
        println!("{}", referencia_con_capacidades);
        assert_eq!(
            referencia_con_capacidades,
            utils::strings::obtener_linea_con_largo_hex(
                &("0".repeat(40).to_string() + "\0" + CAPABILITIES + "\n")
            )
        );
    }

    #[test]
    #[serial]
    fn test02_obtener_refs_con_ref_vacia_devuelve_ref_nula() {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR").to_string() + "/server_test_dir/test02/.gir/");
        let refs = server_utils::obtener_refs_de(dir).unwrap();
        println!("{:?}", refs);
        assert_eq!(
            refs[0],
            utils::strings::obtener_linea_con_largo_hex(
                &("0".repeat(40).to_string() + "\0" + CAPABILITIES + "\n")
            )
        );
    }

    #[test]
    #[serial]
    fn test03_obtener_refs_con_ref_head_devuelve_ref_head() {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR").to_string() + "/server_test_dir/test03/.gir/");
        let refs = server_utils::obtener_refs_de(dir).unwrap();
        println!("{:?}", refs);
        assert_eq!(
            refs[0],
            utils::strings::obtener_linea_con_largo_hex(
                &("4163eb28ec61fd1d0c17cf9b77f4c17e1e338b0b".to_string()
                    + " HEAD\0"
                    + CAPABILITIES
                    + "\n")
            )
        );
    }
}
