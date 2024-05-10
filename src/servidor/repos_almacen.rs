use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
///Un almacenador de que guarda cada repo con su respectivo
/// arc mutex asosiado. Util cuando un varios thread quieren
/// acceder al mismo repo, para sincronizarse
pub struct ReposAlmacen {
    ///la llave es el repo y el valor el Mutex
    pub repo_mutexes: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl ReposAlmacen {
    ///Crea un Alcenador Repositorios
    pub fn new() -> Self {
        ReposAlmacen {
            repo_mutexes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    ///Obtien el mutex asosiado a cierto repositorio. En caso de no existir
    /// todavia el repositorio, lo crea en la estructura y lo coloca con su
    /// determinado mutex valor  
    pub fn obtener_mutex_del_repo(&self, repo: &str) -> Result<Arc<Mutex<()>>, String> {
        Ok(self
            .repo_mutexes
            .lock()
            .map_err(|e| e.to_string())?
            .entry(repo.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }
}

impl Default for ReposAlmacen {
    fn default() -> Self {
        Self::new()
    }
}
