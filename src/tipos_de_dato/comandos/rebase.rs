use std::io::prelude::*;
use std::{fs::OpenOptions, path::PathBuf, sync::Arc};

use crate::tipos_de_dato::comando::Ejecutar;
use crate::utils::index::{self, escribir_index};
use crate::utils::ramas;
use crate::{
    tipos_de_dato::{
        comandos::write_tree::conseguir_arbol_en_directorio,
        logger::Logger,
        objetos::{commit::CommitObj, tree::Tree},
    },
    utils::io,
};

use super::checkout::Checkout;
use super::{commit::Commit, log::Log, merge::Merge};

pub struct Rebase {
    pub rama: Option<String>,
    /// Guarda la rama actual.
    pub rama_actual: String,
    /// Logger para imprimir mensajes en el archivo log.
    pub logger: Arc<Logger>,
    /// Indica si se debe abortar el rebase.
    pub abort: bool,
    /// Indica si se debe continuar el rebase.
    pub continue_: bool,
}

impl Rebase {
    /// Crea un nuevo objeto Rebase a partir de los argumentos pasados por linea de comandos.
    /// En el caso de indicar continue o abort, el objeto no va a tener una rama asociada.
    pub fn from(args: Vec<String>, logger: Arc<Logger>) -> Result<Rebase, String> {
        if args.len() != 1 {
            return Err("Se esperaba un argumento".to_string());
        }
        let rama_actual = ramas::obtener_rama_actual()?;
        let arg = args.get(0).ok_or("Se esperaba un argumento")?;
        match arg.as_str() {
            "--abort" => Ok(Rebase {
                rama: None,
                rama_actual,
                logger,
                abort: true,
                continue_: false,
            }),
            "--continue" => Ok(Rebase {
                rama: None,
                rama_actual,
                logger,
                abort: false,
                continue_: true,
            }),
            _ => Ok(Rebase {
                rama: Some(arg.clone()),
                rama_actual,
                logger,
                abort: false,
                continue_: false,
            }),
        }
    }

    /// Obtiene el hash del commit base entre la rama actual y la rama pasada por parametro.
    /// El commit base es el primer commit que tienen en comun las dos ramas.
    /// En caso de no encontrar un commit base devuelve un error.
    fn obtener_commit_base_entre_dos_branches(&self, rama: &str) -> Result<String, String> {
        let hash_commit_actual = ramas::obtener_hash_commit_asociado_rama_actual()?;
        let hash_commit_a_rebasear = Merge::obtener_commit_de_branch(rama)?;

        let commit_obj_actual = CommitObj::from_hash(hash_commit_actual, self.logger.clone())?;
        let commit_obj_a_rebasear =
            CommitObj::from_hash(hash_commit_a_rebasear, self.logger.clone())?;

        let commits_branch_actual =
            Log::obtener_listas_de_commits(commit_obj_actual, self.logger.clone())?;
        let commits_branch_a_rebasear =
            Log::obtener_listas_de_commits(commit_obj_a_rebasear, self.logger.clone())?;

        for commit_actual in commits_branch_actual {
            for commit_branch_merge in commits_branch_a_rebasear.clone() {
                if commit_actual.hash == commit_branch_merge.hash {
                    return Ok(commit_actual.hash);
                }
            }
        }
        Err("No se encontro un commit base entre las dos ramas".to_string())
    }

    /// Obtiene la lista de commits que se le debe aplicar a la rama actual.
    fn obtener_commits_a_aplicar(&self, rama: &str) -> Result<Vec<CommitObj>, String> {
        let hash_ultimo_commit = ramas::obtener_hash_commit_asociado_rama_actual()?;
        let ultimo_commit = CommitObj::from_hash(hash_ultimo_commit, self.logger.clone())?;
        let commits = Log::obtener_listas_de_commits(ultimo_commit, self.logger.clone())?;
        let hash_commit_base = self.obtener_commit_base_entre_dos_branches(rama)?;
        let commits_spliteados: Vec<&[CommitObj]> = commits
            .split(|commit| commit.hash == hash_commit_base)
            .collect();

        commits_spliteados
            .get(0)
            .ok_or("No se encontro el commit base".to_string())
            .map(|commits| commits.to_vec())
    }

    /// Crea la carpeta .gir/rebase-merge y los archivos necesarios para realizar el rebase.
    /// En caso de no poder crear la carpeta o los archivos devuelve un error.
    fn crear_carpeta_rebase(
        &self,
        commits_a_aplicar: &[CommitObj],
        tip_nuevo: &str,
    ) -> Result<(), String> {
        io::crear_directorio(".gir/rebase-merge")?;
        io::escribir_bytes(".gir/rebase-merge/end", commits_a_aplicar.len().to_string())?;

        let mut archivo_to_do = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(".gir/rebase-merge/git-rebase-todo")
            .map_err(|_| "No se pudo abrir el archivo .gir/rebase-merge/git-rebase-todo")?;

        for commit in commits_a_aplicar.iter() {
            writeln!(archivo_to_do, "pick {} {}", commit.hash, commit.mensaje).map_err(|_| {
                "No se pudo escribir en el archivo .gir/rebase-merge/git-rebase-todo"
            })?;
        }

        let ref_head = io::leer_a_string(".gir/HEAD")?;
        io::escribir_bytes(".gir/rebase-merge/head-name", ref_head)?;

        let head = ramas::obtener_hash_commit_asociado_rama_actual()?;
        io::escribir_bytes(".gir/rebase-merge/orig-head", head)?;
        io::escribir_bytes(".gir/rebase-merge/msgnum", 0.to_string())?;
        io::escribir_bytes(".gir/rebase-merge/onto", tip_nuevo)?;

        Ok(())
    }

    /// Actualiza el archivo .gir/rebase-merge/git-rebase-todo con el commit que se acaba de aplicar.
    /// Este archivo contiene los commits que faltan aplicar.
    /// Actualiza el archivo .gir/rebase-merge/done con el commit que se acaba de aplicar.
    /// Este archivo contiene los commits que ya se aplicaron.
    /// Actualiza el archivo .gir/rebase-merge/message con el mensaje del commit que se acaba de aplicar.
    /// Actualiza el archivo .gir/rebase-merge/msgnum con el numero de commit que se acaba de aplicar.
    fn actualizar_carpeta_rebase(&self, commit: &CommitObj) -> Result<(), String> {
        let to_do = io::leer_a_string(".gir/rebase-merge/git-rebase-todo")?;
        let mut to_do = to_do.lines().collect::<Vec<&str>>();
        to_do.remove(0);
        let to_do = to_do.join("\n");
        io::escribir_bytes(".gir/rebase-merge/git-rebase-todo", to_do)?;

        let mut archivo_done = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(".gir/rebase-merge/done")
            .map_err(|_| "No se pudo abrir el archivo .gir/rebase-merge/done")?;

        writeln!(archivo_done, "pick {} {}", commit.hash, commit.mensaje)
            .map_err(|_| "No se pudo escribir en el archivo .gir/rebase-merge/done")?;

        let msgnum = io::leer_a_string(".gir/rebase-merge/msgnum")?;
        let msgnum = msgnum
            .parse::<usize>()
            .map_err(|_| "No se pudo parsear msgnum")?;
        let msgnum = msgnum + 1;
        io::escribir_bytes(".gir/rebase-merge/msgnum", msgnum.to_string())?;
        io::escribir_bytes(".gir/rebase-merge/message", commit.mensaje.clone())?;

        Ok(())
    }

    /// Actualiza el archivo .gir/rebase-merge/rewritten-list con el commit que se acaba de aplicar.
    /// Este archivo contiene los commits que ya se aplicaron.
    fn actualizar_lista_de_commits_aplicados(&self, commit_sha: &str) -> Result<(), String> {
        let mut archivo = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(".gir/rebase-merge/rewritten-list")
            .map_err(|_| "No se pudo abrir el archivo .gir/rebase-merge/rewritten-list")?;

        let tip = ramas::obtener_hash_commit_asociado_rama_actual()?;

        writeln!(archivo, "{} {}", tip, commit_sha)
            .map_err(|_| "No se pudo escribir en el archivo .gir/rebase-merge/rewritten-list")?;

        Ok(())
    }

    /// Realiza el rebase por primera vez.
    /// Crea la carpeta .gir/rebase-merge y los archivos necesarios para realizar el rebase.
    fn primera_vez(&self) -> Result<String, String> {
        if PathBuf::from(".gir/rebase-merge").exists() {
            return Err("Hay rebase en progreso".to_string());
        }

        let rama = self.rama.as_ref().ok_or("No se especifico una rama")?;

        self.logger.log("Rebaseando...");
        let commits_a_aplicar = self.obtener_commits_a_aplicar(rama)?;

        let tip_nuevo = io::leer_a_string(format!(".gir/refs/heads/{}", rama))?;
        self.crear_carpeta_rebase(&commits_a_aplicar, &tip_nuevo)?;

        let branch_actual = self.rama_actual.clone();
        io::escribir_bytes(format!(".gir/refs/heads/{branch_actual}"), &tip_nuevo)?;

        let hash_arbol_commit = conseguir_arbol_en_directorio(&tip_nuevo, ".gir/objects/")?;
        let arbol = Tree::from_hash(&hash_arbol_commit, PathBuf::from("./"), self.logger.clone())?;

        arbol.escribir_en_directorio()?;

        self.rebasear_commits(commits_a_aplicar)?;
        self.logger.log("Rebase finalizado");

        Ok(format!(
            "Se aplicaron los commits de la rama {} a la rama {}",
            rama, self.rama_actual
        ))
    }

    /// Aplica los commits a la rama actual.
    /// En caso de encontrar un conflicto, se detiene el rebase y se guarda el estado actual.
    /// En caso de no encontrar conflictos, se aplica el commit y se continua con el rebase.
    fn rebasear_commits(&self, commits_a_aplicar: Vec<CommitObj>) -> Result<(), String> {
        for commit in commits_a_aplicar {
            self.actualizar_carpeta_rebase(&commit)?;

            let conflictos = commit.aplicar_a_directorio()?;
            if !conflictos.is_empty() {
                io::escribir_bytes(".gir/rebase-merge/stopped-sha", commit.hash)?;
                let mut index = index::leer_index(self.logger.clone())?;

                let mut index_nuevo: Vec<_> = index
                    .iter_mut()
                    .map(|objeto_index| {
                        if conflictos.contains(&objeto_index.objeto.obtener_path()) {
                            objeto_index.merge = true;
                        }
                        objeto_index.clone()
                    })
                    .collect();

                escribir_index(self.logger.clone(), &mut index_nuevo)?;

                return Err("Se encontro un conflicto".to_string());
            }

            self.actualizar_lista_de_commits_aplicados(&commit.hash)?;

            let mut comando_commit = Commit {
                mensaje: commit.mensaje,
                logger: self.logger.clone(),
                rama_actual: self.rama_actual.clone(),
            };
            comando_commit.ejecutar()?;
        }

        Ok(())
    }

    /// Aborta el rebase.
    /// Vuelve al estado original previo a comenzar con el rebase.
    /// Borra toda la informacion que se creo para el rebase.
    fn abortar(&self) -> Result<String, String> {
        let head_name = io::leer_a_string(".gir/rebase-merge/head-name")?;
        let orig_head = io::leer_a_string(".gir/rebase-merge/orig-head")?;

        let rama = head_name
            .split('/')
            .last()
            .ok_or("No se pudo obtener la rama")?;

        io::escribir_bytes(format!(".gir/refs/heads/{}", rama), orig_head)?;

        let tree = Checkout::obtener_arbol_commit_actual(self.logger.clone())?;

        tree.escribir_en_directorio()?;

        io::rm_directorio(".gir/rebase-merge")?;

        index::limpiar_archivo_index()?;

        self.logger.log("Rebase abortado");
        Ok("Rebase abortado".to_string())
    }

    /// Continua con el rebase.
    /// Aplica el commit que se estaba aplicando antes de detener el rebase.
    /// En caso de no haber mas commits para aplicar, termina el rebase.
    /// En caso de encontrar un conflicto, se detiene el rebase y se guarda el estado actual.
    fn continuar(&self) -> Result<String, String> {
        if !PathBuf::from(".gir/rebase-merge").exists() {
            return Err("No hay rebase en progreso".to_string());
        }
        let mensaje_commit = io::leer_a_string(".gir/rebase-merge/message")?;
        let mut commit = Commit::from(
            &mut vec!["-m".to_string(), mensaje_commit],
            self.logger.clone(),
        )?;
        commit.ejecutar()?;

        let contenido_to_do = io::leer_a_string(".gir/rebase-merge/git-rebase-todo")?;
        let lineas_to_do = contenido_to_do.lines().collect::<Vec<&str>>();

        let mut commits_restantes = Vec::new();

        for linea in lineas_to_do {
            let linea_spliteada = linea.split(' ').collect::<Vec<&str>>();
            if linea_spliteada.len() != 3 {
                return Err("No se pudo parsear la linea del archivo git-rebase-todo".to_string());
            }

            let commit = CommitObj::from_hash(linea_spliteada[1].to_string(), self.logger.clone())?;
            commits_restantes.push(commit);
        }

        self.rebasear_commits(commits_restantes)?;
        let msg_num = io::leer_a_string(".gir/rebase-merge/msgnum")?;
        let end = io::leer_a_string(".gir/rebase-merge/end")?;
        if msg_num == end {
            io::rm_directorio(".gir/rebase-merge")?;
            index::limpiar_archivo_index()?;
        }
        Ok("Rebase terminado con extito".to_string())
    }
}

impl Ejecutar for Rebase {
    /// Ejecuta el comando rebase.
    fn ejecutar(&mut self) -> Result<String, String> {
        if self.abort {
            return self.abortar();
        }

        if self.continue_ {
            return self.continuar();
        }

        if self.rama.is_some() {
            return self.primera_vez();
        }

        Err("No se especifico una rama".to_string())
    }
}
