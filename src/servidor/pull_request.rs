use crate::{
    tipos_de_dato::{
        comandos::{log::Log, merge::Merge},
        http::error::ErrorHttp,
        logger::Logger,
        objetos::commit::CommitObj,
    },
    utils::{self, io},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

const OPEN: &str = "open";
const CLOSED: &str = "closed";

#[derive(Serialize, Deserialize, Debug)]
pub struct PullRequest {
    pub numero: u64,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default = "default_valor_opcional"
    )]
    pub titulo: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default = "default_valor_opcional"
    )]
    pub descripcion: Option<String>,
    ///representa el estado del pr: solo puede ser `open` o `closedd`
    pub estado: String,
    pub autor: String,
    pub rama_head: String,
    pub rama_base: String,
    pub fecha_creacion: String,
    pub fecha_modificacion: String,
    pub repositorio: String,
}

fn default_valor_opcional() -> Option<String> {
    None
}

impl PullRequest {
    ///Crea un pull request apartir del body de la request.
    ///
    /// ## Argumentos
    /// - repositorio: el repositorio para el cual se va a crear el pr.
    ///                 Tiene que existir `./srv/{repositorio}`
    /// -body: el cuerpo de la request recibida. El body tiene que contener
    ///         los campos: `head` y `base`. Puede tener como opcionales:
    ///         `title` y `body`.
    ///
    /// ## Resultado
    /// - El pr creados con los campos obligatorios y opcionales. El numero del
    ///     pr depende de la cantidad de prs que ya tenga el repostiorio. El estado
    ///     inicial siempre es `open`     
    ///
    /// ## Errores
    /// - Si falta algun campo obligatorio
    /// - Si no exite repositorio
    /// - Si no existe `rama_head`, `rama_base` en el repositorio
    pub fn crear_pr(
        repositorio: &str,
        body: HashMap<String, String>,
    ) -> Result<PullRequest, ErrorHttp> {
        Self::verificar_repositorio(repositorio)?;

        let numero = Self::obtener_numero(repositorio)?;
        let titulo = Self::obtener_titulo(&body);
        let descripcion = Self::obtener_descripcion(&body);
        let estado = OPEN.to_string();
        let (autor, rama_head) = Self::obtener_autor_y_rama_head(repositorio, &body)?;
        let rama_base = Self::obtener_rama_base(repositorio, &body)?;
        let fecha_actual = Self::obtener_fecha_actual();

        Ok(PullRequest {
            numero,
            titulo,
            descripcion,
            estado,
            autor,
            rama_head,
            rama_base,
            fecha_creacion: fecha_actual.clone(),
            fecha_modificacion: fecha_actual,
            repositorio: repositorio.to_string(),
        })
    }

    ///Valida que el pr cumple todo los filtros recibidos en el body. Si no pasa alguno de
    /// los filtros se devuelve false. Si pasa todos true   
    ///
    /// ## Argumetos
    /// - body: el body de la request, desde donde se sacan los filtros
    ///         a aplicar al pr. Los filtros que se aceptan son `state`,
    ///         `head` y `base`
    /// ## Resultado
    /// - si pasa o no todo los filtros recibidos en el body
    pub fn filtrar(&self, body: &HashMap<String, String>) -> bool {
        let mut pasa_el_filtro_del_estado = true;
        let mut pasa_el_filtro_de_rama_base = true;
        let mut pasa_el_filtro_de_autor_y_rama_head = true;

        //si esta el parametro `state`, valida que sea el mismo al del pr
        if let Some(estado) = body.get("state") {
            pasa_el_filtro_del_estado = self.estado == *estado;
        }

        //si esta el parametro `base`, valida que sea el mismo al del pr
        if let Some(rama_base) = body.get("base") {
            pasa_el_filtro_de_rama_base = self.rama_base == *rama_base;
        }

        //si esta el parametro `head`, valida que sea el mismo el autor y rama head
        //al del pr
        if let Some(usuario_y_rama_head) = body.get("head") {
            let autor_y_rama_base_actual = format!("{}:{}", self.autor, self.rama_head);
            pasa_el_filtro_de_autor_y_rama_head = autor_y_rama_base_actual == *usuario_y_rama_head;
        }

        pasa_el_filtro_de_rama_base
            && pasa_el_filtro_del_estado
            && pasa_el_filtro_de_autor_y_rama_head
    }

    ///Actualiza los campos de un pull request con los parametros del body recibido
    /// Si el pr ya esta cerrado(`estado = "closed"`) no se puede actualizar. Devuelve
    /// si algun campo fue actualizado
    ///
    /// ## Argumentos
    /// - body: el cuerpo del request recivido. Los campos a actualizar pueden ser:
    ///     `state`, `title`, `body` o `base`. En caso de ser `base`, tiene que existir la
    ///     rama base. En caso de ser `state`, tiene ser `"open"` o `"closed"`.
    /// - repositorio: el repositorio al cual pertenece el pr. Tiene que existir
    ///
    /// ## Resultado
    /// - devuelve si se actualizo algun apartado
    ///
    /// ## Errores
    /// - Si no existe la rama base de `base`
    /// - Si `state` no es `"open"` o `"closed"`    
    pub fn actualizar(&mut self, body: HashMap<String, String>) -> Result<bool, ErrorHttp> {
        if self.estado == *CLOSED {
            return Ok(false);
        }

        let se_actualizo_titulo = self.actualizar_titulo(&body);
        let se_actualizo_descripcion = self.actualizar_descripcion(&body);
        let se_actulizo_estado = self.actualizar_estado(&body)?;
        let se_actualiza_rama_base = self.actualizar_rama_base(&body)?;

        let se_actualizo_el_pull_request = se_actualiza_rama_base
            || se_actualizo_descripcion
            || se_actulizo_estado
            || se_actualizo_titulo;

        if se_actualizo_el_pull_request {
            self.fecha_modificacion = Self::obtener_fecha_actual();
        }

        Ok(se_actualizo_el_pull_request)
    }

    fn actualizar_rama_base(&mut self, body: &HashMap<String, String>) -> Result<bool, ErrorHttp> {
        if let Some(nueva_rama_base) = body.get("base") {
            Self::validar_rama(nueva_rama_base, &self.repositorio)?;
            Self::verificar_rama_base_distinta_de_head(&self.rama_head, nueva_rama_base)?;

            let se_actualizo_rama_base = self.rama_base != *nueva_rama_base;
            self.rama_base = nueva_rama_base.to_owned();
            Ok(se_actualizo_rama_base)
        } else {
            Ok(false)
        }
    }

    fn verificar_rama_base_distinta_de_head(
        rama_head: &str,
        rama_base: &str,
    ) -> Result<(), ErrorHttp> {
        if *rama_base == *rama_head {
            return Err(ErrorHttp::ValidationFailed(format!(
                "Rama base ({rama_base}) y rama head ({rama_head}) no puede ser iguales"
            )));
        }

        Ok(())
    }

    fn actualizar_estado(&mut self, body: &HashMap<String, String>) -> Result<bool, ErrorHttp> {
        if let Some(estado) = body.get("state") {
            //verifico que el estado solo pueda ser de los posibles
            if estado == OPEN || estado == CLOSED {
                let se_cambio_estado = self.estado != *estado;
                self.estado = estado.to_owned();
                return Ok(se_cambio_estado);
            }

            Err(ErrorHttp::ValidationFailed(format!(
                "El status {estado} no coincide con ninguno de los posibles: `open` o `closed`"
            )))
        } else {
            Ok(false)
        }
    }

    fn actualizar_descripcion(&mut self, body: &HashMap<String, String>) -> bool {
        let descripcion_nueva = Self::obtener_descripcion(body);

        let se_actualizo_descripcion = self.descripcion != descripcion_nueva;

        self.descripcion = descripcion_nueva;
        se_actualizo_descripcion
    }

    fn actualizar_titulo(&mut self, body: &HashMap<String, String>) -> bool {
        let titulo_nuevo = Self::obtener_titulo(body);

        let se_actualizo_titulo = self.titulo != titulo_nuevo;

        self.titulo = titulo_nuevo;
        se_actualizo_titulo
    }

    fn obtener_fecha_actual() -> String {
        let ahora: DateTime<Utc> = Utc::now();
        ahora.to_rfc3339()
    }

    fn verificar_repositorio(repositorio: &str) -> Result<(), ErrorHttp> {
        let dir_repositorio = PathBuf::from(format!("./srv/{repositorio}"));

        if dir_repositorio.exists() {
            Ok(())
        } else {
            Err(ErrorHttp::ValidationFailed(format!(
                "No existe en el server el repositorio {repositorio}"
            )))
        }
    }

    pub fn obtener_commits(&self, logger: Arc<Logger>) -> Result<Vec<CommitObj>, ErrorHttp> {
        self._obtener_commits(logger)
            .map_err(ErrorHttp::InternalServerError)
    }

    fn _obtener_commits(&self, logger: Arc<Logger>) -> Result<Vec<CommitObj>, String> {
        self.entrar_a_repositorio()
            .map_err(|e| e.obtener_mensaje())?;
        let hash_ultimo_commit = Merge::obtener_commit_de_branch(&self.rama_head)?;
        let ultimo_commit = CommitObj::from_hash(hash_ultimo_commit, logger.clone())?;
        let commits = Log::obtener_listas_de_commits(ultimo_commit, logger.clone())?;
        let hash_commit_base = Merge::obtener_commit_base_entre_dos_branches(
            &self.rama_base,
            &self.rama_head,
            logger.clone(),
        )?;
        self.salir_del_repositorio()
            .map_err(|e| e.obtener_mensaje())?;

        let commits_spliteados: Vec<&[CommitObj]> = commits
            .split(|commit| commit.hash == hash_commit_base)
            .collect();

        commits_spliteados
            .get(0)
            .ok_or("No se encontro el commit base".to_string())
            .map(|commits| commits.to_vec())
    }

    fn obtener_numero(repositorio: &str) -> Result<u64, ErrorHttp> {
        let direccion = PathBuf::from(format!("./srv/{repositorio}/pulls"));
        if !direccion.exists() {
            return Ok(1);
        }

        let numero = utils::io::cantidad_entradas_dir(&direccion).map_err(|_| {
            ErrorHttp::InternalServerError("Fallo al obtener el numero del pr".to_string())
        })? + 1;

        Ok(numero)
    }

    fn obtener_rama_base(
        repositorio: &str,
        body: &HashMap<String, String>,
    ) -> Result<String, ErrorHttp> {
        if let Some(rama_base) = body.get("base") {
            Self::validar_rama(rama_base, repositorio)?;
            Ok(rama_base.to_string())
        } else {
            Err(ErrorHttp::ValidationFailed(
                "Falta el parametro 'base' en el body de la request".to_string(),
            ))
        }
    }

    fn obtener_autor_y_rama_head(
        repositorio: &str,
        body: &HashMap<String, String>,
    ) -> Result<(String, String), ErrorHttp> {
        if let Some(autor_y_rama_head) = body.get("head") {
            let (autor, rama_head) = Self::separara_autor_y_rama_head(autor_y_rama_head)?;
            Self::validar_rama(&rama_head, repositorio)?;
            Ok((autor, rama_head))
        } else {
            Err(ErrorHttp::ValidationFailed(
                "Falta el parametro 'head' en el body de la request".to_string(),
            ))
        }
    }

    //Comprueba si existe en
    fn validar_rama(rama: &str, repositorio: &str) -> Result<(), ErrorHttp> {
        let direccion = PathBuf::from(format!("./srv/{repositorio}/.gir/refs/heads/{rama}"));
        println!("direccion: {:?}", direccion);
        if !direccion.exists() {
            Err(ErrorHttp::ValidationFailed(format!(
                "No existe la rama {rama} en el repositorio {repositorio}"
            )))
        } else {
            Ok(())
        }
    }

    fn separara_autor_y_rama_head(autor_y_rama_head: &str) -> Result<(String, String), ErrorHttp> {
        if let Some((autor, rama_head)) = autor_y_rama_head.split_once(':') {
            Ok((autor.to_string(), rama_head.to_string()))
        } else {
            Err(ErrorHttp::ValidationFailed(format!(
                "Fallo al separar el autor de rama head: {autor_y_rama_head}"
            )))
        }
    }

    fn obtener_titulo(body: &HashMap<String, String>) -> Option<String> {
        body.get("title").map(|titulo| titulo.to_owned())
    }

    fn obtener_descripcion(body: &HashMap<String, String>) -> Option<String> {
        body.get("body").map(|descripcion| descripcion.to_owned())
    }

    pub fn entrar_a_repositorio(&self) -> Result<(), ErrorHttp> {
        let direccion = format!("srv/{}", self.repositorio);
        utils::io::cambiar_directorio(direccion).map_err(ErrorHttp::InternalServerError)?;
        Ok(())
    }

    pub fn salir_del_repositorio(&self) -> Result<(), ErrorHttp> {
        utils::io::cambiar_directorio("../../").map_err(ErrorHttp::InternalServerError)?;
        Ok(())
    }

    pub fn guardar_pr(&self, direccion: &PathBuf) -> Result<(), ErrorHttp> {
        let pr_serializado = serde_json::to_string(&self).map_err(|e| {
            ErrorHttp::InternalServerError(format!(
                "No se ha podido serializar el pull request: {}",
                e
            ))
        })?;
        io::escribir_bytes(direccion, pr_serializado.as_bytes()).map_err(|e| {
            ErrorHttp::InternalServerError(format!(
                "No se ha podido guardar el pull request: {}",
                e
            ))
        })?;
        Ok(())
    }

    pub fn cargar_pr(direccion: &PathBuf) -> Result<PullRequest, ErrorHttp> {
        let contenido_pull_request = utils::io::leer_a_string(direccion).map_err(|e| {
            ErrorHttp::InternalServerError(format!("Fallo al leer la entrada {:?}: {e}", direccion))
        })?;
        let pull_request =
            serde_json::from_str::<PullRequest>(&contenido_pull_request).map_err(|e| {
                ErrorHttp::InternalServerError(format!(
                    "Fallo al serializar el contenido {contenido_pull_request}: {e}"
                ))
            })?;
        Ok(pull_request)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use serial_test::serial;

    use crate::{
        servidor::{
            gir_server::ServidorGir, repos_almacen::ReposAlmacen, vector_threads::VectorThreads,
        },
        tipos_de_dato::{
            comando::Ejecutar,
            comandos::{add::Add, commit::Commit, push::Push},
        },
        utils::testing::crear_repo_para_pr,
    };
    use std::{fs::remove_file, net::TcpListener, sync::Mutex};

    fn agregar_commit_a_repo(logger: Arc<Logger>) {
        io::escribir_bytes("archivo", "contenido3").unwrap();
        let mut add = Add::from(vec!["archivo".to_string()], logger.clone()).unwrap();
        add.ejecutar().unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));

        let mut commit = Commit::from(
            &mut ["-m".to_string(), "commit".to_string()].to_vec(),
            logger.clone(),
        )
        .unwrap();
        commit.ejecutar().unwrap();

        let mut push = Push::new(
            &mut vec!["-u".to_string(), "origin".to_string(), "master".to_string()],
            logger.clone(),
        )
        .unwrap();
        push.ejecutar().unwrap();
    }

    #[test]
    fn test01_guardar_pr() {
        let pr = {
            let numero = 1;
            let titulo = Option::Some(String::from("Titulazo"));
            let descripcion = Option::Some(String::from("Descripcion"));
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("Rama head");
            let rama_base = String::from("Rama base");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test01_guardar_pr".to_string(),
            }
        };
        let direccion = PathBuf::from("tmp/test01.json");
        pr.guardar_pr(&direccion).unwrap();
        let pr_cargado = PullRequest::cargar_pr(&direccion).unwrap();
        assert_eq!(pr.numero, pr_cargado.numero);
        assert_eq!(pr.titulo, pr_cargado.titulo);
        assert_eq!(pr.descripcion, pr_cargado.descripcion);
        assert_eq!(pr.fecha_creacion, pr_cargado.fecha_creacion);
        assert_eq!(pr.fecha_modificacion, pr_cargado.fecha_modificacion);
        remove_file("tmp/test01.json").unwrap();
    }

    #[test]
    fn test02_se_puede_guardar_y_cargar_un_pr_con_un_campo_que_no_se_seriliza() {
        let pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("Rama head");
            let rama_base = String::from("Rama base");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio:
                    "test02_se_puede_guardar_y_cargar_un_pr_con_un_campo_que_no_se_seriliza"
                        .to_string(),
            }
        };
        let direccion = PathBuf::from("tmp/test02.json");
        pr.guardar_pr(&direccion).unwrap();
        let pr_cargado = PullRequest::cargar_pr(&direccion).unwrap();
        assert_eq!(pr.numero, pr_cargado.numero);
        assert_eq!(pr.titulo, pr_cargado.titulo);
        assert_eq!(pr.descripcion, pr_cargado.descripcion);
        assert_eq!(pr.fecha_creacion, pr_cargado.fecha_creacion);
        assert_eq!(pr.fecha_modificacion, pr_cargado.fecha_modificacion);
        remove_file("tmp/test02.json").unwrap();
    }

    #[test]
    fn test03_se_puede_actualizar_el_titulo() {
        let mut pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("Rama head");
            let rama_base = String::from("Rama base");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test03_se_puede_actualizar_el_titulo".to_string(),
            }
        };

        let titulo_a_cambiar = "Si ves esto funciona".to_string();
        let mut body = HashMap::new();
        body.insert("title".to_string(), titulo_a_cambiar.clone());
        pr.actualizar_titulo(&body);

        assert_eq!(pr.titulo, Some(titulo_a_cambiar));
    }

    #[test]
    fn test04_se_puede_actualizar_la_descripcion() {
        let mut pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = Some(String::from("Muahahah"));
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("Rama head");
            let rama_base = String::from("Rama base");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test04_se_puede_actualizar_la_descripcion".to_string(),
            }
        };

        let descripcion_a_actualizar = "Si ves esto funciona".to_string();
        let mut body = HashMap::new();
        body.insert("body".to_string(), descripcion_a_actualizar.clone());
        pr.actualizar_descripcion(&body);

        assert_eq!(pr.descripcion, Some(descripcion_a_actualizar));
    }

    #[test]
    fn test05_se_puede_actualizar_el_estado() {
        let mut pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("Rama head");
            let rama_base = String::from("Rama base");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test05_se_puede_actualizar_el_estado".to_string(),
            }
        };

        let estado_actulizar = "closed".to_string();
        let mut body = HashMap::new();
        body.insert("state".to_string(), estado_actulizar.clone());
        pr.actualizar_estado(&body).unwrap();

        assert_eq!(pr.estado, estado_actulizar);
    }

    #[test]
    #[should_panic]
    fn test06_se_el_estado_no_puede_cambiar_a_algo_que_no_se_open_o_closed() {
        let mut pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("Rama head");
            let rama_base = String::from("Rama base");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test06_se_el_estado_no_puede_cambiar_a_algo_que_no_se_open_o_closed"
                    .to_string(),
            }
        };

        let estado_actulizar = "modo diablo".to_string();
        let mut body = HashMap::new();
        body.insert("state".to_string(), estado_actulizar.clone());
        pr.actualizar_estado(&body).unwrap();
    }

    #[test]
    fn test07_se_puede_actualizar_la_rama_base() {
        let mut pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "repo_test_07_pull_request".to_string(),
            }
        };

        let rama_base_actualizar = "bombastic_fantastic".to_string();
        let test_repo = format!(
            "./srv/test07_se_puede_actualizar_la_rama_base/.gir/refs/heads/{rama_base_actualizar}"
        );

        utils::io::crear_archivo(&test_repo).unwrap();
        let mut body = HashMap::new();
        body.insert("base".to_string(), rama_base_actualizar.clone());
        pr.actualizar_rama_base(&body).unwrap();

        assert_eq!(pr.rama_base, rama_base_actualizar);

        remove_file(test_repo).unwrap();
    }

    #[test]
    #[should_panic]
    fn test08_no_se_puede_actualizar_la_rama_base_con_una_rama_inexistente() {
        let mut pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test08_no_se_puede_actualizar_la_rama_base_con_una_rama_inexistente"
                    .to_string(),
            }
        };

        let rama_base_actualizar = "bombastic_fantastic".to_string();
        //creo el repositorio pero no la rama
        let test_repo = "./srv/test08_no_se_puede_actualizar_la_rama_base_con_una_rama_inexistente/.gir/refs/heads".to_string();
        utils::io::crear_directorio(&test_repo).unwrap();

        let mut body = HashMap::new();
        body.insert("base".to_string(), rama_base_actualizar.clone());

        pr.actualizar_rama_base(&body).unwrap();
        remove_file(test_repo).unwrap();
    }

    #[test]
    #[should_panic]
    fn test09_no_se_puede_actualizar_la_rama_base_a_la_rama_head() {
        let mut pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test09_no_se_puede_actualizar_la_rama_base_a_la_rama_head"
                    .to_string(),
            }
        };

        let rama_base_actualizar = "master".to_string();

        let test_repo = format!("./srv/test09_no_se_puede_actualizar_la_rama_base_a_la_rama_head/.gir/refs/heads/{rama_base_actualizar}");
        utils::io::crear_directorio(&test_repo).unwrap();

        let mut body = HashMap::new();
        body.insert("base".to_string(), rama_base_actualizar.clone());

        pr.actualizar_rama_base(&body).unwrap();
        remove_file(test_repo).unwrap();
    }

    #[test]
    fn test_10_se_puede_filtrar_el_pr_acorde_a_su_estado() {
        let pr_1 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_10_se_puede_filtrar_el_pr_acorde_a_su_estado".to_string(),
            }
        };

        let pr_2 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("Autor");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_10_se_puede_filtrar_el_pr_acorde_a_su_estado".to_string(),
            }
        };

        let mut body = HashMap::new();
        body.insert("state".to_string(), "closed".to_string());

        assert!(!pr_1.filtrar(&body));
        assert!(pr_2.filtrar(&body));

        body.insert("state".to_string(), "open".to_string());

        assert!(pr_1.filtrar(&body));
        assert!(!pr_2.filtrar(&body));
    }

    #[test]
    fn test_11_se_puede_filtrar_el_pr_acorde_a_su_rama_base() {
        let pr_1 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("Autor");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_11_se_puede_filtrar_el_pr_acorde_a_su_rama_base".to_string(),
            }
        };

        let pr_2 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("Autor");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("Motomami");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_11_se_puede_filtrar_el_pr_acorde_a_su_rama_base".to_string(),
            }
        };

        let mut body = HashMap::new();
        body.insert("base".to_string(), "master".to_string());

        assert!(pr_1.filtrar(&body));
        assert!(!pr_2.filtrar(&body));
    }

    #[test]
    fn test_12_se_puede_filtrar_el_pr_acorde_a_su_autor_y_rama_head() {
        let pr_1 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("siro");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_12_se_puede_filtrar_el_pr_acorde_a_su_autor_y_rama_head"
                    .to_string(),
            }
        };

        let pr_2 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("siro");
            let rama_head = String::from("server");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_12_se_puede_filtrar_el_pr_acorde_a_su_autor_y_rama_head"
                    .to_string(),
            }
        };

        let pr_4 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("Juapi");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "repo".to_string(),
            }
        };

        let pr_3 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("Mateo");
            let rama_head = String::from("GUI");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_12_se_puede_filtrar_el_pr_acorde_a_su_autor_y_rama_head"
                    .to_string(),
            }
        };

        let mut body = HashMap::new();
        body.insert("head".to_string(), "siro:trabajo".to_string());

        assert!(pr_1.filtrar(&body));
        assert!(!pr_2.filtrar(&body));
        assert!(!pr_3.filtrar(&body));
        assert!(!pr_4.filtrar(&body));
    }

    #[test]
    fn test_13_se_puede_filtrar_el_pr_con_varios_filtros() {
        let pr_1 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("open");
            let autor = String::from("siro");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_13_se_puede_filtrar_el_pr_con_varios_filtros".to_string(),
            }
        };

        let pr_2 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("siro");
            let rama_head = String::from("server");
            let rama_base = String::from("master");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_13_se_puede_filtrar_el_pr_con_varios_filtros".to_string(),
            }
        };

        let pr_3 = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("Juapi");
            let rama_head = String::from("trabajo");
            let rama_base = String::from("GUI");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "test_13_se_puede_filtrar_el_pr_con_varios_filtros".to_string(),
            }
        };

        let mut body = HashMap::new();
        body.insert("state".to_string(), "open".to_string());
        body.insert("base".to_string(), "master".to_string());

        assert!(pr_1.filtrar(&body));
        assert!(!pr_2.filtrar(&body));
        assert!(!pr_3.filtrar(&body));
    }

    #[test]
    #[serial]
    fn test_14_crear_un_pull_request_y_pushear_commits_obteiene_commits_correctos() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/pr_test_14")).unwrap());

        let logger_clone = logger.clone();
        let (tx, _) = std::sync::mpsc::channel();
        let repos_almacen = ReposAlmacen::new();

        let handle = std::thread::spawn(move || {
            let threads: VectorThreads = Arc::new(Mutex::new(Vec::new()));
            let listener = TcpListener::bind("127.0.0.1:9933").unwrap();

            let mut servidor_gir = ServidorGir {
                listener,
                threads,
                logger: logger_clone,
                main: None,
                tx,
                repos_almacen,
            };
            servidor_gir.iniciar_servidor().unwrap();
        });

        if handle.is_finished() {
            panic!("No se pudo iniciar el servidor");
        }

        std::thread::sleep(std::time::Duration::from_secs(1));

        let _ = io::rm_directorio("tmp/pr_test_14_dir");
        let _ = io::rm_directorio("srv/repo/");
        io::crear_directorio("tmp/pr_test_14_dir").unwrap();
        io::cambiar_directorio("tmp/pr_test_14_dir").unwrap();

        crear_repo_para_pr(logger.clone());
        std::thread::sleep(std::time::Duration::from_secs(1));

        let pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("Juapi");
            let rama_head = String::from("master");
            let rama_base = String::from("rama");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "repo".to_string(),
            }
        };

        io::cambiar_directorio("../../").unwrap();

        let commits = pr.obtener_commits(logger.clone()).unwrap();
        assert!(commits.len() == 1);
        io::rm_directorio("tmp/pr_test_14_dir").unwrap();
    }

    #[test]
    #[serial]
    fn test_15_crear_un_pull_request_y_pushear_commits_cambia_los_commits() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/pr_test_15")).unwrap());

        let logger_clone = logger.clone();
        let (tx, _) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let threads: VectorThreads = Arc::new(Mutex::new(Vec::new()));
            let listener = TcpListener::bind("127.0.0.1:9933").unwrap();

            let mut servidor_gir = ServidorGir {
                listener,
                threads,
                logger: logger_clone,
                main: None,
                tx,
                repos_almacen: ReposAlmacen::new(),
            };
            servidor_gir.iniciar_servidor().unwrap();
        });

        if handle.is_finished() {
            panic!("No se pudo iniciar el servidor");
        }

        std::thread::sleep(std::time::Duration::from_secs(1));

        let _ = io::rm_directorio("tmp/pr_test_15_dir");
        let _ = io::rm_directorio("srv/repo/");
        io::crear_directorio("tmp/pr_test_15_dir").unwrap();
        io::cambiar_directorio("tmp/pr_test_15_dir").unwrap();

        crear_repo_para_pr(logger.clone());
        std::thread::sleep(std::time::Duration::from_secs(1));

        let pr = {
            let numero = 1;
            let titulo = None;
            let descripcion = None;
            let estado = String::from("closed");
            let autor = String::from("Juapi");
            let rama_head = String::from("master");
            let rama_base = String::from("rama");
            let fecha_creacion = String::from("Fecha creacion");
            let fecha_modificacion = String::from("Fecha modificacion");
            PullRequest {
                numero,
                titulo,
                descripcion,
                estado,
                rama_head,
                rama_base,
                fecha_creacion,
                fecha_modificacion,
                autor,
                repositorio: "repo".to_string(),
            }
        };

        io::cambiar_directorio("../../").unwrap();

        let commits = pr.obtener_commits(logger.clone()).unwrap();
        assert!(commits.len() == 1);

        io::cambiar_directorio("tmp/pr_test_15_dir").unwrap();
        agregar_commit_a_repo(logger.clone());
        std::thread::sleep(std::time::Duration::from_secs(1));

        io::cambiar_directorio("../../").unwrap();
        let commits = pr.obtener_commits(logger.clone()).unwrap();
        assert!(commits.len() == 2);

        io::rm_directorio("tmp/pr_test_15_dir").unwrap();
    }
}
