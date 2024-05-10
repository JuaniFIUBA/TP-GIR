use std::{path::PathBuf, sync::Arc};

use chrono::{FixedOffset, LocalResult, TimeZone};
use serde::{Deserialize, Serialize};

use crate::{
    tipos_de_dato::{
        comando::Ejecutar,
        comandos::{add::Add, cat_file},
        date::Date,
        logger::Logger,
        region::{unificar_regiones, Region},
        tipo_diff::TipoDiff,
    },
    utils::io::{escribir_bytes, leer_a_string},
};

use super::tree::Tree;

const AMARILLO: &str = "\x1B[33m";
const RESET: &str = "\x1B[0m";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitObj {
    /// Hash del objeto commit.
    pub hash: String,
    /// Hash del objeto tree que tiene el commit.
    pub hash_tree: String,
    /// Nombre del autor del commit.
    pub autor: String,
    /// Mail del autor del commit.
    pub mail: String,
    /// Fecha del commit, guardada en fomarto unix.
    pub date: Date,
    /// Mensaje del commit.
    pub mensaje: String,
    /// Hash de los commits padres del commit.
    pub padres: Vec<String>,
    #[serde(skip)]
    pub logger: Arc<Logger>,
}

impl CommitObj {
    /// Formatea el timestamp del commit.
    /// Toma el timestamp en formato unix y lo formatea en el formato que se muestra en el log.
    fn format_timestamp(
        timestamp: i64,
        offset_horas: i32,
        offset_minutos: i32,
    ) -> Result<String, String> {
        let offset_seconds = offset_horas * 3600 + offset_minutos * 60;
        let offset_str = format!(
            "{:>+03}{:02}",
            offset_seconds / 3600,
            (offset_seconds % 3600) / 60
        );

        let offset = match FixedOffset::east_opt(offset_seconds) {
            Some(offset) => offset,
            None => return Err("No se pudo obtener el offset".to_string()),
        };

        let datetime = match offset.timestamp_opt(timestamp, 0) {
            LocalResult::Single(datetime) => datetime,
            _ => return Err("No se pudo obtener el datetime".to_string()),
        };

        let datetime_formateado = datetime.format("%a %b %d %H:%M:%S %Y");
        let formatted_timestamp = format!("{} {}", datetime_formateado, offset_str);

        Ok(formatted_timestamp)
    }

    /// Recibe un Date y lo formatea en el formato que se muestra en el log.
    fn formatear_date(date: &Date) -> Result<String, String> {
        let timestamp = match date.tiempo.parse::<i64>() {
            Ok(timestamp) => timestamp,
            Err(_) => return Err("No se pudo obtener el timestamp".to_string()),
        };
        let (horas, minutos) = date.offset.split_at(3);
        let offset_horas = horas[0..3].parse::<i32>().unwrap_or(-3);
        let offset_minutos = minutos.parse::<i32>().unwrap_or(0);
        Self::format_timestamp(timestamp, offset_horas, offset_minutos)
    }

    /// Aplica el diff del commit al directorio actual.
    /// Devuelve un vector con los archivos que tuvieron conflictos.
    pub fn aplicar_a_directorio(&self) -> Result<Vec<PathBuf>, String> {
        let mut conflictos = Vec::new();

        let tree_actual =
            Tree::from_hash(&self.hash_tree, PathBuf::from("."), self.logger.clone())?;

        let hash_tree_padre =
            CommitObj::from_hash(self.padres[0].clone(), self.logger.clone())?.hash_tree;

        let tree_padre =
            Tree::from_hash(&hash_tree_padre, PathBuf::from("."), self.logger.clone())?;

        let deep_diffs = tree_padre.deep_changes(&tree_actual)?;

        for (archivo, diffs) in deep_diffs {
            let contenido_archivo = leer_a_string(&archivo)?;

            let archivo_por_regiones = aplicar_diff(&contenido_archivo, diffs);
            let hubo_conflictos = archivo_por_regiones.iter().any(|region| match region {
                Region::Normal(_) => false,
                Region::Conflicto(_, _) => true,
            });

            if hubo_conflictos {
                conflictos.push(PathBuf::from(&archivo));
            }

            let contenido_a_escribir = archivo_por_regiones
                .iter()
                .map(|region| format!("{}", region))
                .collect::<Vec<String>>()
                .join("\n");
            escribir_bytes(&archivo, &contenido_a_escribir)?;

            let mut add = Add::from(vec![archivo], self.logger.clone())?;
            add.ejecutar()?;
        }
        Ok(conflictos)
    }

    /// Crea un objeto commit a partir de un hash de commit escrito en base.
    /// Devuelve un error si el hash no es valido o si no se pudo obtener el contenido del commit.
    /// Devuelve error si no se logran llenar todos los campos del commit.
    pub fn from_hash(hash: String, logger: Arc<Logger>) -> Result<CommitObj, String> {
        if hash.len() != 40 {
            return Err("Hash invalido".to_string());
        }
        let (_header, contenido) = cat_file::obtener_contenido_objeto(&hash)?;
        let mut padres: Vec<String> = Vec::new();
        let mut autor_option: Option<String> = None;
        let mut mail_option: Option<String> = None;
        let mut date_option: Option<Date> = None;
        let mut hash_tree_option: Option<String> = None;

        for linea in contenido.split('\n') {
            let linea_splitteada = linea.split(' ').collect::<Vec<&str>>();
            match linea_splitteada[0] {
                "parent" => padres.push(linea_splitteada[1].to_string()),
                "author" => {
                    autor_option = Some(linea_splitteada[1].to_string());
                    mail_option = Some(linea_splitteada[2].to_string());
                    date_option = Some(Date {
                        tiempo: linea_splitteada[3].to_string(),
                        offset: linea_splitteada[4].to_string(),
                    });
                }
                "tree" => {
                    hash_tree_option = Some(linea_splitteada[1].to_string());
                }
                "commiter" => {}
                _ => break,
            }
        }

        let (autor, mail, date, hash_tree) =
            match (autor_option, mail_option, date_option, hash_tree_option) {
                (Some(autor), Some(mail), Some(date), Some(hash_tree)) => {
                    (autor, mail, date, hash_tree)
                }
                _ => return Err("No se pudo obtener el contenido del commit".to_string()),
            };

        let linea_splitteada_contenido = contenido.splitn(2, "\n\n").collect::<Vec<&str>>();
        let mensaje = linea_splitteada_contenido[1].to_string();

        let objeto = CommitObj {
            hash,
            hash_tree,
            autor,
            mail,
            date,
            mensaje,
            padres,
            logger,
        };

        Ok(objeto)
    }

    /// Formatea el contenido del commit para mostrarlo en el log.
    pub fn format_log(&self) -> Result<String, String> {
        let mut log = format!("{}commit {} {}\n", AMARILLO, self.hash, RESET);

        if self.padres.len() > 1 {
            log.push_str("Merge: ");
            for padre in &self.padres {
                log.push_str(&format!("{} ", &padre[..7]));
            }
            log.push('\n');
        }
        log.push_str(&format!("Autor: {} <{}>\n", self.autor, self.mail));
        log.push_str(&format!("Date: {}\n", Self::formatear_date(&self.date)?));
        log.push_str(&format!("\n     {}\n", self.mensaje));
        Ok(log)
    }
}

/// Aplica el diff del commit al texto recibido.
/// Devuelve un vector con las lineas del texto con el diff aplicado.
/// Si hay conflictos, de ser posible los unifica y devuelve una region de conflicto.
fn aplicar_diff(texto: &str, diffs: Vec<(usize, TipoDiff)>) -> Vec<Region> {
    let mut contenido_final = vec![];
    let lineas = texto.lines().collect::<Vec<_>>();

    let mut anterior_fue_conflicto = false;
    for (i, linea_actual) in lineas.iter().enumerate() {
        let diffs_linea: Vec<_> = diffs.iter().filter(|(linea, _)| *linea - 1 == i).collect();

        if anterior_fue_conflicto {
            let diffs_a_agregar: Vec<_> = diffs_linea
                .iter()
                .filter(|(_, diff)| !matches!(diff, TipoDiff::Removed(_)))
                .collect();

            let mut buffer = Vec::new();
            for diff in diffs_a_agregar {
                match &diff.1 {
                    TipoDiff::Added(linea) => buffer.push(linea.to_string()),
                    TipoDiff::Unchanged(linea) => buffer.push(linea.to_string()),
                    _ => {}
                }
            }
            contenido_final.push(Region::Conflicto(
                linea_actual.to_string(),
                buffer.join("\n"),
            ));
            anterior_fue_conflicto = false;
            continue;
        }
        if diffs_linea.len() == 1 {
            match &diffs_linea[0].1 {
                TipoDiff::Added(linea) => {
                    contenido_final.push(Region::Normal(linea_actual.to_string()));
                    contenido_final.push(Region::Normal(linea.to_string()));
                }
                TipoDiff::Removed(linea) => {
                    if linea != linea_actual {
                        anterior_fue_conflicto = true;
                        contenido_final
                            .push(Region::Conflicto(linea_actual.to_string(), String::new()))
                    }
                }
                TipoDiff::Unchanged(linea) => {
                    contenido_final.push(Region::Normal(linea.to_string()))
                }
            }
        } else if diffs_linea.len() == 2 {
            if let TipoDiff::Removed(linea) = &diffs_linea[0].1 {
                if let TipoDiff::Added(linea2) = &diffs_linea[1].1 {
                    if linea != linea_actual {
                        anterior_fue_conflicto = true;
                        contenido_final.push(Region::Conflicto(
                            linea_actual.to_string(),
                            linea2.to_string(),
                        ));
                        continue;
                    }
                    contenido_final.push(Region::Normal(linea2.to_string()))
                }
            }
        }
    }
    unificar_regiones(contenido_final)
}

#[cfg(test)]

mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test01_rearmar_timestamp_log() {
        let timestamp = 1234567890;
        let offset_horas = -03;
        let offset_minutos = 00;
        let timestamp_formateado =
            CommitObj::format_timestamp(timestamp, offset_horas, offset_minutos).unwrap();
        let timestamp_formateado_esperado = "Fri Feb 13 20:31:30 2009 -0300";
        assert_eq!(timestamp_formateado, timestamp_formateado_esperado);
    }

    #[test]
    #[serial]
    fn test02_formatear_log() {
        let hash_commit = "1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t".to_string();
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/commit_obj_test02")).unwrap());
        let objeto = CommitObj {
            hash: hash_commit.clone(),
            autor: "nombre_apellido".to_string(),
            mail: "mail".to_string(),
            date: Date {
                tiempo: "1234567890".to_string(),
                offset: "-0300".to_string(),
            },
            mensaje: "Mensaje del commit".to_string(),
            padres: vec![],
            hash_tree: "1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t".to_string(),
            logger,
        };

        let contenido_log = objeto.format_log().unwrap();
        let contenido_log_esperado = format!("{AMARILLO}commit 1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t {RESET}\nAutor: nombre_apellido <mail>\nDate: Fri Feb 13 20:31:30 2009 -0300\n\n     Mensaje del commit\n");
        assert_eq!(contenido_log, contenido_log_esperado);
    }

    #[test]
    #[serial]
    fn test03_aplicar_diff_agregando_linea() {
        let texto = "primera linea\nsegunda linea\ntercera linea";
        let diff = vec![
            (1, TipoDiff::Added("entre 1 y 2".to_string())),
            (2, TipoDiff::Unchanged("segunda linea".to_string())),
            (3, TipoDiff::Unchanged("tercera linea".to_string())),
        ];

        let resultado = aplicar_diff(texto, diff);

        let resultado_esperado = vec![
            Region::Normal("primera linea".to_string()),
            Region::Normal("entre 1 y 2".to_string()),
            Region::Normal("segunda linea".to_string()),
            Region::Normal("tercera linea".to_string()),
        ];

        assert_eq!(resultado, resultado_esperado);
    }

    #[test]
    #[serial]
    fn test04_aplicar_diff_eliminando_linea() {
        let texto = "primera linea\nsegunda linea\ntercera linea";
        let diff = vec![
            (1, TipoDiff::Unchanged("primera linea".to_string())),
            (2, TipoDiff::Removed("segunda linea".to_string())),
            (3, TipoDiff::Unchanged("tercera linea".to_string())),
        ];

        let resultado = aplicar_diff(texto, diff);

        let resultado_esperado = vec![
            Region::Normal("primera linea".to_string()),
            Region::Normal("tercera linea".to_string()),
        ];

        assert_eq!(resultado, resultado_esperado);
    }

    #[test]
    #[serial]
    fn test05_aplicar_diff_modificando_linea() {
        let texto = "primera linea\nsegunda linea\ntercera linea";
        let diff = vec![
            (1, TipoDiff::Unchanged("primera linea".to_string())),
            (2, TipoDiff::Removed("segunda linea".to_string())),
            (2, TipoDiff::Added("segunda linea modificada".to_string())),
            (3, TipoDiff::Unchanged("tercera linea".to_string())),
        ];

        let resultado = aplicar_diff(texto, diff);

        let resultado_esperado = vec![
            Region::Normal("primera linea".to_string()),
            Region::Normal("segunda linea modificada".to_string()),
            Region::Normal("tercera linea".to_string()),
        ];

        assert_eq!(resultado, resultado_esperado);
    }

    #[test]
    #[serial]
    fn test06_aplicar_diff_conflicto() {
        let texto = "primera linea\nsegunda linea\ntercera linea";
        let diff = vec![
            (1, TipoDiff::Unchanged("primera linea".to_string())),
            (
                2,
                TipoDiff::Removed("segunda linea diferente a la original".to_string()),
            ),
            (2, TipoDiff::Added("segunda linea modificada".to_string())),
            (3, TipoDiff::Unchanged("tercera linea".to_string())),
        ];

        let resultado = aplicar_diff(texto, diff);

        let resultado_esperado = vec![
            Region::Normal("primera linea".to_string()),
            Region::Conflicto(
                "segunda linea".to_string(),
                "segunda linea modificada".to_string(),
            ),
            Region::Normal("tercera linea".to_string()),
        ];

        assert_eq!(resultado, resultado_esperado);
    }
}
