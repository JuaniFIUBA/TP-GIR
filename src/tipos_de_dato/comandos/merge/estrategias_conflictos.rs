use crate::tipos_de_dato::{
    conflicto::Conflicto, lado_conflicto::LadoConflicto, region::Region, tipo_diff::TipoDiff,
};

/// Esta funcion contempla el conflicto donde tenemos conflicto del tipo Add-Remove vs Add-Remove,
/// es decir, dos modificaciones en la misma linea
pub fn conflicto_len_4(conflicto: &Conflicto) -> Region {
    let mut lado_head = String::new();
    let mut lado_entrante = String::new();

    for (diff, lado) in conflicto {
        if let TipoDiff::Added(linea) = diff {
            match lado {
                LadoConflicto::Head => lado_head.push_str(&format!("{}\n", linea)),
                LadoConflicto::Entrante => lado_entrante.push_str(&format!("{}\n", linea)),
            };
        }
    }

    Region::Conflicto(lado_head, lado_entrante)
}

/// Esta funcion es auxiliar de conflicto_len_3, ya que se repite el mismo codigo para ambos lados
/// del conflicto. En el caso que la longitud es 0 se agrega la linea base ya que el lado opuesto
/// quiso eliminar dicha linea (es porque los conflictos de longitud 3 son del tipo Add-Remove vs Remove)
pub fn un_lado_conflicto_len_3(conflicto: Vec<&TipoDiff>, linea_base: &str) -> String {
    let mut lado = String::new();
    if conflicto.len() == 1 {
        match conflicto[0] {
            TipoDiff::Added(ref linea) => lado.push_str(&format!("{linea_base}\n{linea}\n")),
            TipoDiff::Unchanged(ref linea) => lado.push_str(&linea.to_string()),
            _ => {}
        };
    } else {
        for diff in conflicto {
            match diff {
                TipoDiff::Added(ref linea) => lado.push_str(&format!("{linea}\n")),
                TipoDiff::Unchanged(ref linea) => lado.push_str(&format!("{linea}\n")),
                _ => {}
            };
        }
    }

    lado
}

/// Esta funcion contempla el conflicto de Add-Remove vs Remove, es decir una linea modificada
/// vs una linea eliminada
pub fn conflicto_len_3(conflicto: &Conflicto, linea_base: &str) -> Region {
    let head: Vec<&TipoDiff> = conflicto
        .iter()
        .filter_map(|(diff, lado)| match lado {
            LadoConflicto::Head => Some(diff),
            _ => None,
        })
        .collect();

    let lado_head = un_lado_conflicto_len_3(head, linea_base);

    let entrante: Vec<&TipoDiff> = conflicto
        .iter()
        .filter_map(|(diff, lado)| match lado {
            LadoConflicto::Entrante => Some(diff),
            _ => None,
        })
        .collect();

    let lado_entrante = un_lado_conflicto_len_3(entrante, linea_base);

    Region::Conflicto(lado_head, lado_entrante)
}

/// Esta funcion contempla todos los casos de longitud 2, sean conflictos no.
pub fn resolver_merge_len_2(
    conflicto: &Conflicto,
    linea_base: &str,
    es_conflicto_obligatorio: bool,
) -> Region {
    match (&conflicto[0].0, &conflicto[1].0) {
        (TipoDiff::Added(linea_1), TipoDiff::Added(linea_2)) => {
            if linea_1 != linea_2 || es_conflicto_obligatorio {
                Region::Conflicto(linea_1.clone(), linea_2.clone())
            } else {
                Region::Normal(linea_1.clone())
            }
        }
        (TipoDiff::Added(linea_1), TipoDiff::Removed(_)) => {
            Region::Conflicto(format!("{linea_base}\n{linea_1}\n"), "".to_string())
        }
        (TipoDiff::Added(linea_1), TipoDiff::Unchanged(linea_2)) => {
            if es_conflicto_obligatorio {
                Region::Conflicto(linea_1.to_owned(), linea_2.to_owned())
            } else {
                Region::Normal(linea_1.clone())
            }
        }
        (TipoDiff::Removed(_), TipoDiff::Added(linea_2)) => {
            Region::Conflicto("".to_string(), format!("{linea_base}\n{linea_2}\n"))
        }
        (TipoDiff::Unchanged(linea_1), TipoDiff::Added(linea_2)) => {
            if es_conflicto_obligatorio {
                Region::Conflicto(linea_1.to_owned(), linea_2.to_owned())
            } else {
                Region::Normal(linea_2.clone())
            }
        }
        (TipoDiff::Unchanged(linea_1), TipoDiff::Unchanged(linea_2)) => {
            if es_conflicto_obligatorio {
                Region::Conflicto(linea_1.to_owned(), linea_2.to_owned())
            } else {
                Region::Normal(linea_1.clone())
            }
        }
        (_, _) => Region::Normal("".to_string()),
    }
}

/// Esta funcion contempla todos los casos de longitud 3, donde no hay conflictos
pub fn resolver_merge_len_3(
    conflicto: &Conflicto,
    linea_base: &str,
    es_conflicto_obligatorio: bool,
) -> Region {
    if es_conflicto_obligatorio {
        conflicto_len_3(conflicto, linea_base)
    } else {
        let mut lineas = String::new();
        for (diff, _) in conflicto {
            if let TipoDiff::Added(linea) = diff {
                lineas.push_str(linea)
            }
        }
        Region::Normal(lineas)
    }
}
