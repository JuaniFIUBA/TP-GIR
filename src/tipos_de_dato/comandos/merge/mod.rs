mod estrategias_conflictos;

use crate::{
    tipos_de_dato::{
        conflicto::Conflicto,
        diffgrid::DiffGrid,
        lado_conflicto::LadoConflicto,
        objeto::Objeto,
        region::{unificar_regiones, Region},
    },
    utils::{index, ramas},
};
use std::{
    path::{self, Path, PathBuf},
    sync::Arc,
};

use crate::{
    tipos_de_dato::{
        comando::Ejecutar,
        comandos::merge::estrategias_conflictos::resolver_merge_len_2,
        logger::Logger,
        objetos::{commit::CommitObj, tree::Tree},
        tipo_diff::TipoDiff,
    },
    utils::{
        compresion::descomprimir_objeto_gir,
        index::{escribir_index, leer_index, ObjetoIndex},
        io,
    },
};

use self::estrategias_conflictos::{conflicto_len_3, conflicto_len_4, resolver_merge_len_3};

use super::{
    cat_file,
    commit::Commit,
    log::Log,
    write_tree::{self, conseguir_arbol_en_directorio},
};

pub struct Merge {
    pub logger: Arc<Logger>,
    pub branch_actual: String,
    pub branch_a_mergear: String,
    pub abort: bool,
    pub no_fast_forward: bool,
}

impl Merge {
    pub fn from(args: &mut Vec<String>, logger: Arc<Logger>) -> Result<Merge, String> {
        if args.len() != 1 {
            return Err("Cantidad de argumentos invalida".to_string());
        }

        if args[0] == "--abort" {
            return Ok(Merge {
                logger,
                branch_actual: "".to_string(),
                branch_a_mergear: "".to_string(),
                abort: true,
                no_fast_forward: false,
            });
        }

        let branch_a_mergear = args.pop().unwrap();
        if !ramas::existe_la_rama(&branch_a_mergear)
            && !ramas::existe_la_rama_remota(&branch_a_mergear)
        {
            return Err("La rama a mergear no existe".to_string());
        }
        let branch_actual = ramas::obtener_rama_actual()?;
        Ok(Merge {
            logger,
            branch_actual,
            branch_a_mergear,
            abort: false,
            no_fast_forward: false,
        })
    }

    pub fn obtener_arbol_commit_actual(branch: &str, logger: Arc<Logger>) -> Result<Tree, String> {
        let head_commit = Self::obtener_commit_de_branch(branch)?;
        let hash_tree_padre = conseguir_arbol_en_directorio(&head_commit, ".gir/objects/")?;
        Tree::from_hash(&hash_tree_padre, PathBuf::from("."), logger.clone())
    }

    /// Devuelve el commit base mas cercano entre dos ramas
    /// Por ejemplo en el arbol a-b-c vs d-b-e, el commit base es b
    pub fn obtener_commit_base_entre_dos_branches(
        branch_1: &str,
        branch_2: &str,
        logger: Arc<Logger>,
    ) -> Result<String, String> {
        let hash_commit_actual = Self::obtener_commit_de_branch(branch_1)?;
        let hash_commit_a_mergear = Self::obtener_commit_de_branch(branch_2)?;

        let commit_obj_actual = CommitObj::from_hash(hash_commit_actual, logger.clone())?;
        let commit_obj_a_mergear = CommitObj::from_hash(hash_commit_a_mergear, logger.clone())?;

        let commits_branch_actual =
            Log::obtener_listas_de_commits(commit_obj_actual, logger.clone())?;
        let commits_branch_a_mergear =
            Log::obtener_listas_de_commits(commit_obj_a_mergear, logger.clone())?;

        for commit_actual in commits_branch_actual {
            for commit_branch_merge in commits_branch_a_mergear.clone() {
                if commit_actual.hash == commit_branch_merge.hash {
                    return Ok(commit_actual.hash);
                }
            }
        }
        Err("No se encontro un commit base entre las dos ramas".to_string())
    }

    /// Devuelve un vector con las lineas que difieren entre dos archivos
    fn obtener_diffs_entre_dos_archivos(
        archivo_1: &str,
        archivo_2: &str,
    ) -> Result<Vec<(usize, TipoDiff)>, String> {
        let archivo_1_splitteado = archivo_1.split('\n').collect::<Vec<&str>>();
        let archivo_2_splitteado = archivo_2.split('\n').collect::<Vec<&str>>();
        let diff = Self::obtener_diff(archivo_1_splitteado, archivo_2_splitteado);
        Ok(diff)
    }

    /// Devuelve un vector con las lineas que difieren entre dos objetos
    fn obtener_diffs_entre_dos_objetos(
        hash_objeto1: &str,
        hash_objeto2: &str,
    ) -> Result<Vec<(usize, TipoDiff)>, String> {
        let (_, contenido1) = cat_file::obtener_contenido_objeto(hash_objeto1)?;
        let (_, contenido2) = cat_file::obtener_contenido_objeto(hash_objeto2)?;
        Self::obtener_diffs_entre_dos_archivos(&contenido1, &contenido2)
    }

    /// Calcula la matriz de Longet Common Subsequence entre dos textos
    /// donde los textos son separados en lineas, para que cada linea sea la
    /// minima unidad divisible (no se pueden partir lineas en partes mas pequeñas)
    fn computar_lcs_grid(texto1: &Vec<&str>, texto2: &Vec<&str>) -> DiffGrid {
        let longitud1 = texto1.len();
        let longitud2 = texto2.len();

        let mut matriz_lcs = vec![vec![0; longitud2 + 1]; longitud1 + 1];

        for i in 0..=longitud1 {
            for j in 0..=longitud2 {
                if i == 0 || j == 0 {
                    matriz_lcs[i][j] = 0;
                } else if texto1[i - 1] == texto2[j - 1] {
                    matriz_lcs[i][j] = 1 + matriz_lcs[i - 1][j - 1];
                } else {
                    matriz_lcs[i][j] = std::cmp::max(matriz_lcs[i][j - 1], matriz_lcs[i - 1][j]);
                }
            }
        }
        matriz_lcs
    }

    /// Devuelve un vector con las lineas que difieren entre dos textos
    /// donde los textos son separados en lineas, En el vector se encuentra una tupla con
    /// el diff y el indice de la linea en el texto1. El texto1 es el texto base, y el texto2
    /// es el texto que se quiere mergear
    pub fn obtener_diff(texto1: Vec<&str>, texto2: Vec<&str>) -> Vec<(usize, TipoDiff)> {
        let diff_grid = Self::computar_lcs_grid(&texto1, &texto2);
        let mut i = texto1.len();
        let mut j = texto2.len();
        let mut resultado_diff: Vec<(usize, TipoDiff)> = Vec::new();

        while i != 0 || j != 0 {
            if i == 0 {
                resultado_diff.push((i, TipoDiff::Added(texto2[j - 1].trim().to_string())));
                j -= 1;
            } else if j == 0 {
                resultado_diff.push((i, TipoDiff::Removed(texto1[i - 1].trim().to_string())));
                i -= 1;
            } else if texto1[i - 1] == texto2[j - 1] {
                resultado_diff.push((i, TipoDiff::Unchanged(texto1[i - 1].trim().to_string())));
                i -= 1;
                j -= 1;
            } else if diff_grid[i - 1][j] <= diff_grid[i][j - 1] {
                resultado_diff.push((i, TipoDiff::Added(texto2[j - 1].trim().to_string())));
                j -= 1;
            } else {
                resultado_diff.push((i, TipoDiff::Removed(texto1[i - 1].trim().to_string())));
                i -= 1
            }
        }
        resultado_diff.reverse();
        resultado_diff
    }

    /// Devuelve si hay conflicto basandonos en los distintos casos posibles. Si hay
    /// un solo diff y es un add, no hay conflicto. Si hay mas de un diff y hay un
    /// unchange significa que no hay conflicto ya que la contraposicion puede ser
    /// aplicada sin problemas
    fn hay_conflicto(posibles_conflictos: &Vec<(TipoDiff, LadoConflicto)>) -> bool {
        if posibles_conflictos.len() == 1 {
            if let TipoDiff::Added(_) = posibles_conflictos[0].0 {
                return false;
            }
        }

        posibles_conflictos
            .iter()
            .all(|(diff, _)| !matches!(diff, TipoDiff::Unchanged(_)))
    }

    /// Resuelve los conflictos con distintas estrategias basandose en la cantidad de
    /// diffs que hay.
    fn resolver_conflicto(conflicto: &Conflicto, linea_base: &str) -> Region {
        if conflicto.len() == 4 {
            conflicto_len_4(conflicto)
        } else {
            conflicto_len_3(conflicto, linea_base)
        }
    }

    /// Dados dos vectores de diffs, uno de la rama actual y otro de la rama a mergear,
    /// junta los dos diffs en un solo vector de posibles conflictos. Cada posicion del
    /// vector representa una linea del archivo, y en cada posicion hay un vector de
    /// conflictos posibles para esa linea.
    fn obtener_posibles_conflictos(
        diff_actual: Vec<(usize, TipoDiff)>,
        diff_a_mergear: Vec<(usize, TipoDiff)>,
    ) -> Vec<Conflicto> {
        let mut posibles_conflictos: Vec<Conflicto> = Vec::new();

        for diff in diff_actual {
            if diff.0 > posibles_conflictos.len() {
                posibles_conflictos.push(Vec::new());
            }
            posibles_conflictos[diff.0 - 1].push((diff.1, LadoConflicto::Head));
        }

        for diff in diff_a_mergear {
            if diff.0 - 1 > posibles_conflictos.len() {
                posibles_conflictos.push(Vec::new());
            }
            posibles_conflictos[diff.0 - 1].push((diff.1, LadoConflicto::Entrante));
        }

        posibles_conflictos
    }

    /// Dado un vector de posibles conflictos, resuelve todos los conflictos
    /// y devuelve el contenido de cada region y un booleano que indica si hubo
    /// conflictos
    fn resolver_conflictos_totales(
        posibles_conflictos: Vec<Conflicto>,
        lineas_archivo_base: Vec<&str>,
        mut anterior_fue_conflicto: bool,
    ) -> (Vec<Region>, bool) {
        let mut contenido_por_regiones: Vec<Region> = Vec::new();
        let mut hubo_conflictos = false;

        for i in 0..posibles_conflictos.len() {
            let posible_conflicto = &posibles_conflictos[i];

            let region = if Self::hay_conflicto(posible_conflicto) {
                hubo_conflictos = true;
                Self::resolver_conflicto(
                    posible_conflicto,
                    lineas_archivo_base.get(i).unwrap_or(&""),
                )
            } else if posible_conflicto.len() == 2 {
                resolver_merge_len_2(
                    posible_conflicto,
                    lineas_archivo_base[i],
                    anterior_fue_conflicto,
                )
            } else {
                resolver_merge_len_3(
                    posible_conflicto,
                    lineas_archivo_base[i],
                    anterior_fue_conflicto,
                )
            };

            anterior_fue_conflicto = Self::hay_conflicto(posible_conflicto);
            contenido_por_regiones.push(region);
        }

        (contenido_por_regiones, hubo_conflictos)
    }

    /// Teniendo el diff de dos archivos respecto a una base devuelve el contenido
    /// del archivo mergeado y un booleano que indica si hubo conflictos
    fn mergear_diffs(
        diff_actual: Vec<(usize, TipoDiff)>,
        diff_a_mergear: Vec<(usize, TipoDiff)>,
        archivo_base: &str,
    ) -> (String, bool) {
        let lineas_archivo_base = archivo_base.split('\n').collect::<Vec<&str>>();
        let posibles_conflictos = Self::obtener_posibles_conflictos(diff_actual, diff_a_mergear);

        let anterior_fue_conflicto = false;

        let (contenido_por_regiones, hubo_conflictos) = Self::resolver_conflictos_totales(
            posibles_conflictos,
            lineas_archivo_base,
            anterior_fue_conflicto,
        );

        let regiones_unificadas = unificar_regiones(contenido_por_regiones);

        let mut resultado = String::new();

        for region in &regiones_unificadas {
            resultado.push_str(&format!("{}\n", region));
        }

        (resultado, hubo_conflictos)
    }

    /// Obtiene los arboles necesarios para realizar el merge.
    /// El arbol base es el arbol del commit base, el arbol branch_actual es el arbol del commit
    /// de la rama actual, y el arbol branch_a_mergear es el arbol del commit de la rama a mergear.
    /// La base es el primer commit en comun entre ambas ramas.
    /// Devuelve un error si no se puede conseguir alguno de los arboles.
    fn obtener_arboles_para_ejecutar_merge(
        &self,
        commit_base: &str,
    ) -> Result<(Tree, Tree, Tree), String> {
        let hash_tree_base =
            write_tree::conseguir_arbol_en_directorio(commit_base, ".gir/objects/")?;
        let tree_base = Tree::from_hash(&hash_tree_base, PathBuf::from("."), self.logger.clone())?;

        let tree_branch_actual =
            Self::obtener_arbol_commit_actual(&self.branch_actual, self.logger.clone())?;
        let tree_branch_a_mergear =
            Self::obtener_arbol_commit_actual(&self.branch_a_mergear, self.logger.clone())?;
        Ok((tree_base, tree_branch_actual, tree_branch_a_mergear))
    }

    /// Recorrer los tres arboles indicados y obtiene todos los objetos asociados a los archivos hoja
    /// de cada uno.
    fn obtener_nodos_hoja_de_arboles_a_mergear(
        tree_base: Tree,
        tree_branch_actual: Tree,
        tree_branch_a_mergear: Tree,
    ) -> (Vec<Objeto>, Vec<Objeto>, Vec<Objeto>) {
        let nodos_hoja_base = tree_base.obtener_objetos_hoja();
        let nodos_hoja_branch_actual = tree_branch_actual.obtener_objetos_hoja();
        let nodos_hoja_branch_a_mergear = tree_branch_a_mergear.obtener_objetos_hoja();
        (
            nodos_hoja_base,
            nodos_hoja_branch_actual,
            nodos_hoja_branch_a_mergear,
        )
    }

    /// Dados dos objetos hoja, o sea, archivos, calcula el diff de cada uno respecto a la base, mergea esos diffs
    /// y escribe el resultado en el archivo base
    /// Devuelve un booleano que indica si hubo conflictos.
    /// En caso de haber conflicto agrega el path del archivo con conflicto al listado de archivos con conflictos
    fn obtener_conflictos_entre_archivos_a_mergear(
        &self,
        objeto_base: &Objeto,
        objeto_a_mergear: &Objeto,
        objeto_actual: &Objeto,
        paths_con_conflictos: &mut Vec<String>,
    ) -> Result<bool, String> {
        let diff_a_mergear = Self::obtener_diffs_entre_dos_objetos(
            &objeto_base.obtener_hash(),
            &objeto_a_mergear.obtener_hash(),
        )?;

        let diff_actual = Self::obtener_diffs_entre_dos_objetos(
            &objeto_base.obtener_hash(),
            &objeto_actual.obtener_hash(),
        )?;

        let contenido_base = descomprimir_objeto_gir(&objeto_base.obtener_hash())?;

        let (resultado, hubo_conflictos) =
            Self::mergear_diffs(diff_actual, diff_a_mergear, &contenido_base);

        io::escribir_bytes(objeto_base.obtener_path(), resultado)?;
        if hubo_conflictos {
            paths_con_conflictos.push(format!("{}\n", objeto_base.obtener_path().display()));
        }
        Ok(hubo_conflictos)
    }

    /// Crea un objeto index dependiendo asiganndo si hubo conflictos o no
    /// entre los archivos a mergear y lo agrega al vector de objetos index
    fn agregar_objeto_index_merge_con_conflicto(
        objeto_base: Objeto,
        objetos_index: &mut Vec<ObjetoIndex>,
        hubo_conflictos: bool,
    ) {
        let objeto = ObjetoIndex {
            objeto: objeto_base,
            es_eliminado: false,
            merge: hubo_conflictos,
        };

        objetos_index.push(objeto);
    }

    /// Busca en los dos arboles a mergear el objeto que se esta mergeando actualmente
    /// Si no encuentra el objeto a mergear en el arbol a mergear, agrega el objeto del arbo,actual al index
    fn buscar_objeto_a_mergear_en_ambos_arboles<'a>(
        objeto_base: &'a Objeto,
        nodos_hoja_branch_a_mergear: &'a [Objeto],
        nodos_hoja_branch_actual: &'a [Objeto],
        objetos_index: &mut Vec<ObjetoIndex>,
    ) -> Option<(&'a Objeto, &'a Objeto)> {
        let nombre_objeto = objeto_base.obtener_path();

        let objeto_a_mergear_encontrado = nodos_hoja_branch_a_mergear
            .iter()
            .find(|&nodo| nodo.obtener_path() == nombre_objeto);

        let objeto_actual_encontrado = nodos_hoja_branch_actual
            .iter()
            .find(|&nodo| nodo.obtener_path() == nombre_objeto);

        match (objeto_a_mergear_encontrado, objeto_actual_encontrado) {
            (Some(objeto_a_mergear), Some(objeto_actual)) => {
                Some((objeto_a_mergear, objeto_actual))
            }
            (None, Some(objeto_actual)) => {
                let objeto = ObjetoIndex {
                    objeto: objeto_actual.clone(),
                    es_eliminado: false,
                    merge: false,
                };

                objetos_index.push(objeto);
                None
            }
            _ => None,
        }
    }

    /// Recorre todos los objetos hoja del arbol base y por cada uno mergea la version de cada rama en un archivo
    fn mergear_todos_los_objetos_del_tree_actual(
        &self,
        nodos_hoja_base: Vec<Objeto>,
        nodos_hoja_branch_actual: Vec<Objeto>,
        nodos_hoja_branch_a_mergear: Vec<Objeto>,
        objetos_index: &mut Vec<ObjetoIndex>,
        paths_con_conflictos: &mut Vec<String>,
    ) -> Result<(), String> {
        for objeto_base in nodos_hoja_base {
            if let Some((objeto_a_mergear, objeto_actual)) =
                Self::buscar_objeto_a_mergear_en_ambos_arboles(
                    &objeto_base,
                    &nodos_hoja_branch_a_mergear,
                    &nodos_hoja_branch_actual,
                    objetos_index,
                )
            {
                let hubo_conflictos = self.obtener_conflictos_entre_archivos_a_mergear(
                    &objeto_base,
                    objeto_a_mergear,
                    objeto_actual,
                    paths_con_conflictos,
                )?;

                Self::agregar_objeto_index_merge_con_conflicto(
                    objeto_base,
                    objetos_index,
                    hubo_conflictos,
                );
            }
        }
        Ok(())
    }

    /// Finaliza el auto-merge, si hay conflictos devuelve un mensaje indicando
    /// los archivos con conflictos, si no hay conflictos realiza el commit
    /// del merge.
    fn finalizar_automerge(
        &self,
        paths_con_conflictos: &mut Vec<String>,
    ) -> Result<String, String> {
        if !paths_con_conflictos.is_empty() {
            Ok(format!(
                "Se encontraron conflictos en los siguientes archivos: \n{:#?}",
                paths_con_conflictos
            ))
        } else {
            let mut commit = Commit::from_merge(self.logger.clone(), &self.branch_actual)?;
            commit.ejecutar()?;
            Ok("Merge completado".to_string())
        }
    }

    /// Realiza un auto-merge, realizando un merge de cada file que difiera entre los dos commits
    fn automerge(&self, commit_base: &str) -> Result<String, String> {
        let (tree_base, tree_branch_actual, tree_branch_a_mergear) =
            self.obtener_arboles_para_ejecutar_merge(commit_base)?;

        let (nodos_hoja_base, nodos_hoja_branch_actual, nodos_hoja_branch_a_mergear) =
            Self::obtener_nodos_hoja_de_arboles_a_mergear(
                tree_base,
                tree_branch_actual,
                tree_branch_a_mergear,
            );

        let mut objetos_index: Vec<ObjetoIndex> = Vec::new();
        let mut paths_con_conflictos: Vec<String> = Vec::new();

        self.mergear_todos_los_objetos_del_tree_actual(
            nodos_hoja_base,
            nodos_hoja_branch_actual,
            nodos_hoja_branch_a_mergear,
            &mut objetos_index,
            &mut paths_con_conflictos,
        )?;

        escribir_index(self.logger.clone(), &mut objetos_index)?;
        self.escribir_merge_head()?;
        self.escribir_mensaje_merge()?;

        self.finalizar_automerge(&mut paths_con_conflictos)
    }

    /// Realiza un fast-forward, moviendo el puntero de la rama actual al commit de la rama a mergear
    pub fn fast_forward(&self) -> Result<String, String> {
        let commit_banch_a_mergear = Self::obtener_commit_de_branch(&self.branch_a_mergear)?;

        io::escribir_bytes(
            Path::new(&format!(".gir/refs/heads/{}", self.branch_actual)),
            commit_banch_a_mergear,
        )?;

        let tree_branch_a_mergear =
            Self::obtener_arbol_commit_actual(&self.branch_a_mergear, self.logger.clone())?;

        tree_branch_a_mergear.escribir_en_directorio()?;
        Ok("Merge con fast-forward completado".to_string())
    }

    /// Busca en el index si hay archivos con el flag merge en true
    /// indicando que hubieron conflictos y no se resolvieron
    pub fn hay_archivos_sin_mergear(logger: Arc<Logger>) -> Result<bool, String> {
        let ruta_index = Path::new(".gir/index");
        if !ruta_index.exists() {
            return Ok(false);
        }
        let contenido_index = leer_index(logger.clone())?;
        if contenido_index.is_empty() {
            return Ok(false);
        }
        Ok(contenido_index.iter().any(|objeto| objeto.merge))
    }

    /// Busca en el merge head si hay un commit para
    /// definir si hay un merge en curso
    pub fn hay_merge_en_curso() -> Result<bool, String> {
        let path = Path::new(".gir/MERGE_HEAD");
        if !path.exists() {
            return Ok(false);
        }

        let merge = io::leer_a_string(".gir/MERGE_HEAD")?;

        Ok(!merge.is_empty())
    }

    pub fn obtener_commit_de_branch(branch: &str) -> Result<String, String> {
        let branch_split = branch.split('/').collect::<Vec<&str>>();
        if branch_split.len() == 1 {
            let ruta = format!(".gir/refs/heads/{}", branch);
            let padre_commit = io::leer_a_string(path::Path::new(&ruta))?;
            Ok(padre_commit)
        } else if branch_split.len() == 2 {
            let ruta = format!(".gir/refs/remotes/{}/{}", branch_split[0], branch_split[1]);
            let padre_commit = io::leer_a_string(path::Path::new(&ruta))?;
            Ok(padre_commit)
        } else {
            Err("Nombre de la rama ambigua".to_string())
        }
    }

    fn escribir_merge_head(&self) -> Result<(), String> {
        let ruta_merge = Path::new(".gir/MERGE_HEAD");
        let commit = Self::obtener_commit_de_branch(&self.branch_a_mergear)?;
        io::escribir_bytes(ruta_merge, commit)?;
        Ok(())
    }

    /// Escribe el mensaje del merge en el archivo COMMIT_EDITMSG.
    fn escribir_mensaje_merge(&self) -> Result<(), String> {
        let ruta_merge_msg = Path::new(".gir/COMMIT_EDITMSG");
        io::escribir_bytes(
            ruta_merge_msg,
            format!(
                "Mergear rama \"{}\" en  \"{}\"",
                self.branch_a_mergear, self.branch_actual
            ),
        )?;
        Ok(())
    }

    /// Limpia los archivos que quedan luego de un merge
    pub fn limpiar_merge_post_commit() -> Result<(), String> {
        let ruta_merge = Path::new(".gir/MERGE_HEAD");
        if ruta_merge.exists() {
            io::rm_directorio(ruta_merge)?;
        }
        Ok(())
    }
}

impl Ejecutar for Merge {
    fn ejecutar(&mut self) -> Result<String, String> {
        self.logger.log("Ejecutando comando merge");

        if Self::hay_merge_en_curso()? {
            return Err("Ya hay un merge en curso".to_string());
        }

        if self.abort {
            io::rm_directorio(".gir/MERGE_HEAD")?;
            index::limpiar_archivo_index()?;
        }

        let commit_actual = Self::obtener_commit_de_branch(&self.branch_actual)?;
        let commit_a_mergear = Self::obtener_commit_de_branch(&self.branch_a_mergear)?;
        let commit_base = Self::obtener_commit_base_entre_dos_branches(
            &self.branch_actual,
            &self.branch_a_mergear,
            self.logger.clone(),
        )?;

        if commit_actual == commit_a_mergear {
            return Ok("No hay nada para mergear".to_string());
        }

        let mensaje = if commit_base == commit_actual && !self.no_fast_forward {
            self.logger.log("Haciendo fast-forward");
            self.fast_forward()
        } else {
            self.logger.log("Realizando auto-merge");
            self.automerge(&commit_base)
        }?;

        self.escribir_merge_head()?;
        Ok(mensaje)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test01_mergear_archivos_sin_conflictos() {
        let base = "primera linea
        segunda linea
        tercera linea
        cuarta linea
        "
        .to_string();

        let version_1 = "primera linea
        segunda linea
        3ra linea
        cuarta linea"
            .to_string();

        let version_2 = "primera linea
        segunda linea
        tercera linea
        cuarta linea"
            .to_string();

        let diff_1 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_1).unwrap();
        let diff_2 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_2).unwrap();
        let (contenido_final, _conflictos) = Merge::mergear_diffs(diff_1, diff_2, &base);

        assert_eq!(
            contenido_final,
            "primera linea\nsegunda linea\n3ra linea\ncuarta linea\n"
        )
    }

    #[test]
    #[serial]
    fn test02_mergear_archivos_con_cambios_cerca() {
        let base = "primera linea
        segunda linea
        tercera linea
        cuarta linea"
            .to_string();

        let version_1 = "primera linea
        segunda_linea
        3ra linea
        cuarta linea"
            .to_string();

        let version_2 = "primera linea
        2da linea
        tercera linea
        cuarta linea"
            .to_string();

        let diff_1 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_1).unwrap();
        let diff_2 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_2).unwrap();
        let (contenido_final, _conflictos) = Merge::mergear_diffs(diff_1, diff_2, &base);
        assert_eq!(
            contenido_final,
            "primera linea\n<<<<<< HEAD\nsegunda_linea\n3ra linea\n======\n2da linea\ntercera linea\n>>>>>> Entrante\ncuarta linea\n"
        )
    }
    #[test]
    #[serial]
    fn test03_mergear_archivos_con_cambios_lejos() {
        let base = "primera linea
        segunda linea
        tercera linea
        cuarta linea"
            .to_string();

        let version_1 = "primera linea
        2da linea
        tercera linea
        cuarta linea"
            .to_string();

        let version_2 = "primera linea
        segunda linea
        tercera linea
        4ta linea"
            .to_string();

        let diff_1 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_1).unwrap();
        let diff_2 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_2).unwrap();
        let (contenido_final, _conflictos) = Merge::mergear_diffs(diff_1, diff_2, &base);

        assert_eq!(
            contenido_final,
            "primera linea\n2da linea\ntercera linea\n4ta linea\n",
        )
    }

    #[test]
    #[serial]
    fn test04_mergear_archivos_con_muchos_conflictos() {
        let base = "primera linea
        segunda linea
        tercera linea
        cuarta linea"
            .to_string();

        let version_1 = "primera linea
        3 linea
        cuarta linea"
            .to_string();

        let version_2 = "primera linea
        2da linea
        3ra linea
        cuarta linea"
            .to_string();

        let diff_1 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_1).unwrap();
        let diff_2 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_2).unwrap();
        let (contenido_final, _conflictos) = Merge::mergear_diffs(diff_1, diff_2, &base);

        assert_eq!(
            contenido_final,
            "primera linea\n<<<<<< HEAD\n3 linea\n======\n2da linea\n3ra linea\n>>>>>> Entrante\ncuarta linea\n"
        )
    }

    #[test]
    #[serial]
    fn test05_mergear_archivos_con_conflictos_y_lineas_repetidas() {
        let base = "primera linea
        segunda linea
        tercera linea
        cuarta linea
        quinta linea"
            .to_string();

        let version_1 = "primera linea
        3 linea
        cuarta linea
        quinta linea"
            .to_string();

        let version_2 = "primera linea
        2da linea
        3ra linea
        cuarta linea
        quinta linea"
            .to_string();

        let diff_1 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_1).unwrap();
        let diff_2 = Merge::obtener_diffs_entre_dos_archivos(&base, &version_2).unwrap();
        let (contenido_final, _conflictos) = Merge::mergear_diffs(diff_1, diff_2, &base);

        assert_eq!(
            contenido_final,
            "primera linea\n<<<<<< HEAD\n3 linea\n======\n2da linea\n3ra linea\n>>>>>> Entrante\ncuarta linea\nquinta linea\n"
        )
    }
}
